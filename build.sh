#!/usr/bin/env sh
# Builds dist/pulldown_cmark_wasm.wasm. Local paths (cargo home, sysroot,
# checkout) are remapped so the binary does not depend on the build machine.
set -eu
cd "$(dirname "$0")"
CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
SYSROOT="$(rustc --print sysroot)"
RUSTFLAGS="--remap-path-prefix=$CARGO_HOME=/cargo --remap-path-prefix=$SYSROOT=/rustc --remap-path-prefix=$(pwd)=/src" \
  cargo build --release --target wasm32-unknown-unknown
mkdir -p dist
cp target/wasm32-unknown-unknown/release/pulldown_cmark_wasm.wasm dist/
