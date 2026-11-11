#!/bin/bash
# Fetches dfp-number-sys 0.2.14 (Rust bindings for the Intel BID128 C library) from crates.io into
# tools/.dfp-number-sys, applies dfp-number-sys.patch (build.rs picks the C defines from the
# TARGET; six bid128 FFI signatures match bid_functions.h) and prints the crate's path.
# Only the tools/ differential harnesses need it: the engine's default number backend is dashu.
set -euo pipefail
cd "$(dirname "$0")"
DIR=.dfp-number-sys
SHA256=56728dc0d3c0fc7b4ae2ebf545743158441b8e6f04998f2418391c5d482b54b8
if [ ! -f "$DIR/.patched" ]; then
  rm -rf "$DIR.tmp" && mkdir -p "$DIR.tmp"
  curl -fsSL https://static.crates.io/crates/dfp-number-sys/dfp-number-sys-0.2.14.crate -o "$DIR.tmp/crate.tar.gz"
  echo "$SHA256  $DIR.tmp/crate.tar.gz" | shasum -a 256 -c - >&2
  tar -xzf "$DIR.tmp/crate.tar.gz" -C "$DIR.tmp" --strip-components=1
  rm "$DIR.tmp/crate.tar.gz"
  patch -d "$DIR.tmp" -p1 --quiet < dfp-number-sys.patch
  touch "$DIR.tmp/.patched"
  rm -rf "$DIR" && mv "$DIR.tmp" "$DIR"
fi
echo "$(pwd)/$DIR"
