//! QNN SDK access: a small trait ([`QnnSdk`]) so discovery logic is testable
//! without any Qualcomm library, and the real dynamic loader behind it.

use std::{fmt, path::PathBuf};

use crate::{
    config::{LoadPolicy, PolicyViolation},
    engine::QnnEngine,
    ffi,
};

/// The QNN core API major version this Provider is written against. A
/// backend reporting a different major version is rejected as incompatible
/// rather than driven through an unverified interface.
pub const SUPPORTED_CORE_API_MAJOR: u32 = 2;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct QnnVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl fmt::Display for QnnVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl From<ffi::RawVersion> for QnnVersion {
    fn from(raw: ffi::RawVersion) -> Self {
        Self {
            major: raw.major,
            minor: raw.minor,
            patch: raw.patch,
        }
    }
}

/// What a backend library reported about itself. Contains no native
/// handle, pointer or path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendProbe {
    pub engine: QnnEngine,
    pub provider_name: String,
    pub backend_id: u32,
    pub core_api: QnnVersion,
    pub backend_api: QnnVersion,
}

/// Why a backend is not available. Variants carry no path and no native
/// error text, so they are safe to log and report as-is.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProbeFailure {
    /// No trusted library directory is configured.
    NoTrustedDirectory,
    /// Every configured directory was rejected by the loading policy.
    DirectoryRejected(PolicyViolation),
    /// No configured directory contains this backend's library.
    LibraryNotFound,
    /// The library exists but the system loader refused it.
    LibraryLoadFailed,
    /// The library does not export `QnnInterface_getProviders`.
    SymbolMissing,
    /// `QnnInterface_getProviders` returned this non-zero error handle.
    InterfaceQueryFailed(u64),
    /// The library reported zero interface providers.
    NoInterfaceProviders,
    /// The library returned null or implausible interface data.
    MalformedInterface,
    /// The reported core API major version is not [`SUPPORTED_CORE_API_MAJOR`].
    IncompatibleCoreApi { found: QnnVersion },
    /// Probing panicked inside this Provider.
    Panicked,
}

impl ProbeFailure {
    /// Stable machine-readable code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::NoTrustedDirectory => "no-trusted-directory",
            Self::DirectoryRejected(_) => "directory-rejected",
            Self::LibraryNotFound => "library-not-found",
            Self::LibraryLoadFailed => "library-load-failed",
            Self::SymbolMissing => "symbol-missing",
            Self::InterfaceQueryFailed(_) => "interface-query-failed",
            Self::NoInterfaceProviders => "no-interface-providers",
            Self::MalformedInterface => "malformed-interface",
            Self::IncompatibleCoreApi { .. } => "incompatible-core-api",
            Self::Panicked => "probe-panicked",
        }
    }
}

impl fmt::Display for ProbeFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoTrustedDirectory => write!(
                f,
                "no trusted library directory configured (set {})",
                crate::config::LIBRARY_DIR_ENV
            ),
            Self::DirectoryRejected(violation) => write!(f, "{violation}"),
            Self::LibraryNotFound => f.write_str("backend library not found"),
            Self::LibraryLoadFailed => f.write_str("backend library could not be loaded"),
            Self::SymbolMissing => f.write_str("QnnInterface_getProviders is not exported"),
            Self::InterfaceQueryFailed(handle) => {
                write!(
                    f,
                    "QnnInterface_getProviders failed (error handle {handle})"
                )
            }
            Self::NoInterfaceProviders => f.write_str("backend reported no interface providers"),
            Self::MalformedInterface => f.write_str("backend returned malformed interface data"),
            Self::IncompatibleCoreApi { found } => write!(
                f,
                "core API {found} is incompatible (supported major: {SUPPORTED_CORE_API_MAJOR})"
            ),
            Self::Panicked => f.write_str("backend probing panicked"),
        }
    }
}

/// Source of truth about which QNN backends exist.
pub trait QnnSdk: Send + Sync {
    fn probe(&self, engine: QnnEngine) -> Result<BackendProbe, ProbeFailure>;
}

/// Loads QNN backend libraries from the directories a [`LoadPolicy`]
/// trusts.
#[derive(Clone, Debug, Default)]
pub struct DynamicQnnSdk {
    policy: LoadPolicy,
}

impl DynamicQnnSdk {
    pub fn new(policy: LoadPolicy) -> Self {
        Self { policy }
    }

    fn locate(&self, engine: QnnEngine) -> Result<PathBuf, ProbeFailure> {
        if self.policy.library_dirs().is_empty() {
            return Err(ProbeFailure::NoTrustedDirectory);
        }
        let mut first_violation = None;
        for dir in self.policy.library_dirs() {
            if let Err(violation) = LoadPolicy::check_dir(dir) {
                first_violation.get_or_insert(violation);
                continue;
            }
            let candidate = dir.join(engine.library_file_name());
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
        Err(match first_violation {
            Some(violation) => ProbeFailure::DirectoryRejected(violation),
            None => ProbeFailure::LibraryNotFound,
        })
    }

    fn probe_path(engine: QnnEngine, path: &std::path::Path) -> Result<BackendProbe, ProbeFailure> {
        // SAFETY: loading runs the library's initializers. Backend libraries
        // are trusted native code located only inside policy-approved
        // directories (`docs/security.md`).
        let library = unsafe { libloading::Library::new(path) }
            .map_err(|_| ProbeFailure::LibraryLoadFailed)?;
        // SAFETY: the symbol name and signature are the QNN C API's
        // (`ffi::GetProvidersFn`); the library outlives every use below.
        let get_providers: libloading::Symbol<ffi::GetProvidersFn> =
            unsafe { library.get(ffi::GET_PROVIDERS_SYMBOL) }
                .map_err(|_| ProbeFailure::SymbolMissing)?;
        // SAFETY: `get_providers` comes from the still-loaded `library`.
        let result = unsafe { ffi::query_first_provider(*get_providers) };
        // The library is never unloaded (`NeverUnload`, provider-loading-abi):
        // backend code may keep threads or callbacks alive after a query.
        std::mem::forget(library);
        let raw = result.map_err(|error| match error {
            ffi::QueryError::Failed(handle) => ProbeFailure::InterfaceQueryFailed(handle),
            ffi::QueryError::NoProviders => ProbeFailure::NoInterfaceProviders,
            ffi::QueryError::Malformed => ProbeFailure::MalformedInterface,
        })?;
        let core_api = QnnVersion::from(raw.api_version.core_api_version);
        if core_api.major != SUPPORTED_CORE_API_MAJOR {
            return Err(ProbeFailure::IncompatibleCoreApi { found: core_api });
        }
        Ok(BackendProbe {
            engine,
            provider_name: raw.provider_name,
            backend_id: raw.backend_id,
            core_api,
            backend_api: QnnVersion::from(raw.api_version.backend_api_version),
        })
    }
}

impl QnnSdk for DynamicQnnSdk {
    fn probe(&self, engine: QnnEngine) -> Result<BackendProbe, ProbeFailure> {
        let path = self.locate(engine)?;
        std::panic::catch_unwind(|| Self::probe_path(engine, &path))
            .unwrap_or(Err(ProbeFailure::Panicked))
    }
}
