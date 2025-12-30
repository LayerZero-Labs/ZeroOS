#!/usr/bin/env bash

set -euo pipefail

export RUSTUP_NO_UPDATE_CHECK=1
PROFILE="dev"
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || (cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd))"
cd "${ROOT}"

# no-std mode
echo "Building fibonacci example in no-std mode ..."
TARGET_TRIPLE="riscv64imac-unknown-none-elf"
OUT_DIR="${ROOT}/target/${TARGET_TRIPLE}/$([ "$PROFILE" = "dev" ] && echo debug || echo "$PROFILE")"
BIN="${OUT_DIR}/fibonacci"

cargo spike build -p fibonacci --target "${TARGET_TRIPLE}" -- --quiet --features=debug --profile "${PROFILE}"
OUT_NOSTD="$(mktemp)"
OUT_STD="$(mktemp)"
trap 'rm -f "${OUT_NOSTD}" "${OUT_STD}"' EXIT

RUST_LOG=debug cargo spike run "${BIN}" --isa RV64IMAC --instructions 10000000 | tee "${OUT_NOSTD}"
grep -q "fibonacci(10) = 55" "${OUT_NOSTD}"
grep -q "Test PASSED" "${OUT_NOSTD}"

# std mode
echo "Building fibonacci example in std mode ..."
TARGET_TRIPLE="riscv64imac-zero-linux-musl"
OUT_DIR="${ROOT}/target/${TARGET_TRIPLE}/$([ "$PROFILE" = "dev" ] && echo debug || echo "$PROFILE")"
BIN="${OUT_DIR}/fibonacci"

cargo spike build -p fibonacci --target "${TARGET_TRIPLE}" --mode std -- --quiet --features=std,debug --profile "${PROFILE}"
RUST_LOG=debug cargo spike run "${BIN}" --isa RV64IMAC --instructions 100000000 | tee "${OUT_STD}"
grep -q "fibonacci(10) = 55" "${OUT_STD}"
grep -q "Test PASSED" "${OUT_STD}"
