#!/usr/bin/env bash
set -euo pipefail

export RUSTUP_NO_UPDATE_CHECK=1
TARGET_TRIPLE="riscv64imac-zero-linux-musl"
PROFILE="dev"
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || (cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd))"
OUT_DIR="${ROOT}/target/${TARGET_TRIPLE}/$([ "$PROFILE" = "dev" ] && echo debug || echo "$PROFILE")"
BIN="${OUT_DIR}/std-smoke"
cd "${ROOT}"

if ! command -v spike >/dev/null 2>&1; then
	echo "Error: 'spike' not found in PATH."
	echo "Hint: install Spike (riscv-isa-sim) and ensure 'spike' is on PATH."
	echo "      You can use: PREFIX=\$HOME/.local/riscv ./scripts/spike-builder build"
	exit 1
fi

echo "Building std-smoke example..."
cargo spike build -p std-smoke --target "${TARGET_TRIPLE}" --mode std --features=std --profile "${PROFILE}" --quiet

echo "Running on Spike simulator..."
OUT="$(mktemp)"
trap 'rm -f "${OUT}"' EXIT

cargo spike run "${BIN}" --isa RV64IMAC --instructions 20000000 | tee "${OUT}"

grep -q "smoke:alloc: ok" "${OUT}"
grep -q "smoke:thread: result=348551" "${OUT}"
grep -q "smoke:thread: ok" "${OUT}"