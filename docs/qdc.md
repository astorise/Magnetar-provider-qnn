# Running on Qualcomm Device Cloud (QDC)

Build on GitHub, run on the device: device hours are spent only executing
prebuilt binaries.

## 1. Build the bundle

Run the **Build for Qualcomm Device Cloud** workflow (Actions tab, "Run
workflow"; optionally set the Magnetar ref). It cross-compiles the unit tests
and the `hardware` test for `aarch64-linux-android` (API 24) inside a
Magnetar checkout and uploads the artifact `qnn-tests-aarch64-linux-android`
(14 days). The bundle holds `unit-tests`, `hardware-tests`,
`run-on-device.sh`, `SHA256SUMS` and `MANIFEST.txt` (provider and Magnetar
SHAs, rustc). It contains no Qualcomm files.

The workflow cannot be dispatched from GitHub until it exists on the default
branch; on pull requests it runs automatically when the crate changes.

## 2. Put the QNN libraries on the device

The QAIRT SDK is licensed by Qualcomm and is not redistributed here. Copy the
`aarch64-android` backend libraries you need (for HTP, `libQnnHtp.so` plus
the Hexagon stub/skel libraries your SDK release requires for the device's
DSP architecture) into one directory on the device, for example
`/data/local/tmp/qnn-libs`. Keep it owner-writable only
(`chmod 755`, not 775/777), otherwise the Provider refuses to load from it.
Which skel files your chip needs comes from the SDK documentation; this
repository has not verified that.

## 3. Run

With `adb` reaching the device (a QDC session that exposes adb, or a phone on
USB):

```bash
unzip qnn-tests-aarch64-linux-android.zip -d bundle && cd bundle
./run-on-device.sh /data/local/tmp/qnn-libs [adb-serial]
```

It verifies the checksums, records device facts, runs both suites under
`/data/local/tmp/magnetar-qnn`, and writes
`results/<timestamp>/{device.txt, unit-tests.log, hardware-tests.log, result.json}`.
Exit code 0 means both suites passed.

`hardware_discovery: passed` means a backend library loaded and answered
`QnnInterface_getProviders`. It is **not** evidence that a graph ran on the
HTP or GPU, and the support matrix only changes with the exact device,
chipset, OS build, SDK version and the logs above.

## What this does not do

- It does not start or reserve a QDC session; that depends on how QDC exposes
  your devices (this repository has not verified an API for it).
- It does not install the SDK libraries.
- Windows ARM64 devices are not covered yet (`aarch64-pc-windows-msvc` would
  need a Windows build job and a PowerShell runner script).
