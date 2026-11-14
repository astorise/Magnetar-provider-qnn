# Compatibility

Recorded at implementation time; re-check before relying on it.

| Item | Value |
| --- | --- |
| Magnetar revision | `16f6c9a70106680272b60c22e951cbaf6769078d` ("Pin the Providers that depend on magnetar-provider-api (#183)") |
| Provider API | `magnetar-provider-api` 1.0.0 (`PROVIDER_API_VERSION` 1) |
| Rust toolchain | 1.100.0 (Magnetar `rust-toolchain.toml`), edition 2024 |
| Reference conventions | `providers/npu` (discovery skeleton), `providers/wgpu`, `providers/cuda` |
| QAIRT/QNN SDK | **none tested** (no SDK in this repository's tooling) |
| Supported QNN core API major | 2 (`SUPPORTED_CORE_API_MAJOR`) -- a declared policy taken from the public API's versioning, not yet checked against an SDK header |
| This repository | initial commit (repository was empty) |

Not done from the specification's reconnaissance list: confirming a
supported QAIRT release, minimum OS/driver requirements, redistributability
and target architectures against Qualcomm's current documentation (the
documentation site is script-rendered and could not be read here). That is a
precondition of Phase 2.
