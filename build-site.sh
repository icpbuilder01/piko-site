#!/bin/bash
# Usage: ./build-site.sh <site dist dir> <output.wasm>
# Embeds the site into the module. Same dist + same Cargo.lock + same
# toolchain = same module hash, on any machine (paths are remapped).
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
dist="$(cd "$1" && pwd)"
out="$2"
export PIKO_SITE_DIR="$dist"
export RUSTFLAGS="--remap-path-prefix=$HOME/.cargo=/cargo --remap-path-prefix=$here=/piko-site --remap-path-prefix=$dist=/site"
cd "$here"
cargo build --release --locked --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/piko_site.wasm "$out"
sha256sum "$out"
