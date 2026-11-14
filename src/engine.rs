//! The QNN execution engines this Provider can expose as Devices.

use magnetar_core::device::DeviceType;

/// One QNN backend. Identity comes from which backend library answered,
/// never from an SoC or marketing name.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum QnnEngine {
    /// Hexagon Tensor Processor / NPU (`libQnnHtp`).
    Htp,
    /// Adreno GPU (`libQnnGpu`).
    Gpu,
    /// Qualcomm CPU reference backend (`libQnnCpu`).
    Cpu,
}

impl QnnEngine {
    pub const ALL: [QnnEngine; 3] = [QnnEngine::Htp, QnnEngine::Gpu, QnnEngine::Cpu];

    /// Short, stable lowercase name used in Device identifiers.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Htp => "htp",
            Self::Gpu => "gpu",
            Self::Cpu => "cpu",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Htp => "Hexagon HTP",
            Self::Gpu => "Adreno GPU",
            Self::Cpu => "Qualcomm CPU",
        }
    }

    pub const fn device_type(self) -> DeviceType {
        match self {
            Self::Htp => DeviceType::Npu,
            Self::Gpu => DeviceType::Gpu,
            Self::Cpu => DeviceType::Cpu,
        }
    }

    /// File name of the backend library, per platform. A bare file name:
    /// it is always joined to an explicitly configured directory and never
    /// handed to the system library search path.
    pub const fn library_file_name(self) -> &'static str {
        #[cfg(target_os = "windows")]
        {
            match self {
                Self::Htp => "QnnHtp.dll",
                Self::Gpu => "QnnGpu.dll",
                Self::Cpu => "QnnCpu.dll",
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            match self {
                Self::Htp => "libQnnHtp.so",
                Self::Gpu => "libQnnGpu.so",
                Self::Cpu => "libQnnCpu.so",
            }
        }
    }
}
