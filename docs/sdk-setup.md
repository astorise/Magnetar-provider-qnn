# SDK setup

The crate builds without any Qualcomm software. To discover real backends:

1. Install QAIRT from Qualcomm under its own license terms. This repository
   ships no SDK binaries, headers or firmware and must not.
2. Point the Provider at the backend libraries for your target
   (`libQnnHtp.so` etc.):
   `export MAGNETAR_QNN_LIB_DIR=/abs/path/to/qairt/lib/<target>`
   The directory must be absolute and not group/world-writable.
3. Run `cargo test --test hardware -- --ignored` with
   `MAGNETAR_QNN_HARDWARE=1` (see README).

HTP on a device typically also needs the matching Hexagon skel libraries on
the device; that is outside this crate and not verified here.

## Required check before any hardware claim

`src/ffi.rs` declares `Qnn_Version_t`, `Qnn_ApiVersion_t`, the leading fields
of `QnnInterface_t` and the `QnnInterface_getProviders` signature from the
public QNN C API **without access to the SDK headers**. On a machine with the
SDK, compare them against `QnnInterface.h` / `QnnTypes.h` (field order,
widths, `Qnn_ErrorHandle_t` width) and record the SDK version in
`docs/compatibility.md`. Also confirm the core API major version that SDK
reports and adjust `SUPPORTED_CORE_API_MAJOR` if it is not 2.
