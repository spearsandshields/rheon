//! # The flat evaluator
//!
//! Register instructions over numbered slots, run by one loop, built from the parser's tree after
//! name resolution. It is built beside the evaluator of closures ([crate::builders]): every
//! expression it does not lower yet is refused ([NotYet]) and built as closures, whole, so no
//! expression mixes the two. It lowers nothing yet.

#![cfg_attr(not(test), allow(dead_code))]

mod resolve;
#[cfg(test)]
mod tests;

use crate::Dialect;
use rheon_feel::Evaluator;
use rheon_feel_parser::AstNode;

/// An expression that the flat evaluator does not lower yet, with the node that stopped it.
pub(crate) struct NotYet<'a> {
  /// The node that is not lowered.
  #[cfg_attr(not(feature = "backends"), allow(dead_code))]
  pub(crate) at: &'a AstNode,
}

/// The evaluator of `node` in `dialect`, or why the flat evaluator does not lower it.
pub(crate) fn lower(node: &AstNode, _dialect: Dialect) -> Result<Evaluator, NotYet<'_>> {
  Err(NotYet { at: node })
}
