# Source me: sets wasmtime as the test runner for `cargo test --target wasm32-wasip1`.
# Nothing else is needed: the engine is pure Rust (dashu number backend), so no C compiler,
# sysroot or extra link flags are involved.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")/.." && pwd)"
export CARGO_TARGET_WASM32_WASIP1_RUNNER="$ROOT/.tools/wasmtime/wasmtime"
