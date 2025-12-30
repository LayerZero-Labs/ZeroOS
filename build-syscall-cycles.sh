#!/usr/bin/env bash

set -euo pipefail

export RUSTUP_NO_UPDATE_CHECK=1
TARGET_TRIPLE="riscv64imac-zero-linux-musl"
PROFILE="release"
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || (cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd))"
OUT_DIR="${ROOT}/target/${TARGET_TRIPLE}/$([ "$PROFILE" = "dev" ] && echo debug || echo "$PROFILE")"
BIN="${OUT_DIR}/syscall-cycles"
cd "${ROOT}"

# std mode only
echo "Building syscall-cycles example in std mode ..."
if [[ "${PROFILE}" = "release" ]]; then
	# Keep release profile tuning explicit (avoid per-crate [profile.release] warnings).
	CARGO_PROFILE_RELEASE_DEBUG=2 \
	CARGO_PROFILE_RELEASE_STRIP=none \
	CARGO_PROFILE_RELEASE_LTO=true \
	CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
	cargo spike build -p syscall-cycles --target "${TARGET_TRIPLE}" --mode std -- --quiet --features=std --profile "${PROFILE}"
else
	cargo spike build -p syscall-cycles --target "${TARGET_TRIPLE}" --mode std -- --quiet --features=std --profile "${PROFILE}"
fi

# Persist logs under target/ so they survive script exit and are easy to share/debug.
LOG_DIR="${ROOT}/target/syscall-cycles-logs"
mkdir -p "${LOG_DIR}"
TRACE_LOG="${LOG_DIR}/trace.log"
OUT="${LOG_DIR}/out.log"
rm -f "${TRACE_LOG}" "${OUT}"
echo "Running on Spike simulator (trace log: ${TRACE_LOG})..."

# Note: syscall-cycles runs many `ecall`s; this needs a high instruction budget.
RUST_LOG=info cargo spike run "${BIN}" --isa RV64IMAC --instructions 20000000 -l --log="${TRACE_LOG}" | tee "${OUT}"

grep -q "syscall:unknown" "${OUT}"
grep -q "Test PASSED" "${OUT}"

echo "Output: ${OUT}"

UNKNOWN_PC="$(
	riscv64-unknown-elf-objdump -d "${BIN}" | awk '
		/<syscall_unknown>:/ { infn=1 }
		infn && /ecall/ && pc=="" { a=$1; sub(/:$/, "", a); pc="0x"a }
		/^[[:space:]]*$/ { infn=0 }
		END { printf "%s", pc }
	'
)"
echo "UNKNOWN_PC: ${UNKNOWN_PC}"

if [[ -z "${UNKNOWN_PC}" ]]; then
	echo "Failed to locate ecall PC for unknown syscall" >&2
	exit 1
fi

echo "Parsing Spike log for instruction counts..."
EXEC_LOG="${LOG_DIR}/syscall_unknown.exec.log"
cargo xtask spike-syscall-instcount \
	--log "${TRACE_LOG}" \
	--target "${UNKNOWN_PC}:unknown" \
	--dump "${EXEC_LOG}"
