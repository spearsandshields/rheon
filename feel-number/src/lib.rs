#![doc = include_str!("../docs/README.md")]
#![deny(rustdoc::broken_intra_doc_links)]

#[macro_use]
extern crate dsntk_macros;

mod backend_dashu;
mod decimal128;
mod errors;

pub use decimal128::FeelNumber;

/// Name of the number backend: `"dashu"`, the pure-Rust decimal128 core on dashu-float, the one
/// backend of this crate. Embedders report it in diagnostics (the wasm module's `info` op).
pub const NUMBER_BACKEND: &str = "dashu";
