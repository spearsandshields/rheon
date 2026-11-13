#!/usr/bin/env bash
# Builds wasm/feel-wasm (crate rheon-feel-wasm) for wasm32-wasip1 with the `wasm` profile
# (root Cargo.toml: opt-level 3, lto, panic=abort, overflow checks) into dist/feel.wasm, prints its size
# (raw / gzip / brotli) and checks the module interface against docs/CONTRACT.md §1:
# imports must all be wasi_snapshot_preview1.*, and the contract exports must be present.
# A plain `cargo build`: the engine is pure Rust, no C toolchain or environment is needed.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

if ! rustup target list --installed 2>/dev/null | grep -qx wasm32-wasip1; then
  echo "the wasm32-wasip1 target is not installed: run ./scripts/bootstrap.sh first" >&2
  exit 1
fi

cd "$ROOT"
# Reproducible output: the only machine-specific content of the release module is the set of
# absolute source paths in panic-location strings (CARGO_HOME for registry crates, this
# repository for the engine crates and feel-wasm). Remap them so that feel.wasm is byte-identical
# whatever the builder's directory layout (rustc's own paths are already /rustc/<hash>). The
# flags are target-specific, so host build scripts are unaffected and the native `cargo test`
# cache is not invalidated.
CARGO_HOME_DIR="${CARGO_HOME:-$HOME/.cargo}"
export CARGO_TARGET_WASM32_WASIP1_RUSTFLAGS="${CARGO_TARGET_WASM32_WASIP1_RUSTFLAGS:-} --remap-path-prefix=$CARGO_HOME_DIR=/cargo-home --remap-path-prefix=$ROOT=/rheon"
echo "==> cargo build --profile wasm --target wasm32-wasip1 -p rheon-feel-wasm (paths remapped: $CARGO_HOME_DIR, $ROOT)"
cargo build --profile wasm --target wasm32-wasip1 -p rheon-feel-wasm

mkdir -p dist
cp target/wasm32-wasip1/wasm/feel_wasm.wasm dist/feel.wasm

node --input-type=module - "$ROOT/dist/feel.wasm" <<'JS'
import { readFileSync } from 'node:fs';
import { gzipSync, brotliCompressSync, constants } from 'node:zlib';
const file = process.argv[2];
const bytes = readFileSync(file);
const kb = (n) => `${(n / 1024).toFixed(0)} KiB (${n} B)`;
const gz = gzipSync(bytes, { level: 9 }).length;
const br = brotliCompressSync(bytes, { params: { [constants.BROTLI_PARAM_QUALITY]: 11 } }).length;
console.log(`dist/feel.wasm  raw ${kb(bytes.length)}  gzip ${kb(gz)}  brotli ${kb(br)}`);

const mod = new WebAssembly.Module(bytes);
const imports = WebAssembly.Module.imports(mod).map((i) => `${i.module}.${i.name}`);
const exports = WebAssembly.Module.exports(mod).map((e) => e.name);
console.log(`imports (${imports.length}): ${imports.join(', ')}`);
console.log(`exports: ${exports.join(', ')}`);
let failed = false;
const foreign = imports.filter((i) => !i.startsWith('wasi_snapshot_preview1.'));
if (foreign.length) { console.error(`ERROR: non-WASI imports: ${foreign.join(', ')}`); failed = true; }
const allowed = new Set(['random_get', 'clock_time_get', 'environ_get', 'environ_sizes_get', 'fd_write', 'fd_close', 'fd_seek',
  'fd_prestat_get', 'fd_prestat_dir_name', 'proc_exit'].map((n) => `wasi_snapshot_preview1.${n}`));
const extra = imports.filter((i) => i.startsWith('wasi_snapshot_preview1.') && !allowed.has(i));
if (extra.length) { console.error(`ERROR: WASI imports outside the contract shim list: ${extra.join(', ')}`); failed = true; }
const missing = ['memory', 'feel_alloc', 'feel_free', 'feel_call', '_initialize'].filter((e) => !exports.includes(e));
if (missing.length) { console.error(`ERROR: missing exports: ${missing.join(', ')} (a reactor must export _initialize, see build.rs)`); failed = true; }

// Memory limits (CONTRACT §1): the shadow stack must be 10 MiB and linear memory capped at 512 MiB.
// Read them from the binary: the memory section (id 5) limits and the initial value of the first
// mutable i32 global (__stack_pointer; with the stack-first layout it equals the stack size).
const readU32 = (buf, pos) => { let result = 0, shift = 0, b; do { b = buf[pos.i++]; result += (b & 0x7f) * 2 ** shift; shift += 7; } while (b & 0x80); return result; };
const readS32 = (buf, pos) => { let result = 0, shift = 0, b; do { b = buf[pos.i++]; result |= (b & 0x7f) << shift; shift += 7; } while (b & 0x80); if (shift < 32 && (b & 0x40)) result |= -1 << shift; return result; };
let maxPages = null, stackPointer = null;
for (const pos = { i: 8 }; pos.i < bytes.length; ) {
  const id = bytes[pos.i++];
  const size = readU32(bytes, pos);
  const end = pos.i + size;
  if (id === 5) {
    readU32(bytes, pos); // count
    const flags = bytes[pos.i++];
    readU32(bytes, pos); // min
    if (flags & 1) maxPages = readU32(bytes, pos);
  } else if (id === 6 && stackPointer === null) {
    const count = readU32(bytes, pos);
    for (let g = 0; g < count && stackPointer === null; g++) {
      const type = bytes[pos.i++], mutable = bytes[pos.i++], op = bytes[pos.i++];
      const value = op === 0x41 ? readS32(bytes, pos) : null;
      while (bytes[pos.i++] !== 0x0b);
      if (type === 0x7f && mutable === 1 && value !== null) stackPointer = value;
    }
  }
  pos.i = end;
}
const MiB = 1024 * 1024;
console.log(`stack: ${stackPointer === null ? '?' : (stackPointer / MiB).toFixed(1) + ' MiB'}  max memory: ${maxPages === null ? 'unbounded' : (maxPages * 65536 / MiB) + ' MiB'}`);
if (stackPointer !== 10 * MiB) { console.error('ERROR: the shadow stack must be 10 MiB (-zstack-size in build.rs)'); failed = true; }
if (maxPages === null || maxPages * 65536 !== 512 * MiB) { console.error('ERROR: linear memory must be capped at 512 MiB (--max-memory in build.rs)'); failed = true; }
if (failed) process.exit(1);
console.log('interface ok');
JS
