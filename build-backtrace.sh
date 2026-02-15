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

# Use frame pointer walking for no-std mode (lightweight)
cargo spike build -p backtrace --target "${TARGET_TRIPLE}" --backtrace=frame-pointers -- --quiet --features=with-spike --profile "${PROFILE}"

echo "Running backtrace example (no-std) - expect panic ..."
# The example intentionally panics, so we capture exit code but continue
# Use --symbolize-backtrace to resolve addresses to function names on the host
cargo spike run "${BIN}" --isa RV64IMAC --instructions 10000000 --symbolize-backtrace 2>&1 | tee "${OUT_NOSTD}" || true

# Verify panic message and backtrace appears
grep -q "intentional panic for backtrace demo" "${OUT_NOSTD}"
grep -q "stack backtrace:" "${OUT_NOSTD}"
echo "no-std + frame-pointers test PASSED"
echo ""

# no-std mode with backtrace off
echo "Building backtrace example in no-std mode (off) ..."
cargo spike build -p backtrace --target "${TARGET_TRIPLE}" --backtrace=off -- --quiet --features=with-spike --profile "${PROFILE}"

echo "Running backtrace example (no-std + off) - expect panic without backtrace ..."
OUT_NOSTD_OFF="$(mktemp)"
trap 'rm -f "${OUT_NOSTD}" "${OUT_STD}" "${OUT_NOSTD_OFF}"' EXIT
cargo spike run "${BIN}" --isa RV64IMAC --instructions 10000000 2>&1 | tee "${OUT_NOSTD_OFF}" || true

grep -q "intentional panic for backtrace demo" "${OUT_NOSTD_OFF}"
# Verify NO backtrace
if grep -q "stack backtrace:" "${OUT_NOSTD_OFF}"; then
	echo "ERROR: backtrace appeared in off mode!"
	exit 1
fi
echo "no-std + off test PASSED (no backtrace as expected)"
echo ""

# std mode
echo ""
echo "Building backtrace example in std mode ..."
TARGET_TRIPLE="riscv64imac-zero-linux-musl"
OUT_DIR="${ROOT}/target/${TARGET_TRIPLE}/$([ "$PROFILE" = "dev" ] && echo debug || echo "$PROFILE")"
BIN="${OUT_DIR}/backtrace"

cargo spike build -p backtrace --target "${TARGET_TRIPLE}" --mode std --backtrace=dwarf -- --quiet --features=std,with-spike --profile "${PROFILE}"

echo "Running backtrace example (std) - expect panic ..."
# The example intentionally panics, so we capture exit code but continue
cargo spike run "${BIN}" --isa RV64IMAC --instructions 100000000 --symbolize-backtrace 2>&1 | tee "${OUT_STD}" || true

# Verify panic message appears
grep -q "intentional panic for backtrace demo" "${OUT_STD}"
echo "std backtrace test PASSED"

echo ""
echo ""
echo "Building backtrace example in std mode (frame-pointers) ..."
cargo spike build -p backtrace --target "${TARGET_TRIPLE}" --mode std --backtrace=frame-pointers -- --quiet --features=std,with-spike --profile "${PROFILE}"

echo "Running backtrace example (std + frame-pointers) - expect panic ..."
OUT_STD_FP="$(mktemp)"
trap 'rm -f "${OUT_NOSTD}" "${OUT_STD}" "${OUT_NOSTD_OFF}" "${OUT_STD_FP}"' EXIT
cargo spike run "${BIN}" --isa RV64IMAC --instructions 100000000 --symbolize-backtrace 2>&1 | tee "${OUT_STD_FP}" || true

grep -q "intentional panic for backtrace demo" "${OUT_STD_FP}"
# Note: std + frame-pointers doesn't print backtrace with default panic hook
# (Rust's std::backtrace requires DWARF; would need custom panic hook for frame-pointers)
# Just verify binary builds and runs
grep -q "stack backtrace:" "${OUT_STD_FP}"
echo "std + frame-pointers backtrace test PASSED"

echo ""
echo "Building backtrace example in std mode (off) ..."
cargo spike build -p backtrace --target "${TARGET_TRIPLE}" --mode std --backtrace=off -- --quiet --features=std,with-spike --profile "${PROFILE}"

echo "Running backtrace example (std + off) - expect panic without backtrace ..."
OUT_STD_OFF="$(mktemp)"
trap 'rm -f "${OUT_NOSTD}" "${OUT_STD}" "${OUT_STD_FP}" "${OUT_STD_OFF}"' EXIT
cargo spike run "${BIN}" --isa RV64IMAC --instructions 100000000 --symbolize-backtrace 2>&1 | tee "${OUT_STD_OFF}" || true

grep -q "intentional panic for backtrace demo" "${OUT_STD_OFF}"
# Verify NO backtrace appears
if grep -q "stack backtrace:" "${OUT_STD_OFF}"; then
	echo "ERROR: backtrace appeared in off mode!"
	exit 1
fi
echo "std + off mode test PASSED (no backtrace as expected)"

echo ""
echo "All backtrace tests completed successfully."
echo "  - no-std + frame-pointers: ✓"
echo "  - no-std + off: ✓"
echo "  - std + dwarf: ✓"
echo "  - std + frame-pointers: ✓"
echo "  - std + off: ✓"
