#![doc = include_str!("../docs/README.md")]
#![deny(rustdoc::broken_intra_doc_links)]

#[macro_use]
extern crate dsntk_macros;

mod bfeel;
mod bifs;
mod equality;
mod errors;
mod evaluator_java;
mod evaluator_pmml;
mod evaluators;
mod filters;
mod flat;
mod infer;
mod invocation;
mod iterations;
mod kernels;
mod macros;
mod session;
mod tests;

pub use crate::bfeel::Dialect;
pub use crate::evaluators::*;
pub use crate::filters::FilterExpressionEvaluator;
pub use crate::infer::{infer_type, infer_type_with, members_of, type_of_text, type_of_value};
pub use crate::invocation::MAX_CALL_DEPTH;
pub use crate::iterations::{Domain, Iteration, Sequence};
pub use crate::session::{Outcome, Session};
