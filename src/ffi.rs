//! The only `unsafe` code in this crate: the minimal QNN C surface needed
//! for backend discovery.
//!
//! # Verification status
//!
//! Only `QnnInterface_getProviders` and the leading, version-stable fields
//! of `QnnInterface_t` (`backendId`, `providerName`, `apiVersion`) are
//! declared; the implementation function table that follows them in the C
//! union is never touched. The declarations follow the public QNN C API
//! (`QnnInterface.h`, `QnnTypes.h`) **but were written without a QAIRT SDK
//! checkout to compare against** (none exists in this repository's tooling),
//! so they are checked against a real SDK header in
//! `docs/sdk-setup.md`'s verification step before any hardware claim.
//! `tests/fake_sdk.rs` exercises this exact code path against a locally
//! compiled library that mimics these declarations; that proves the loader
//! logic, not agreement with Qualcomm's binaries.

use std::ffi::{CStr, c_char};

/// `Qnn_Version_t`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RawVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

/// `Qnn_ApiVersion_t`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RawApiVersion {
    pub core_api_version: RawVersion,
    pub backend_api_version: RawVersion,
}

/// Prefix of `QnnInterface_t`, up to (excluding) the implementation union.
#[repr(C)]
pub struct RawInterfaceHeader {
    pub backend_id: u32,
    pub provider_name: *const c_char,
    pub api_version: RawApiVersion,
}

/// `Qnn_ErrorHandle_t QnnInterface_getProviders(const QnnInterface_t*** providerList,
/// uint32_t* numProviders)`; `0` is `QNN_SUCCESS`.
pub type GetProvidersFn =
    unsafe extern "C" fn(*mut *const *const RawInterfaceHeader, *mut u32) -> u64;

pub const GET_PROVIDERS_SYMBOL: &[u8] = b"QnnInterface_getProviders\0";

/// Upper bound on providers read from one library; a larger count is
/// treated as a malformed answer.
const MAX_PROVIDERS: u32 = 64;
const MAX_NAME_LEN: usize = 64;

/// A plain-data copy of what a backend library reported.
pub struct RawProvider {
    pub backend_id: u32,
    pub provider_name: String,
    pub api_version: RawApiVersion,
}

pub enum QueryError {
    /// The library returned a non-zero `Qnn_ErrorHandle_t`.
    Failed(u64),
    NoProviders,
    Malformed,
}

/// Calls `QnnInterface_getProviders` and copies the first provider's header
/// into owned memory.
///
/// # Safety
///
/// `get_providers` must be the `QnnInterface_getProviders` entry point of a
/// loaded QNN backend library that stays loaded for the duration of this
/// call. The library is trusted native code (see `docs/security.md`).
pub unsafe fn query_first_provider(
    get_providers: GetProvidersFn,
) -> Result<RawProvider, QueryError> {
    let mut list: *const *const RawInterfaceHeader = std::ptr::null();
    let mut count: u32 = 0;
    // SAFETY: both out-pointers are valid for writes; the callee contract is
    // the one documented on this function.
    let status = unsafe { get_providers(&mut list, &mut count) };
    if status != 0 {
        return Err(QueryError::Failed(status));
    }
    if count == 0 {
        return Err(QueryError::NoProviders);
    }
    if list.is_null() || count > MAX_PROVIDERS {
        return Err(QueryError::Malformed);
    }
    // SAFETY: `list` is non-null and the backend reports `count` entries
    // (bounded above); the array is static backend-owned memory.
    let first = unsafe { *list };
    if first.is_null() {
        return Err(QueryError::Malformed);
    }
    // SAFETY: `first` is a non-null `QnnInterface_t*`; only its leading,
    // version-stable fields are read, and they are copied out immediately.
    let header = unsafe { &*first };
    let provider_name = if header.provider_name.is_null() {
        String::new()
    } else {
        // SAFETY: a non-null `providerName` is a NUL-terminated C string.
        sanitize(unsafe { CStr::from_ptr(header.provider_name) }.to_bytes())
    };
    Ok(RawProvider {
        backend_id: header.backend_id,
        provider_name,
        api_version: header.api_version,
    })
}

/// Keeps printable ASCII only and bounds the length, so a backend string can
/// never inject control characters or unbounded text into logs and Device
/// metadata.
fn sanitize(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take(MAX_NAME_LEN)
        .map(|&b| {
            if (0x20..0x7f).contains(&b) {
                b as char
            } else {
                '?'
            }
        })
        .collect()
}
