//! Qualcomm (QAIRT/QNN) execution Provider for the Magnetar local AI
//! Runtime: Hexagon HTP/NPU, Adreno GPU and Qualcomm CPU backends, each
//! exposed as its own Magnetar `Device` only when the backend library is
//! actually found, loaded and answers the QNN interface query.
//!
//! **Status: Phase 1 (lifecycle and discovery).** No compute Kernel and no
//! `ProviderExecutionApi` yet -- see the README and `docs/support-matrix.md`.

mod config;
mod device;
mod engine;
mod ffi;
mod provider;
mod sdk;

#[cfg(any(test, feature = "mock"))]
pub mod mock;

pub use config::{LIBRARY_DIR_ENV, LoadPolicy, PolicyViolation};
pub use device::{qualcomm_device_descriptor, qualcomm_device_id};
pub use engine::QnnEngine;
pub use provider::{
    BackendDiagnostic, BackendStatus, QUALCOMM_PROVIDER_NAME, QualcommProvider,
    qualcomm_provider_metadata,
};
pub use sdk::{
    BackendProbe, DynamicQnnSdk, ProbeFailure, QnnSdk, QnnVersion, SUPPORTED_CORE_API_MAJOR,
};
