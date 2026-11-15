# Security

- Backend libraries are trusted native code running in the Runtime process,
  not sandboxed plugins. Loading one executes its initializers.
- Libraries are loaded only by full path from directories listed in
  `MAGNETAR_QNN_LIB_DIR`. No `PATH`/`LD_LIBRARY_PATH`/working-directory
  search. Directories must be absolute, existing, and (Unix) not
  group/world-writable. Unset means nothing is loaded.
- Libraries are never unloaded (`NeverUnload`).
- `ProbeFailure` carries no path or native error text. Backend-supplied
  strings are reduced to <= 64 printable ASCII characters. Health messages
  also pass through Magnetar's own diagnostic redaction.
- No tensors, model data or proprietary context bytes are handled yet.
- Panics inside probing are caught and become `ProbeFailure::Panicked`; the
  crate exports no `extern "C"` functions, so no unwinding crosses an ABI.
- Limitation: a malicious or ABI-mismatched library in a trusted directory
  can still crash or compromise the process. Directory trust is the control.
