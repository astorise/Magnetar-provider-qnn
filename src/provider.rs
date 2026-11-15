//! `QnnProvider`: one logical Provider, one Magnetar Device per QNN
//! backend (HTP, GPU, CPU) that is actually present.
//!
//! # Status: discovery only
//!
//! Backends are discovered by loading their library and querying the QNN
//! interface. There is no graph compilation, tensor movement or Kernel
//! execution yet, so the Provider advertises **no Kernels** and returns no
//! `ProviderExecutionApi`. Consequently:
//!
//! - no backend found -> [`ProviderHealth::Unavailable`], zero Devices;
//! - backend(s) found -> their Devices, with [`ProviderHealth::Degraded`]:
//!   hardware software is present but this Provider cannot execute on it.
//!
//! `new()` never fails for a missing SDK, driver or hardware.

use std::sync::Arc;

use magnetar_core::{
    ProviderBinding, ProviderHealth,
    device::{Device, DeviceDescriptor},
    provider_status::{HealthDiagnostic, HealthScope, HealthState, ProviderHealthReport},
};
use magnetar_provider_api::{Provider, ProviderError, ProviderMetadata, ProviderRegistry};

use crate::{
    config::LoadPolicy,
    device::qnn_device_descriptor,
    engine::QnnEngine,
    sdk::{BackendProbe, DynamicQnnSdk, ProbeFailure, QnnSdk},
};

pub const QNN_PROVIDER_NAME: &str = "qnn";
const QNN_PROVIDER_VERSION: &str = env!("CARGO_PKG_VERSION");
const QNN_PROVIDER_VENDOR: &str = "Qualcomm (QAIRT/QNN)";

pub fn qnn_provider_metadata() -> ProviderMetadata {
    ProviderMetadata::new(
        QNN_PROVIDER_NAME,
        QNN_PROVIDER_VERSION,
        QNN_PROVIDER_VENDOR,
        "Qualcomm QNN backend discovery (Hexagon HTP, Adreno GPU, CPU) -- no compute \
         Kernels implemented yet (see this crate's own README)",
    )
}

/// Outcome of probing one engine.
#[derive(Clone, Debug)]
pub enum BackendStatus {
    Discovered {
        probe: BackendProbe,
        device: Arc<DeviceDescriptor>,
    },
    Unavailable {
        engine: QnnEngine,
        failure: ProbeFailure,
    },
}

impl BackendStatus {
    pub fn engine(&self) -> QnnEngine {
        match self {
            Self::Discovered { probe, .. } => probe.engine,
            Self::Unavailable { engine, .. } => *engine,
        }
    }
}

/// Redacted, loggable view of one backend's status.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendDiagnostic {
    pub engine: QnnEngine,
    /// `"discovered"` or a [`ProbeFailure::code`].
    pub code: &'static str,
    pub message: String,
}

pub struct QnnProvider {
    metadata: ProviderMetadata,
    backends: Vec<BackendStatus>,
}

impl QnnProvider {
    /// Probes backends from the directories in `MAGNETAR_QNN_LIB_DIR`.
    pub fn new() -> Self {
        Self::with_policy(LoadPolicy::from_env())
    }

    pub fn with_policy(policy: LoadPolicy) -> Self {
        Self::with_sdk(&DynamicQnnSdk::new(policy))
    }

    /// Probes every engine through `sdk` (a mock in tests).
    pub fn with_sdk(sdk: &dyn QnnSdk) -> Self {
        let backends = QnnEngine::ALL
            .into_iter()
            .map(|engine| match sdk.probe(engine) {
                Ok(probe) => {
                    let device = Arc::new(qnn_device_descriptor(&probe, QNN_PROVIDER_NAME));
                    BackendStatus::Discovered { probe, device }
                }
                Err(failure) => BackendStatus::Unavailable { engine, failure },
            })
            .collect();
        Self {
            metadata: qnn_provider_metadata(),
            backends,
        }
    }

    /// Whether at least one backend was discovered.
    pub fn is_available(&self) -> bool {
        self.backends
            .iter()
            .any(|b| matches!(b, BackendStatus::Discovered { .. }))
    }

    pub fn backends(&self) -> &[BackendStatus] {
        &self.backends
    }

    pub fn diagnostics(&self) -> Vec<BackendDiagnostic> {
        self.backends
            .iter()
            .map(|status| match status {
                BackendStatus::Discovered { probe, .. } => BackendDiagnostic {
                    engine: probe.engine,
                    code: "discovered",
                    message: format!(
                        "backend discovered (core API {}); execution not implemented",
                        probe.core_api
                    ),
                },
                BackendStatus::Unavailable { engine, failure } => BackendDiagnostic {
                    engine: *engine,
                    code: failure.code(),
                    message: failure.to_string(),
                },
            })
            .collect()
    }
}

impl Default for QnnProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Provider for QnnProvider {
    fn metadata(&self) -> ProviderMetadata {
        self.metadata.clone()
    }

    fn register(&self, _registry: &mut ProviderRegistry) -> Result<(), ProviderError> {
        // Devices are registered by the Runtime through `devices()`;
        // registering them here too would double-register them.
        Ok(())
    }

    fn health(&self) -> ProviderHealth {
        if self.is_available() {
            ProviderHealth::Degraded
        } else {
            ProviderHealth::Unavailable
        }
    }

    fn health_report(&self) -> ProviderHealthReport {
        let mut report =
            ProviderHealthReport::new(ProviderBinding::new(QNN_PROVIDER_NAME), self.health());
        report.diagnostics = self
            .diagnostics()
            .into_iter()
            .map(|d| {
                let state = if d.code == "discovered" {
                    HealthState::Degraded
                } else {
                    HealthState::Unavailable
                };
                HealthDiagnostic::new(HealthScope::Device, state)
                    .with_code(format!("qnn.{}.{}", d.engine.as_str(), d.code))
                    .with_message(d.message)
            })
            .collect();
        report
    }

    fn devices(&self) -> Vec<Arc<dyn Device>> {
        self.backends
            .iter()
            .filter_map(|status| match status {
                BackendStatus::Discovered { device, .. } => Some(device.clone() as Arc<dyn Device>),
                BackendStatus::Unavailable { .. } => None,
            })
            .collect()
    }

    // `kernel_advertisements` and `execution_api` keep their `None`/empty
    // defaults on purpose: nothing executable exists to advertise.
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockSdk;
    use magnetar_core::device::{DeviceId, DeviceType};

    #[test]
    fn absent_sdk_is_unavailable_with_no_devices() {
        let provider = QnnProvider::with_sdk(&MockSdk::absent());
        assert!(!provider.is_available());
        assert_eq!(provider.health(), ProviderHealth::Unavailable);
        assert!(provider.devices().is_empty());
        let diagnostics = provider.diagnostics();
        assert_eq!(diagnostics.len(), QnnEngine::ALL.len());
        assert!(diagnostics.iter().all(|d| d.code == "library-not-found"));
    }

    #[test]
    fn each_discovered_backend_is_its_own_device() {
        let sdk = MockSdk::absent()
            .with_backend(QnnEngine::Htp)
            .with_backend(QnnEngine::Gpu);
        let provider = QnnProvider::with_sdk(&sdk);
        assert_eq!(provider.health(), ProviderHealth::Degraded);
        let devices = provider.devices();
        let mut ids: Vec<_> = devices.iter().map(|d| d.id().to_string()).collect();
        ids.sort();
        assert_eq!(ids, ["qnn:gpu:0", "qnn:htp:0"]);
        let htp = devices
            .iter()
            .find(|d| d.id() == &DeviceId::new("qnn:htp:0"))
            .expect("htp device");
        assert_eq!(htp.device_type(), DeviceType::Npu);
        let gpu = devices
            .iter()
            .find(|d| d.id() == &DeviceId::new("qnn:gpu:0"))
            .expect("gpu device");
        assert_eq!(gpu.device_type(), DeviceType::Gpu);
        assert_ne!(htp.metadata().architecture, gpu.metadata().architecture);
    }

    #[test]
    fn devices_do_not_claim_unproven_capabilities() {
        let sdk = MockSdk::absent().with_backend(QnnEngine::Htp);
        let provider = QnnProvider::with_sdk(&sdk);
        let devices = provider.devices();
        let metadata = devices[0].metadata();
        assert!(metadata.dtype_support.is_empty());
        assert!(metadata.layout_support.is_empty());
        assert!(metadata.memory_class_support.is_empty());
        assert!(metadata.execution_capabilities.is_empty());
        assert_eq!(metadata.memory_capacity, 0);
        assert!(provider.kernel_advertisements().is_empty());
        assert!(provider.execution_api().is_none());
    }

    #[test]
    fn devices_register_in_the_runtime_registry_once() {
        let sdk = MockSdk::absent()
            .with_backend(QnnEngine::Htp)
            .with_backend(QnnEngine::Cpu);
        let provider = QnnProvider::with_sdk(&sdk);
        let mut registry = ProviderRegistry::default();
        registry
            .register_devices(QNN_PROVIDER_NAME, provider.devices())
            .expect("registration");
        assert_eq!(registry.devices().count(), 2);
        // Registering the same Devices again is a duplicate, not a no-op.
        assert!(
            registry
                .register_devices(QNN_PROVIDER_NAME, provider.devices())
                .is_err()
        );
    }

    #[test]
    fn failures_are_reported_per_backend_and_redacted() {
        let sdk = MockSdk::absent().with_backend(QnnEngine::Gpu).with_failure(
            QnnEngine::Htp,
            ProbeFailure::IncompatibleCoreApi {
                found: crate::sdk::QnnVersion {
                    major: 9,
                    minor: 1,
                    patch: 0,
                },
            },
        );
        let provider = QnnProvider::with_sdk(&sdk);
        let diagnostics = provider.diagnostics();
        let htp = diagnostics
            .iter()
            .find(|d| d.engine == QnnEngine::Htp)
            .expect("htp");
        assert_eq!(htp.code, "incompatible-core-api");
        assert!(diagnostics.iter().all(|d| !d.message.contains('/')));
        assert_eq!(provider.devices().len(), 1);

        let report = provider.health_report();
        assert_eq!(report.diagnostics.len(), QnnEngine::ALL.len());
        assert!(report.diagnostics.iter().any(|d| {
            d.code.as_deref() == Some("qnn.htp.incompatible-core-api")
                && d.state == HealthState::Unavailable
        }));
    }

    #[test]
    fn unconfigured_dynamic_sdk_is_unavailable() {
        let provider = QnnProvider::with_policy(LoadPolicy::default());
        assert_eq!(provider.health(), ProviderHealth::Unavailable);
        assert!(
            provider
                .diagnostics()
                .iter()
                .all(|d| d.code == "no-trusted-directory")
        );
    }

    #[test]
    fn metadata_is_well_formed() {
        let metadata = QnnProvider::with_sdk(&MockSdk::absent()).metadata();
        assert_eq!(metadata.name, QNN_PROVIDER_NAME);
        assert!(!metadata.description.is_empty());
        assert!(metadata.compute_operation_schema_support.is_empty());
    }
}
