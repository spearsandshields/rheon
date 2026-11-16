//! # Lowering
//!
//! The instructions of a resolved expression. Every node becomes the instructions that compute
//! what the evaluator of closures computes for it, emitted in the order in which that evaluator
//! evaluates the parts of the node, so that errors, warnings and limits are recorded in the same
//! order. A value known when the expression is built and whose evaluation records nothing is a
//! constant; any other value is computed into a temporary slot, which its one reader moves out.
//! A node that is not lowered yet refuses the whole expression ([NotYet]).

use super::NotYet;
use super::program::{Op, Opnd, Program, ProgramBuilder, Slot};
use super::resolve::Resolved;
use crate::{Dialect, kernels};
use rheon_feel::values::Value;
use rheon_feel_parser::AstNode;
use std::sync::Arc;

/// The program of the resolved expression `resolved` in `dialect`.
pub(crate) fn lower<'a>(resolved: &Resolved<'a>, _dialect: Dialect) -> Result<Arc<Program>, NotYet<'a>> {
  let mut lower = Lower {
    resolved,
    builder: ProgramBuilder::default(),
    next: 0,
    max: 0,
  };
  let result = lower.value(resolved.root(), None)?;
  lower.builder.emit(Op::Return { src: result });
  Ok(lower.builder.finish(lower.max))
}

struct Lower<'r, 'a> {
  resolved: &'r Resolved<'a>,
  builder: ProgramBuilder,
  /// The first free temporary.
  next: Slot,
  /// The slots used so far.
  max: u32,
}

impl<'a> Lower<'_, 'a> {
  fn temporary(&mut self) -> Slot {
    let slot = self.next;
    self.next += 1;
    self.max = self.max.max(self.next);
    slot
  }

  /// The operand of the value of `node`: a constant, or a temporary that the instructions emitted
  /// here compute (`dst` when given).
  fn value(&mut self, node: u32, dst: Option<Slot>) -> Result<Opnd, NotYet<'a>> {
    let ast = self.resolved.nodes[node as usize].ast;
    if let Some(constant) = self.constant(ast) {
      return Ok(constant);
    }
    let dst = dst.unwrap_or_else(|| self.temporary());
    let mark = self.next;
    match ast {
      // a literal without a value (one with a value is a constant)
      AstNode::Numeric(..) | AstNode::At(_) => {
        if let Some(Err(text)) = literal(ast) {
          let msg = self.builder.message(text);
          self.builder.emit(Op::Fail { dst, msg });
        }
      }
      _ => return Err(NotYet { at: ast }),
    }
    self.next = mark;
    Ok(Opnd::take(dst))
  }

  /// The constant of `ast` when its value is known now: a literal whose evaluation records nothing
  /// but a warning, which is recorded where it is evaluated.
  fn constant(&mut self, ast: &AstNode) -> Option<Opnd> {
    let value = match ast {
      AstNode::Boolean(value) => Value::Boolean(*value),
      AstNode::Null => Value::Null(None),
      AstNode::String(text) => Value::String(text.as_str().into()),
      AstNode::Numeric(..) | AstNode::At(_) => {
        let (value, warning) = literal(ast)?.ok()?;
        if let Some(warning) = warning {
          let msg = self.builder.message(warning);
          self.builder.emit(Op::Warn { msg });
        }
        value
      }
      _ => return None,
    };
    Some(self.builder.konst(value))
  }
}

/// The value of a number or an `@` literal, with the warning its evaluation records, or the error
/// of a literal that has no value; `None` for another node.
fn literal(ast: &AstNode) -> Option<Result<(Value, Option<String>), String>> {
  match ast {
    AstNode::Numeric(before, after, sign, exponent) => Some(kernels::number_literal(before, after, *sign, exponent).map(|number| (Value::Number(number), None))),
    AstNode::At(text) => Some(kernels::at_literal(text)),
    _ => None,
  }
}
