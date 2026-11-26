//! # The flat evaluator
//!
//! Register instructions over numbered slots, run by one loop, built from the parser's tree after
//! name resolution. It is the evaluator of every expression: a tree of a shape that the parser
//! never produces (only a tree built by hand has one) is malformed ([Malformed]), and evaluates
//! to null with one error that names the kind of the node that stopped the lowering.
//!
//! An expression is resolved ([resolve]: what every name reads, where every binder is kept),
//! lowered to a [program::Program] ([lower]) and run by the loop of [vm], one run per evaluation.

mod depends;
mod lower;
mod program;
mod resolve;
#[cfg(test)]
mod tests;
pub(crate) mod trace;
mod vm;

pub(crate) use resolve::{Info, LayoutKind, NONE, Place, Read, Region, RegionKind, Resolved, Target, resolve_names, variables};
pub(crate) use vm::{code_of, run_function};

use crate::Dialect;
use rheon_feel::{Evaluator, FeelScope};
use rheon_feel_parser::AstNode;
use std::fmt::{self, Write};

/// A tree of a shape that the parser never produces, with the node that stopped the lowering.
pub(crate) struct Malformed<'a> {
  /// The node that has no lowering.
  pub(crate) at: &'a AstNode,
}

impl Malformed<'_> {
  /// The error of the evaluation of the tree: it names the kind of the node, never its content.
  pub(crate) fn message(&self) -> String {
    format!("malformed expression tree: unexpected {} node", kind_of(self.at))
  }
}

/// The kind of `node` as the parser's tree names it (`Add`, `For`, …): the start of its debug
/// text, whose formatting is stopped after the name.
pub(crate) fn kind_of(node: &AstNode) -> String {
  struct Kind(String);
  impl Write for Kind {
    fn write_str(&mut self, text: &str) -> fmt::Result {
      match text.find(|c: char| !c.is_ascii_alphanumeric()) {
        Some(end) => {
          self.0.push_str(&text[..end]);
          Err(fmt::Error)
        }
        None => {
          self.0.push_str(text);
          Ok(())
        }
      }
    }
  }
  let mut kind = Kind(String::new());
  let _ = write!(kind, "{node:?}");
  kind.0
}

/// The evaluator of `node` in `dialect`, or the node that makes the tree malformed.
pub(crate) fn lower(node: &AstNode, dialect: Dialect) -> Result<Evaluator, Malformed<'_>> {
  let resolved = resolve::resolve(node);
  let program = lower::lower(resolved, dialect, None)?;
  Ok(Box::new(move |scope: &FeelScope| program.run(scope)))
}

/// [lower] with a recording instruction after the value of each node that `targets` gives a
/// number for, and where the loops are that it gives one for: an evaluation inside
/// [trace::recording] keeps those values.
pub(crate) fn lower_traced<'a>(node: &'a AstNode, dialect: Dialect, targets: &crate::Tracing<'_>) -> Result<Evaluator, Malformed<'a>> {
  let resolved = resolve::resolve(node);
  let program = lower::lower(resolved, dialect, Some(targets))?;
  Ok(Box::new(move |scope: &FeelScope| program.run(scope)))
}
