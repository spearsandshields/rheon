# rheon-feel-wasm

[![crates.io](https://img.shields.io/crates/v/rheon-feel-wasm.svg)](https://crates.io/crates/rheon-feel-wasm)
[![docs.rs](https://docs.rs/rheon-feel-wasm/badge.svg)](https://docs.rs/rheon-feel-wasm)
[![license](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-9370DB.svg)](https://github.com/spearsandshields/rheon#license)

The [rheon](https://github.com/spearsandshields/rheon) FEEL engine as a WebAssembly module:
one exported function, `call`, takes a JSON request (`evaluate`, `parse`, `info`) and returns a
JSON response, with numbers carried as exact 34-digit decimal text. The ABI and the JSON
contract are specified in
[docs/CONTRACT.md](https://github.com/spearsandshields/rheon/blob/main/docs/CONTRACT.md).

This crate is the *source* of the module. Most users want the built artifact instead:

- the npm package [`@rheon/feel-runtime`](https://www.npmjs.com/package/@rheon/feel-runtime)
  ships `feel.wasm` with a TypeScript runtime (Web Worker with timeouts, WASI shim, Node runner);
- every [GitHub Release](https://github.com/spearsandshields/rheon/releases) attaches
  `feel.wasm` and its sha256.

## Building the module

```sh
rustup target add wasm32-wasip1
cargo build --profile wasm --target wasm32-wasip1 -p rheon-feel-wasm
# → target/wasm32-wasip1/wasm/feel_wasm.wasm
```

The `wasm` profile (opt-level z, lto, panic=abort, overflow checks) is defined in the
repository's root `Cargo.toml`; from a crates.io checkout use `--release`. The build script
links Rust's `crt1-reactor.o`, so the module is a WASI reactor (`_initialize`, no per-call
command wrappers), sets an 8 MiB shadow stack and caps linear memory at 512 MiB. Any
WASI-preview1 host runs it: Wasmtime, Node's `node:wasi`, or the ten-function shim in the npm
package for browsers.

The crate also builds natively (an `rlib`), which is how its contract tests run; the same tests
run under Wasmtime with `--target wasm32-wasip1`.

## License

Licensed under either of the MIT license or the Apache License, Version 2.0, at your option.
See `LICENSE-MIT`, `LICENSE` and `NOTICE` in the
[repository](https://github.com/spearsandshields/rheon).
