//! # Lowering
//!
//! The instructions of a resolved expression. Every node becomes the instructions that compute
//! what the evaluator of closures computes for it, emitted in the order in which that evaluator
//! evaluates the parts of the node, so that errors, warnings and limits are recorded in the same
//! order. A value known when the expression is built and whose evaluation records nothing is a
//! constant; any other value is computed into a temporary slot, which its one reader moves out.
//! A node that is not lowered yet refuses the whole expression ([NotYet]).

use super::NotYet;
use super::program::{Compare, Op, Opnd, Program, ProgramBuilder, Slot};
use super::resolve::{Info, Resolved, Target};
use crate::Dialect;
use crate::kernels::{self, Arithmetic};
use rheon_feel::values::Value;
use rheon_feel_parser::AstNode;
use std::sync::Arc;

/// The program of the resolved expression `resolved` in `dialect`.
pub(crate) fn lower<'a>(resolved: &Resolved<'a>, dialect: Dialect) -> Result<Arc<Program>, NotYet<'a>> {
  let mut lower = Lower {
    resolved,
    dialect,
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
  dialect: Dialect,
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
          self.fail(dst, text);
        }
      }
      AstNode::Name(_) | AstNode::QualifiedName(_) => self.read(node, dst)?,
      AstNode::Path(_, name) | AstNode::Descendant(_, name) => match name.as_ref() {
        AstNode::Name(name) => {
          let a = self.value(self.resolved.kids(node)[0], None)?;
          let name = self.builder.name(name);
          self.builder.emit(match ast {
            AstNode::Path(..) => Op::Path { dst, a, name },
            _ => Op::Descendant { dst, a, name },
          });
        }
        other => self.fail(dst, crate::builders::err_expected_name(other)),
      },
      AstNode::Add(..) | AstNode::Sub(..) | AstNode::Mul(..) | AstNode::Div(..) | AstNode::Exp(..) => {
        let op = match ast {
          AstNode::Add(..) => Arithmetic::Addition,
          AstNode::Sub(..) => Arithmetic::Subtraction,
          AstNode::Mul(..) => Arithmetic::Multiplication,
          AstNode::Div(..) => Arithmetic::Division,
          _ => Arithmetic::Exponentiation,
        };
        let (a, b) = self.operands(node)?;
        // B-FEEL arithmetic converts mismatched operands (DMN 1.6 chapter 11)
        self.builder.emit(match self.dialect {
          Dialect::Feel => Op::Arith { op, dst, a, b },
          Dialect::BFeel => Op::ArithB { op, dst, a, b },
        });
      }
      AstNode::Neg(_) => {
        let a = self.value(self.resolved.kids(node)[0], None)?;
        self.builder.emit(Op::Neg { dst, a });
      }
      AstNode::Eq(..) | AstNode::Nq(..) | AstNode::Lt(..) | AstNode::Le(..) | AstNode::Gt(..) | AstNode::Ge(..) => {
        let op = match ast {
          AstNode::Eq(..) => Compare::Eq,
          AstNode::Nq(..) => Compare::Ne,
          AstNode::Lt(..) => Compare::Lt,
          AstNode::Le(..) => Compare::Le,
          AstNode::Gt(..) => Compare::Gt,
          _ => Compare::Ge,
        };
        let (a, b) = self.operands(node)?;
        self.builder.emit(Op::Compare { op, dst, a, b });
        self.boolean_default(dst, op == Compare::Ne);
      }
      AstNode::Between(..) => {
        let args = self.temporary();
        self.temporary();
        self.temporary();
        let resolved = self.resolved;
        for (slot, &kid) in (args..).zip(resolved.kids(node)) {
          self.into(kid, slot)?;
        }
        self.builder.emit(Op::Between { dst, args });
        self.boolean_default(dst, false);
      }
      _ => return Err(NotYet { at: ast }),
    }
    self.next = mark;
    Ok(Opnd::take(dst))
  }

  /// The value of `node` into the slot `dst`.
  fn into(&mut self, node: u32, dst: Slot) -> Result<(), NotYet<'a>> {
    let src = self.value(node, Some(dst))?;
    if src != Opnd::take(dst) {
      self.builder.emit(Op::Move { dst, src });
    }
    Ok(())
  }

  /// The operands of the two children of `node`, evaluated in order.
  fn operands(&mut self, node: u32) -> Result<(Opnd, Opnd), NotYet<'a>> {
    let kids = self.resolved.kids(node);
    let (lhs, rhs) = (kids[0], kids[1]);
    Ok((self.value(lhs, None)?, self.value(rhs, None)?))
  }

  /// B-FEEL: a boolean operator never gives null, but `ne` (DMN 1.6 chapter 11).
  fn boolean_default(&mut self, dst: Slot, ne: bool) {
    if self.dialect == Dialect::BFeel {
      self.builder.emit(Op::BoolDefault { dst, ne });
    }
  }

  /// `dst` is null with the error `text`, recorded at every evaluation.
  fn fail(&mut self, dst: Slot, text: String) {
    let msg = self.builder.message(text);
    self.builder.emit(Op::Fail { dst, msg });
  }

  /// A name read into `dst`: in the root expression, from the scope of the host.
  fn read(&mut self, node: u32, dst: Slot) -> Result<(), NotYet<'a>> {
    let rnode = &self.resolved.nodes[node as usize];
    let Info::Read(read) = &rnode.info else {
      // a qualified name with a part that is no segment
      return Err(NotYet { at: rnode.ast });
    };
    if !read.probes.is_empty() {
      return Err(NotYet { at: rnode.ast });
    }
    match (rnode.ast, read.target) {
      (AstNode::Name(name), Target::Free(_)) => {
        let name = self.builder.name(name);
        self.builder.emit(Op::LoadName { dst, name });
      }
      (AstNode::QualifiedName(segments), Target::Free(_) | Target::Search) => {
        let segments = segments
          .iter()
          .filter_map(|segment| match segment {
            AstNode::QualifiedNameSegment(name) => Some(name.clone()),
            _ => None,
          })
          .collect();
        let search = self.builder.search(segments);
        self.builder.emit(Op::Search { dst, search });
      }
      _ => return Err(NotYet { at: rnode.ast }),
    }
    Ok(())
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
