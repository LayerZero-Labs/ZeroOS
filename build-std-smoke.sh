#!/usr/bin/env bash
set -euo pipefail

export RUSTUP_NO_UPDATE_CHECK=1

TARGET_TRIPLE="riscv64imac-zero-linux-musl"
PROFILE="dev"
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || (cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd))"
OUT_DIR="${ROOT}/target/${TARGET_TRIPLE}/$([ "$PROFILE" = "dev" ] && echo debug || echo "$PROFILE")"
BIN="${OUT_DIR}/std-smoke"
cd "${ROOT}"

echo "Building std-smoke example..."
cargo spike build -p std-smoke --target "${TARGET_TRIPLE}" --mode std --backtrace=enable -- --quiet --features=std,backtrace --profile "${PROFILE}"

echo "Running on Spike simulator..."
OUT="$(mktemp)"
trap 'rm -f "${OUT}"' EXIT

cargo spike run "${BIN}" --isa RV64IMAC --instructions 200000000 | tee "${OUT}"

grep -q "smoke:alloc: ok" "${OUT}"
grep -q "smoke:thread: result=348551" "${OUT}"
grep -q "smoke:thread: ok" "${OUT}"
