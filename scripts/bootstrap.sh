#!/usr/bin/env bash
# Prepares the tools the wasm build and tests need. Idempotent: safe to re-run.
#
#   1. the `wasm32-wasip1` target for the rustc pinned by rust-toolchain.toml;
#   2. wasmtime in .tools/wasmtime (the test runner for `cargo test --target wasm32-wasip1`).
#
# The engine crates (dsntk-*) are part of this repository, so there is no fork to fetch.
#
# There is no C toolchain any more: the number backend is pure Rust (dashu), so neither wasi-sdk
# nor the Intel decimal library is downloaded. Downloads are verified against pinned sha256 sums
# and unpacked in a temporary directory that is moved into place only when complete. Tool
# directories are keyed by version (.tools/wasmtime-<v>-<platform>, with .tools/wasmtime a
# symlink to the current one), so bumping a version installs the new one.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TOOLS="$ROOT/.tools"
WASMTIME_VERSION=v49.0.3

# --- 1. the wasm target (rust-toolchain.toml pins the compiler) -----------------------------------
(cd "$ROOT" && rustup target add wasm32-wasip1 >/dev/null)
echo "rustup target wasm32-wasip1 ok ($(cd "$ROOT" && rustc --version))"

# --- 2. wasmtime ---------------------------------------------------------------------------------
mkdir -p "$TOOLS"
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64)  WT_PLAT=aarch64-macos; WT_SHA256=d7708ba1cc2da171db2c8273d8602f3e0684935dc7cab40148cdcca1da83c0f8 ;;
  Darwin-x86_64) WT_PLAT=x86_64-macos;  WT_SHA256=bc131e20ed9cb34083fca1a203b38ec9d9e611e96596a10ab0cc9223cb06cd88 ;;
  Linux-x86_64)  WT_PLAT=x86_64-linux;  WT_SHA256=4001225413f208345b980525f31c4080e1873808b38d0ddc7918497340667fbb ;;
  Linux-aarch64) WT_PLAT=aarch64-linux; WT_SHA256=880fcb026da88ad7fc1713d572b0d03d139758b7da96cd3cb8c9cbf847a521df ;;
  *) echo "unsupported platform $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac

sha256() { if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi; }

WT_DIR="wasmtime-$WASMTIME_VERSION-$WT_PLAT"
if [[ ! -x "$TOOLS/$WT_DIR/wasmtime" ]]; then
  echo "==> $WT_DIR"
  STAGE="$(mktemp -d "$ROOT/.bootstrap-tmp.XXXXXX")"
  trap 'rm -rf "$STAGE"' EXIT
  archive="$STAGE/wasmtime.tar.xz"
  curl -fsSL "https://github.com/bytecodealliance/wasmtime/releases/download/$WASMTIME_VERSION/$WT_DIR.tar.xz" -o "$archive"
  actual="$(sha256 "$archive")"
  if [[ "$actual" != "$WT_SHA256" ]]; then
    echo "checksum mismatch for wasmtime: expected $WT_SHA256, got $actual" >&2
    exit 1
  fi
  mkdir -p "$STAGE/unpack"
  tar xJf "$archive" -C "$STAGE/unpack"
  xattr -dr com.apple.quarantine "$STAGE/unpack" 2>/dev/null || true
  rm -rf "$TOOLS/$WT_DIR"
  mv "$STAGE/unpack/$WT_DIR" "$TOOLS/$WT_DIR"
fi
# .tools/wasmtime is a symlink to the current version (scripts/env.sh uses it)
if [[ -e "$TOOLS/wasmtime" && ! -L "$TOOLS/wasmtime" ]]; then rm -rf "$TOOLS/wasmtime"; fi
ln -sfn "$WT_DIR" "$TOOLS/wasmtime"
echo "wasmtime: $("$TOOLS/wasmtime/wasmtime" --version)"
echo "bootstrap ok"
