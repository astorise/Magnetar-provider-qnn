# Architecture

```text
Magnetar Runtime (vendor-neutral)
  | ProviderRegistry: Provider::devices(), health_report()
  v
QnnProvider  (src/provider.rs)
  |-- QnnSdk trait (src/sdk.rs)   <- MockSdk in tests
  |     `-- DynamicQnnSdk: LoadPolicy -> dlopen -> QnnInterface_getProviders
  |-- ffi.rs  (the only `unsafe` in the crate)
  `-- per backend: QnnEngine::{Htp,Gpu,Cpu} -> Device or diagnostic
```

## Boundaries

- One Provider, up to three Devices, never merged into a unified memory
  domain. Device selection stays with Magnetar's Resolution Policy.
- Native pointers, library handles and paths never leave `ffi.rs`/`sdk.rs`.
  `BackendProbe` is plain data.
- No fallback: an absent or incompatible backend yields no Device. Nothing
  here runs on the CPU in place of the NPU/GPU.

## Why built-in, not a dynamic library

`docs/provider-loading-abi.md` in Magnetar defines the `magnetar_provider_v1`
descriptor ABI, but Magnetar's other Providers (`cpu`, `cuda`, `wgpu`, `npu`,
...) are built-in crates over `magnetar-provider-api`, and the descriptor ABI
does not yet carry the execution functions this Provider will need.
Following that convention, this crate exports no `Box<dyn Provider>` across a
library boundary. When the ABI gains execution payloads, a thin
`magnetar_provider_v1` adapter belongs in this repository.

## Lifecycle

1. `QnnProvider::new()` reads `MAGNETAR_QNN_LIB_DIR`, probes every
   engine once, stores a `BackendStatus` each. It cannot fail.
2. `devices()` returns Devices for discovered backends only.
3. `health()`: `Unavailable` (nothing found) or `Degraded` (found, but no
   execution path). It becomes `Available` only when compute exists.
4. Backend libraries are never unloaded (`NeverUnload`).

## Not yet designed here

Graph lowering, context cache, memory registration, execution/cancellation:
Phase 2+, gated on a real target and SDK. Cancellation, when added, must
return `unsupported` where the SDK cannot cancel.
