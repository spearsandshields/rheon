//! # The flat evaluator
//!
//! Register instructions over numbered slots, run by one loop, built from the parser's tree after
//! name resolution. It is built beside the evaluator of closures ([crate::builders]): every
//! expression it does not lower yet is refused ([NotYet]) and built as closures, whole, so no
//! expression mixes the two.
//!
//! An expression is resolved ([resolve]: what every name reads, where every binder is kept),
//! lowered to a [program::Program] ([lower]) and run by the loop of [vm], one run per evaluation.

#![cfg_attr(not(test), allow(dead_code))]

mod lower;
mod program;
mod resolve;
#[cfg(test)]
mod tests;
mod vm;

use crate::Dialect;
use rheon_feel::{Evaluator, FeelScope};
use rheon_feel_parser::AstNode;

/// An expression that the flat evaluator does not lower yet, with the node that stopped it.
pub(crate) struct NotYet<'a> {
  /// The node that is not lowered.
  #[cfg_attr(not(feature = "backends"), allow(dead_code))]
  pub(crate) at: &'a AstNode,
}

/// The evaluator of `node` in `dialect`, or why the flat evaluator does not lower it.
pub(crate) fn lower(node: &AstNode, dialect: Dialect) -> Result<Evaluator, NotYet<'_>> {
  let resolved = resolve::resolve(node);
  let program = lower::lower(&resolved, dialect)?;
  #[cfg(feature = "backends")]
  crate::testing::lowered();
  Ok(Box::new(move |scope: &FeelScope| program.run(scope)))
}
