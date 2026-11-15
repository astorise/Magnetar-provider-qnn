//! Exercises the real `dlopen` + `QnnInterface_getProviders` path against a
//! tiny library compiled on the fly with the system C compiler. It proves
//! this crate's loader logic and failure mapping; it is NOT a Qualcomm SDK
//! and says nothing about HTP/GPU hardware. Skips (loudly) without `cc`.
#![cfg(target_os = "linux")]

use std::{path::Path, process::Command};

use magnetar_provider_qnn::{
    DynamicQnnSdk, LoadPolicy, ProbeFailure, QnnEngine, QnnProvider, QnnSdk,
};

const PREAMBLE: &str = r#"
#include <stdint.h>
typedef struct { uint32_t major, minor, patch; } Ver;
typedef struct { Ver core; Ver backend; } ApiVer;
typedef struct { uint32_t backendId; const char* providerName; ApiVer apiVersion; uint64_t impl[64]; } Iface;
"#;

fn good(core_major: u32) -> String {
    format!(
        r#"{PREAMBLE}
static Iface iface = {{ 6, "fake_htp", {{ {{ {core_major}, 4, 0 }}, {{ 5, 1, 2 }} }}, {{0}} }};
static const Iface* list[1] = {{ &iface }};
uint64_t QnnInterface_getProviders(const Iface*** out, uint32_t* n) {{ *out = list; *n = 1; return 0; }}
"#
    )
}

const FAILING: &str = r#"
#include <stdint.h>
uint64_t QnnInterface_getProviders(const void*** out, uint32_t* n) { *out = 0; *n = 0; return 1003; }
"#;
const EMPTY: &str = r#"
#include <stdint.h>
uint64_t QnnInterface_getProviders(const void*** out, uint32_t* n) { *out = 0; *n = 0; return 0; }
"#;
const NO_SYMBOL: &str = "int unrelated(void) { return 0; }\n";
const NULL_ENTRY: &str = r#"
#include <stdint.h>
static const void* list[1] = { 0 };
uint64_t QnnInterface_getProviders(const void*** out, uint32_t* n) { *out = list; *n = 1; return 0; }
"#;

/// Compiles `source` as `libQnnHtp.so` in a fresh directory.
fn build(source: &str) -> Option<tempfile::TempDir> {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("fake.c");
    std::fs::write(&src, source).unwrap();
    let out = dir.path().join(QnnEngine::Htp.library_file_name());
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-o"])
        .arg(&out)
        .arg(&src)
        .status()
        .ok()?;
    status.success().then_some(dir)
}

fn sdk_for(dir: &Path) -> DynamicQnnSdk {
    DynamicQnnSdk::new(LoadPolicy::with_library_dirs([dir.to_path_buf()]))
}

macro_rules! fake_lib {
    ($source:expr) => {
        match build($source) {
            Some(dir) => dir,
            None => {
                eprintln!("skipping: no working C compiler (`cc`) on this host");
                return;
            }
        }
    };
}

#[test]
fn discovers_a_backend_through_real_dynamic_loading() {
    let dir = fake_lib!(&good(2));
    let probe = sdk_for(dir.path()).probe(QnnEngine::Htp).expect("probe");
    assert_eq!(probe.provider_name, "fake_htp");
    assert_eq!(probe.backend_id, 6);
    assert_eq!(probe.core_api.to_string(), "2.4.0");
    assert_eq!(probe.backend_api.to_string(), "5.1.2");
    // GPU and CPU libraries are absent from the same directory.
    assert_eq!(
        sdk_for(dir.path()).probe(QnnEngine::Gpu),
        Err(ProbeFailure::LibraryNotFound)
    );
}

#[test]
fn provider_exposes_only_the_engine_that_loaded() {
    use magnetar_provider_api::Provider;
    let dir = fake_lib!(&good(2));
    let provider =
        QnnProvider::with_policy(LoadPolicy::with_library_dirs([dir.path().to_path_buf()]));
    let devices = provider.devices();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].id().as_str(), "qnn:htp:0");
}

#[test]
fn rejects_an_incompatible_core_api_major() {
    let dir = fake_lib!(&good(3));
    match sdk_for(dir.path()).probe(QnnEngine::Htp) {
        Err(ProbeFailure::IncompatibleCoreApi { found }) => assert_eq!(found.major, 3),
        other => panic!("expected IncompatibleCoreApi, got {other:?}"),
    }
}

#[test]
fn maps_interface_failures_to_distinct_errors() {
    let cases: [(&str, ProbeFailure); 4] = [
        (FAILING, ProbeFailure::InterfaceQueryFailed(1003)),
        (EMPTY, ProbeFailure::NoInterfaceProviders),
        (NO_SYMBOL, ProbeFailure::SymbolMissing),
        (NULL_ENTRY, ProbeFailure::MalformedInterface),
    ];
    for (source, expected) in cases {
        let dir = fake_lib!(source);
        assert_eq!(sdk_for(dir.path()).probe(QnnEngine::Htp), Err(expected));
    }
}

#[test]
fn a_corrupt_library_is_a_load_failure_not_a_crash() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join(QnnEngine::Htp.library_file_name()),
        b"not an elf",
    )
    .unwrap();
    assert_eq!(
        sdk_for(dir.path()).probe(QnnEngine::Htp),
        Err(ProbeFailure::LibraryLoadFailed)
    );
}

#[test]
fn untrusted_directories_are_never_loaded_from() {
    use std::os::unix::fs::PermissionsExt;
    let dir = fake_lib!(&good(2));
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o777)).unwrap();
    assert!(matches!(
        sdk_for(dir.path()).probe(QnnEngine::Htp),
        Err(ProbeFailure::DirectoryRejected(_))
    ));
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(
        DynamicQnnSdk::new(LoadPolicy::default()).probe(QnnEngine::Htp),
        Err(ProbeFailure::NoTrustedDirectory)
    );
}
