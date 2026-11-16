#!/usr/bin/env bash
# Prints cargo's target directory for this workspace: CARGO_TARGET_DIR, a cargo configuration's
# build.target-dir, or ./target, as cargo itself resolves it.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo metadata --format-version 1 --no-deps | node -e 'let s = ""; process.stdin.on("data", (d) => (s += d)).on("end", () => console.log(JSON.parse(s).target_directory))'
