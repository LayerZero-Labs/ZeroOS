#!/bin/bash
set -euo pipefail

PREFIX="${PREFIX:-/opt/riscv}"
OUTDIR="${OUTDIR:-/srv}"
NAME="${NAME:-spike}"

log(){ echo -e "\033[1;32m==> $*\033[0m"; }
err(){ echo -e "\033[1;31m[!] $*\033[0m" >&2; exit 1; }

version(){ "$PREFIX/bin/spike" --help 2>&1 | sed -n 's/.*\([0-9]\+\.[0-9]\+\.[0-9]\+\).*/\1/p' || echo unknown; }

build(){
  rm -rf /tmp/riscv-isa-sim
  git clone --depth=1 https://github.com/riscv-software-src/riscv-isa-sim.git /tmp/riscv-isa-sim
  cd /tmp/riscv-isa-sim && mkdir build && cd build
  ../configure --prefix="$PREFIX" LDFLAGS="-s" && make -j"$(nproc)" install
  log "Installed Spike $(version)"
}

package(){
  [[ -x "$PREFIX/bin/spike" ]] || err "Spike not found in $PREFIX/bin"
  ver=$(version)
  tarball="$OUTDIR/${NAME}-${ver}-$(uname -s)-$(uname -m).tar.gz"
  mkdir -p "$OUTDIR"
  tar zcf "$tarball" -C "$PREFIX" .
  log "Created $tarball ($(du -h "$tarball" | cut -f1))"
}

case "${1:-}" in
  build) build;;
  package) package;;
  *) err "Usage: $0 spike|package";;
esac
