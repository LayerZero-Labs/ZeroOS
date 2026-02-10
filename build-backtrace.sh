#!/usr/bin/env bash
# Build and run the backtrace example on spike emulator.
#
# This example demonstrates panic handling with stack traces.
# The output should show the panic message and (with symbols) a backtrace.
# Note: The example intentionally panics, so both runs will exit non-zero.

set -euo pipefail

export RUSTUP_NO_UPDATE_CHECK=1
PROFILE="dev"
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || (cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd))"
cd "${ROOT}"

OUT_NOSTD="$(mktemp)"
OUT_STD="$(mktemp)"
trap 'rm -f "${OUT_NOSTD}" "${OUT_STD}"' EXIT

# no-std mode
echo "Building backtrace example in no-std mode ..."
TARGET_TRIPLE="riscv64imac-unknown-none-elf"
OUT_DIR="${ROOT}/target/${TARGET_TRIPLE}/$([ "$PROFILE" = "dev" ] && echo debug || echo "$PROFILE")"
BIN="${OUT_DIR}/backtrace"

# Enable frame pointers for accurate stack traces
export RUSTFLAGS="${RUSTFLAGS:-} -Cforce-frame-pointers=yes"
cargo spike build -p backtrace --target "${TARGET_TRIPLE}" -- --quiet --features=with-spike,backtrace --profile "${PROFILE}"

echo "Running backtrace example (no-std) - expect panic ..."
# The example intentionally panics, so we capture exit code but continue
# Use --symbolize-backtrace to resolve addresses to function names on the host
cargo spike run "${BIN}" --isa RV64IMAC --instructions 10000000 --symbolize-backtrace 2>&1 | tee "${OUT_NOSTD}" || true

# Verify panic message and backtrace appears
grep -q "intentional panic for backtrace demo" "${OUT_NOSTD}"
grep -q "stack backtrace:" "${OUT_NOSTD}"
echo "no-std backtrace test PASSED"
echo ""

# std mode
echo ""
echo "Building backtrace example in std mode ..."
TARGET_TRIPLE="riscv64imac-zero-linux-musl"
OUT_DIR="${ROOT}/target/${TARGET_TRIPLE}/$([ "$PROFILE" = "dev" ] && echo debug || echo "$PROFILE")"
BIN="${OUT_DIR}/backtrace"

cargo spike build -p backtrace --target "${TARGET_TRIPLE}" --mode std --backtrace=enable -- --quiet --features=std,with-spike --profile "${PROFILE}"

echo "Running backtrace example (std) - expect panic ..."
# The example intentionally panics, so we capture exit code but continue
cargo spike run "${BIN}" --isa RV64IMAC --instructions 100000000 --symbolize-backtrace 2>&1 | tee "${OUT_STD}" || true

# Verify panic message appears
grep -q "intentional panic for backtrace demo" "${OUT_STD}"
echo "std backtrace test PASSED"

echo ""
echo "All backtrace tests completed successfully."
