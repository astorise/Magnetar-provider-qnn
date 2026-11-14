# magnetar-provider-qualcomm

An external Qualcomm execution Provider for the
[Magnetar](https://github.com/astorise/Magnetar) local AI Runtime, built on
the Qualcomm AI Runtime (QAIRT / QNN). One logical `QualcommProvider`
exposes one Magnetar `Device` per QNN backend that is actually present:
Hexagon HTP (NPU), Adreno GPU and Qualcomm CPU.

## Status: Phase 1 -- lifecycle and discovery. No compute yet.

| Capability | State |
| --- | --- |
| Trusted-path, run-time loading of QNN backend libraries | implemented, tested |
| Per-backend discovery (`QnnInterface_getProviders`), version gate | implemented, tested against a locally compiled fake library and a mock |
| One Device per backend (`qualcomm:htp:0`, `qualcomm:gpu:0`, `qualcomm:cpu:0`) | implemented, tested |
| Structured, redacted per-backend diagnostics and health | implemented, tested |
| Kernels, graph lowering, tensor movement, `ProviderExecutionApi` | **not implemented** |
| Context cache, quantization, LLM path | **not implemented** |
| Verification on real Qualcomm hardware | **none yet** |

What this means in practice:

- With no SDK, no trusted directory or no backend library, the Provider is
  `Unavailable` with zero Devices and a diagnostic per backend. It never
  fabricates a Device.
- With a backend found, its Device is registered and the Provider reports
  `Degraded`: the software is present but this Provider cannot execute on it
  yet. It advertises **no Kernels**, so Magnetar's Resolution Policy never
  routes work to it.
- Devices advertise no dtype, layout, memory class or capacity: none of that
  has been proven by a query or a validated graph.
- The QNN FFI declarations (`src/ffi.rs`) were written **without a QAIRT SDK
  to compare against**. `docs/sdk-setup.md` lists the header check that must
  pass on a machine with the SDK before any hardware claim. A successful
  mock or fake-library run is not HTP/GPU evidence.

Not a "supported on Snapdragon X" claim of any kind; see
`docs/support-matrix.md` (every row is `not tested`).

## Use

The Provider is a built-in Magnetar Provider, like `providers/cuda` and
`providers/npu`: the stable dynamic ABI does not yet expose the execution
functions this Provider needs (see `docs/architecture.md`).

```rust
use magnetar_provider_qualcomm::QualcommProvider;
use magnetar_provider_api::Provider;

// Reads trusted directories from MAGNETAR_QUALCOMM_LIB_DIR.
let provider = QualcommProvider::new();
for d in provider.diagnostics() {
    println!("{:?}: {} - {}", d.engine, d.code, d.message);
}
let devices = provider.devices();
```

`MAGNETAR_QUALCOMM_LIB_DIR` is a `PATH`-style list of absolute directories
containing `libQnnHtp.so` / `libQnnGpu.so` / `libQnnCpu.so` (`Qnn*.dll` on
Windows). Relative, missing or group/world-writable directories are rejected.
No other location is searched.

## Test

```bash
# From a Magnetar checkout with this repository at providers/qualcomm
cargo fmt --manifest-path providers/qualcomm/Cargo.toml -- --check
cargo clippy --locked --manifest-path providers/qualcomm/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path providers/qualcomm/Cargo.toml
```

No Qualcomm SDK or hardware is needed. `tests/fake_sdk.rs` compiles a tiny C
library with the system `cc` (skipped if absent, Linux only) to exercise the
real `dlopen` path and each failure mapping.

Hardware validation is opt-in and separate:

```bash
MAGNETAR_QUALCOMM_HARDWARE=1 MAGNETAR_QUALCOMM_LIB_DIR=/path/to/qairt/lib \
  cargo test --test hardware -- --ignored
```

Without the opt-in these tests print `SKIPPED`; they never pass by inference.

## Documents

- `docs/architecture.md` -- boundaries, lifecycle, why built-in.
- `docs/compatibility.md` -- pinned Magnetar revision, toolchain, SDK status.
- `docs/support-matrix.md` -- per device/OS/backend status.
- `docs/sdk-setup.md` -- SDK setup, header verification, licensing caveats.
- `docs/security.md` -- trusted loading, diagnostics redaction.
- `docs/performance.md` -- benchmark method (no numbers yet).

## Remaining roadmap

Phases 2-5 of the implementation specification: a first executable operator
on a real HTP target compared against Reference CPU, graph lowering and
context cache, a Qwen inference slice, then Adreno/Windows ARM64/embedded.
Each is blocked on hardware access and a QAIRT SDK, and none is claimed.

## Continuous integration

`.github/workflows/ci.yml` calls Magnetar's shared module workflow
(`submodule-ci.yml`): formatting, Clippy with warnings denied, tests with the
committed `Cargo.lock`, `cargo-deny`. The crate is built inside a Magnetar
checkout at `providers/qualcomm`. `.github/workflows/hardware-qualcomm.yml`
is manual and runs only on a runner labelled `qualcomm-hardware`.

## License

MIT. This repository contains no Qualcomm SDK binaries or headers.
