#!/usr/bin/env bash
set -euo pipefail

export RUSTUP_NO_UPDATE_CHECK=1
TARGET_TRIPLE="riscv64imac-zero-linux-musl"
PROFILE="release"
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || (cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd))"
EXAMPLE_DIR="examples/c-smoke"
OUT_DIR="${ROOT}/target/${TARGET_TRIPLE}/$([ "$PROFILE" = "dev" ] && echo debug || echo "$PROFILE")"

echo "Building c-smoke example..."

# Build Rust static library (c-staticlib) using cargo spike build
echo "Building c-staticlib (Rust)..."
cd "${ROOT}"
cargo spike build \
	-p c-staticlib \
	--target "${TARGET_TRIPLE}" \
	--mode std \
	--memory-origin 0x80000000 \
	--memory-size 128Mi \
	--heap-size 64Mi \
	--stack-size 2Mi \
	-- \
	--quiet \
	--profile "${PROFILE}"

# Prepare ZeroOS output layout
echo "Preparing output directories..."
OUTPUT_BASE="${OUT_DIR}/zeroos/c-smoke"
mkdir -p "${OUTPUT_BASE}"
TARGET_SPEC="${OUTPUT_BASE}/${TARGET_TRIPLE}.json"
LINKER_SCRIPT="${OUTPUT_BASE}/linker.ld"
OUTPUT_DIR="${OUTPUT_BASE}"

cargo spike generate target \
	--profile "${TARGET_TRIPLE}" \
	--output "${TARGET_SPEC}"

# Generate linker script
echo "Generating linker script..."
cargo spike generate linker \
	--ram-start 0x80000000 \
	--ram-size 128Mi \
	--heap-size 64Mi \
	--stack-size 2Mi \
	--entry-point _start \
	--output "${LINKER_SCRIPT}"

# Build C application
echo "Building C application..."
cd "${ROOT}/${EXAMPLE_DIR}/c"
LIB_PATH="${OUT_DIR}/libc_staticlib.a"
make clean OUTPUT_DIR="${OUTPUT_DIR}"
make CC="${HOME}/.zeroos/musl/bin/riscv64-linux-musl-gcc" \
	LIB_FFI="${LIB_PATH}" \
	OUTPUT_DIR="${OUTPUT_DIR}" \
	LINKER="${LINKER_SCRIPT}" \
	LIB_DIR="${HOME}/.zeroos/musl/riscv64-linux-musl/lib"

echo "Build complete. Running..."
# Run it using cargo-spike
echo "Running on Spike simulator..."
OUT="$(mktemp)"
trap 'rm -f "${OUT}"' EXIT
cargo spike run "${OUTPUT_DIR}/c-smoke" --isa RV64IMAC --instructions 10000000 | tee "${OUT}"

# Basic correctness check: ensure expected output is present.
grep -q "Testing printf" "${OUT}"
grep -q "smoke:alloc: ok" "${OUT}"
grep -q "smoke:thread: ok" "${OUT}"
