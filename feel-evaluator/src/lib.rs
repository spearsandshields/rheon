#![doc = include_str!("../docs/README.md")]
#![deny(rustdoc::broken_intra_doc_links)]

#[macro_use]
extern crate dsntk_macros;

mod bfeel;
mod bifs;
mod builders;
mod equality;
mod errors;
#[cfg(not(target_arch = "wasm32"))]
mod evaluator_java;
#[cfg(target_arch = "wasm32")]
#[path = "evaluator_java_stub.rs"]
mod evaluator_java;
mod evaluator_pmml;
mod evaluators;
mod filters;
mod invocation;
mod iterations;
mod kernels;
mod macros;
mod session;
mod tests;

pub use crate::bfeel::Dialect;
pub use crate::evaluators::*;
pub use crate::filters::FilterExpressionEvaluator;
pub use crate::invocation::MAX_CALL_DEPTH;
pub use crate::iterations::{Domain, Iteration, Sequence};
pub use crate::session::{Outcome, Session};
