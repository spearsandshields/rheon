#![doc = include_str!("../docs/README.md")]
#![deny(rustdoc::broken_intra_doc_links)]

#[macro_use]
extern crate dsntk_macros;

mod bifs;
mod builders;
mod errors;
#[cfg(not(target_arch = "wasm32"))]
mod evaluator_java;
#[cfg(target_arch = "wasm32")]
#[path = "evaluator_java_stub.rs"]
mod evaluator_java;
mod evaluator_pmml;
mod evaluators;
mod filters;
mod iterations;
mod macros;
mod tests;

pub use crate::builders::{MAX_CALL_DEPTH, call_depth_exceeded, reset_call_depth};
pub use crate::evaluators::*;
pub use crate::filters::FilterExpressionEvaluator;
pub use crate::iterations::{EveryExpressionEvaluator, ForExpressionEvaluator, SomeExpressionEvaluator};
