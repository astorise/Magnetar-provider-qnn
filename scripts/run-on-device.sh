#!/usr/bin/env bash
# Runs the cross-compiled test bundle on an Android device through adb and
# records the outcome. Usable from a Qualcomm Device Cloud session that gives
# you adb, or from any machine with a Snapdragon phone attached.
#
#   ./run-on-device.sh <device-lib-dir> [adb-serial]
#
# Transport (default adb). For an SSH session (e.g. Qualcomm Device Cloud):
#   TRANSPORT=ssh SSH_TARGET=user@host SSH_PORT=22 SSH_KEY=~/.ssh/key \
#     ./run-on-device.sh <device-lib-dir>
# Host keys are trusted on first use and never stored (sessions are
# short-lived), which does not protect against an active MITM on first
# connect. Set REMOTE_DIR if /data/local/tmp is not writable for that user.
# Device model/build properties are only recorded with DEVICE_INFO=1: they
# can be confidential and results are often uploaded as public artifacts.
#
# <device-lib-dir> is an ABSOLUTE directory ON THE DEVICE holding the QNN
# backend libraries (libQnnHtp.so, ...). The libraries are Qualcomm's and are
# never part of this bundle; put them there yourself. The directory must not
# be group- or world-writable, or the Provider refuses to load from it.
#
# Output: results/<utc timestamp>/{device.txt,unit-tests.log,hardware-tests.log,result.json}
# Exit code 0 only if every executed suite passed.
set -euo pipefail

LIB_DIR="${1:?usage: $0 <device-lib-dir> [adb-serial]}"
SERIAL="${2:-}"
HERE="$(cd "$(dirname "$0")" && pwd)"
TRANSPORT="${TRANSPORT:-adb}"
REMOTE="${REMOTE_DIR:-/data/local/tmp/magnetar-qnn}"

case "$LIB_DIR" in /*) ;; *) echo "device lib dir must be absolute" >&2; exit 2 ;; esac

case "$TRANSPORT" in
  adb)
    command -v adb >/dev/null || { echo "adb not found" >&2; exit 2; }
    adb_() { if [ -n "$SERIAL" ]; then adb -s "$SERIAL" "$@"; else adb "$@"; fi; }
    dev_shell() { adb_ shell "$1"; }
    dev_push() { adb_ push "$@" >/dev/null; } # files..., remote dir
    ;;
  ssh)
    : "${SSH_TARGET:?SSH_TARGET (user@host) is required}"
    SSH_OPTS=(-p "${SSH_PORT:-22}" -o BatchMode=yes -o StrictHostKeyChecking=accept-new
              -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR -o ConnectTimeout=30)
    [ -n "${SSH_KEY:-}" ] && SSH_OPTS+=(-i "$SSH_KEY")
    dev_shell() { ssh "${SSH_OPTS[@]}" "$SSH_TARGET" "$1"; }
    dev_push() { # files..., remote dir
      local dest="${*: -1}"; local files=("${@:1:$#-1}")
      scp "${SSH_OPTS[@]/#-p/-P}" "${files[@]}" "$SSH_TARGET:$dest/" >/dev/null
    }
    ;;
  *) echo "unknown TRANSPORT '$TRANSPORT'" >&2; exit 2 ;;
esac
( cd "$HERE" && sha256sum -c SHA256SUMS >/dev/null ) || { echo "bundle checksum mismatch" >&2; exit 2; }

OUT="$HERE/results/$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$OUT"

# Exact device facts for the support matrix (docs/support-matrix.md), only
# when asked: they may be confidential (see header).
: > "$OUT/device.txt"
[ "${DEVICE_INFO:-0}" = 1 ] && {
  for prop in ro.product.manufacturer ro.product.model ro.product.device \
              ro.soc.manufacturer ro.soc.model ro.board.platform \
              ro.build.fingerprint ro.build.version.release ro.build.version.sdk \
              ro.product.cpu.abi; do
    echo "$prop=$(dev_shell "getprop $prop" | tr -d '\r')"
  done
  echo "kernel=$(dev_shell 'uname -a' | tr -d '\r')"
} >> "$OUT/device.txt" || true

dev_shell "rm -rf $REMOTE && mkdir -p $REMOTE/tmp"
dev_push "$HERE/unit-tests" "$HERE/hardware-tests" "$REMOTE"
dev_shell "chmod 755 $REMOTE/unit-tests $REMOTE/hardware-tests"

# Runs one suite; prints its exit code. `adb shell` does not propagate the
# remote exit status on every Android version, so it is echoed explicitly.
run_suite() { # name, extra test args
  local name="$1"; shift
  local status
  status=$(dev_shell "cd $REMOTE && TMPDIR=$REMOTE/tmp \
      MAGNETAR_QNN_HARDWARE=1 MAGNETAR_QNN_LIB_DIR='$LIB_DIR' \
      ./$name $* --nocapture; echo __EXIT__=\$?" \
    | tr -d '\r' | tee "$OUT/$name.log" | sed -n 's/^__EXIT__=//p' | tail -1)
  echo "${status:-missing}"
}

unit_status=$(run_suite unit-tests)
hw_status=$(run_suite hardware-tests --ignored)

# A hardware test that prints SKIPPED did not run its assertions; report it
# as skipped, never as passed.
hw_result=failed
if [ "$hw_status" = 0 ]; then
  if grep -q '^SKIPPED' "$OUT/hardware-tests.log"; then hw_result=skipped; else hw_result=passed; fi
fi
unit_result=failed; [ "$unit_status" = 0 ] && unit_result=passed

cat > "$OUT/result.json" <<JSON
{
  "unit_tests": "$unit_result",
  "hardware_discovery": "$hw_result",
  "lib_dir_on_device": "$LIB_DIR",
  "note": "hardware_discovery passed means a QNN backend library loaded and answered QnnInterface_getProviders. It does not mean any graph ran on HTP/GPU."
}
JSON
cat "$OUT/result.json"
echo "results in $OUT"
[ "$unit_result" = passed ] && [ "$hw_result" = passed ]
