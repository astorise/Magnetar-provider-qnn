//! Real-hardware validation. Opt-in only: set `MAGNETAR_QUALCOMM_HARDWARE=1`
//! and `MAGNETAR_QUALCOMM_LIB_DIR` on a Qualcomm target and run
//! `cargo test --test hardware -- --ignored`. Without the opt-in the tests
//! report themselves skipped; they never pass by inference.

use magnetar_provider_api::Provider;
use magnetar_provider_qualcomm::QualcommProvider;

fn enabled() -> bool {
    let on = std::env::var("MAGNETAR_QUALCOMM_HARDWARE").is_ok_and(|v| v == "1");
    if !on {
        eprintln!("SKIPPED: MAGNETAR_QUALCOMM_HARDWARE=1 not set");
    }
    on
}

/// Phase 1 evidence only: a real backend library loads and answers the QNN
/// interface query. This does NOT show that any graph executes on HTP.
#[test]
#[ignore = "requires a Qualcomm target with the QAIRT runtime installed"]
fn a_real_qnn_backend_is_discovered() {
    if !enabled() {
        return;
    }
    let provider = QualcommProvider::new();
    for diagnostic in provider.diagnostics() {
        eprintln!(
            "{:?}: {} - {}",
            diagnostic.engine, diagnostic.code, diagnostic.message
        );
    }
    assert!(provider.is_available(), "no QNN backend discovered");
    assert!(!provider.devices().is_empty());
}
