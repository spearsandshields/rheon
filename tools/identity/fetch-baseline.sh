#!/usr/bin/env bash
# Puts the reference module of the identity check in .tools/baseline/feel.wasm: the feel.wasm asset
# of the GitHub Release named in baseline.json, verified against its sha256 there. Fails closed: a
# download whose sum does not match is deleted. A file already in place with the right sum is
# kept, so the check also runs offline when the release asset was copied there.
# Needs the GitHub CLI (`gh`, with GH_TOKEN in CI) only when it downloads.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
field() { node -e 'const v = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"))[process.argv[2]]; if (typeof v !== "string") process.exit(1); console.log(v)' "$here/baseline.json" "$1"; }
sha256() { if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi; }
tag="$(field tag)"
expected="$(field sha256)"
dir="$root/.tools/baseline"
file="$dir/feel.wasm"
if [ -f "$file" ] && [ "$(sha256 "$file")" = "$expected" ]; then
  echo "baseline $tag: $file (sha256 verified)"
  exit 0
fi
rm -f "$file"
mkdir -p "$dir"
gh release download "$tag" --pattern feel.wasm --dir "$dir"
actual="$(sha256 "$file")"
if [ "$actual" != "$expected" ]; then
  rm -f "$file"
  echo "error: feel.wasm of $tag has sha256 $actual, baseline.json expects $expected" >&2
  exit 1
fi
echo "baseline $tag: $file (downloaded, sha256 verified)"
