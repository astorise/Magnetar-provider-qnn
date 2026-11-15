//! Runtime-owned Device metadata for discovered QNN backends.
//!
//! Only portable, stable metadata is exposed -- never a QNN handle, context
//! or library path. Nothing is advertised that discovery has not proven:
//! `dtype_support`, `layout_support`, `memory_class_support` and
//! `execution_capabilities` stay empty and `memory_capacity` stays `0`
//! (unknown) until backend queries and validated graphs back real values.

use magnetar_core::{
    ProviderPressureLevel,
    device::{DeviceDescriptor, DeviceId, DeviceMetadata},
};

use crate::{engine::QnnEngine, sdk::BackendProbe};

/// Stable Device identifier, e.g. `qnn:htp:0`. One Device per engine:
/// the QNN interface query does not enumerate physical devices, so the
/// ordinal is always `0` until a device query is verified against an SDK.
pub fn qnn_device_id(engine: QnnEngine, ordinal: u32) -> DeviceId {
    DeviceId::new(format!("qnn:{}:{ordinal}", engine.as_str()))
}

/// Builds the Device a successfully probed backend is advertised as.
pub fn qnn_device_descriptor(probe: &BackendProbe, provider_name: &str) -> DeviceDescriptor {
    let engine = probe.engine;
    let mut metadata = DeviceMetadata::new(
        qnn_device_id(engine, 0),
        format!("{} (QNN {})", engine.display_name(), probe.provider_name),
        engine.device_type(),
        provider_name,
    );
    metadata.vendor = "Qualcomm".into();
    metadata.architecture = format!(
        "qnn-{}/core-api-{}/backend-api-{}",
        engine.as_str(),
        probe.core_api,
        probe.backend_api
    );
    metadata.pressure = ProviderPressureLevel::Unknown;
    DeviceDescriptor::new(metadata)
}
