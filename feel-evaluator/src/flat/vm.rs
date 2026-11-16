//! # The run loop
//!
//! One evaluation of a [Program]: its frame of slots, and one loop that runs the instructions in
//! order. Every value is computed by the kernels that the evaluator of closures calls, in the
//! order it calls them, and every error, warning and limit is recorded in the evaluation's
//! diagnostics as that evaluator records it.

use super::program::{Compare, Kernel1, Kernel2, ListKind, Op, Operand, Opnd, Program};
use crate::bfeel;
use crate::kernels::{self, Arithmetic};
use rheon_feel::values::Value;
use rheon_feel::{FeelScope, diagnostics, value_null};
use std::sync::Arc;

impl Program {
  /// Evaluates the program in `scope`.
  pub(crate) fn run(self: &Arc<Self>, scope: &FeelScope) -> Value {
    Vm::new(self, scope).execute()
  }
}

/// The state of one run of a program.
struct Vm<'p> {
  program: &'p Program,
  host: &'p FeelScope,
  slots: Vec<Value>,
  /// The lists being built, innermost last.
  lists: Vec<Vec<Value>>,
}

impl<'p> Vm<'p> {
  fn new(program: &'p Program, host: &'p FeelScope) -> Self {
    Self {
      program,
      host,
      slots: vec![Value::Null(None); program.slots as usize],
      lists: vec![],
    }
  }

  /// The value of the temporary `slot`, which is left null.
  fn take(&mut self, slot: usize) -> Value {
    std::mem::replace(&mut self.slots[slot], Value::Null(None))
  }

  /// The value of an operand; a temporary is left null.
  fn read(&mut self, opnd: Opnd) -> Value {
    match opnd.decode() {
      Operand::Slot(slot) => self.slots[slot as usize].clone(),
      Operand::Take(slot) => self.take(slot as usize),
      Operand::Const(index) => self.program.consts[index as usize].clone(),
    }
  }

  fn execute(&mut self) -> Value {
    let program = self.program;
    let mut pc = 0;
    loop {
      let op = program.code[pc];
      pc += 1;
      match op {
        Op::Fail { dst, msg } => self.slots[dst as usize] = value_null!(&*program.messages[msg as usize]),
        Op::Warn { msg } => diagnostics::warn(&program.messages[msg as usize]),
        Op::LoadName { dst, name } => {
          let name = &program.names[name as usize];
          self.slots[dst as usize] = self.host.get_value(name).unwrap_or_else(|| kernels::free_name(name));
        }
        Op::Search { dst, search } => self.slots[dst as usize] = self.host.search(&program.searches[search as usize]).unwrap_or_else(kernels::free_qualified_name),
        Op::Path { dst, a, name } => {
          let a = self.read(a);
          self.slots[dst as usize] = kernels::path(a, &program.names[name as usize]);
        }
        Op::Descendant { dst, a, name } => {
          let a = self.read(a);
          self.slots[dst as usize] = kernels::descendants(a, &program.names[name as usize]);
        }
        Op::Move { dst, src } => self.slots[dst as usize] = self.read(src),
        Op::Arith { op, dst, a, b } => {
          let (a, b) = (self.read(a), self.read(b));
          self.slots[dst as usize] = op.apply(a, b);
        }
        Op::ArithB { op, dst, a, b } => {
          let (a, b) = (self.read(a), self.read(b));
          self.slots[dst as usize] = bfeel::arithmetic(op, a, b);
        }
        Op::Neg { dst, a } => self.slots[dst as usize] = Arithmetic::negation(self.read(a)),
        Op::Compare { op, dst, a, b } => {
          let (a, b) = (self.read(a), self.read(b));
          self.slots[dst as usize] = match op {
            Compare::Eq => kernels::equal(&a, &b),
            Compare::Ne => kernels::not_equal(&a, &b),
            Compare::Lt => kernels::compare_lt(a, b),
            Compare::Le => kernels::compare_le(a, b),
            Compare::Gt => kernels::compare_gt(a, b),
            Compare::Ge => kernels::compare_ge(a, b),
          };
        }
        Op::Between { dst, args } => {
          let args = args as usize;
          let (a, low, high) = (self.take(args), self.take(args + 1), self.take(args + 2));
          self.slots[dst as usize] = kernels::between(a, low, high);
        }
        Op::BoolDefault { dst, ne } => {
          let value = self.take(dst as usize);
          self.slots[dst as usize] = bfeel::boolean_result(value, ne);
        }
        Op::Kernel1 { kernel, dst, a } => {
          let a = self.read(a);
          self.slots[dst as usize] = match kernel {
            Kernel1::UnaryEq => Value::UnaryEqual(Box::new(a)),
            Kernel1::UnaryNe => Value::UnaryNotEqual(Box::new(a)),
            Kernel1::UnaryLt => Value::UnaryLess(Box::new(a)),
            Kernel1::UnaryLe => Value::UnaryLessOrEqual(Box::new(a)),
            Kernel1::UnaryGt => Value::UnaryGreater(Box::new(a)),
            Kernel1::UnaryGe => Value::UnaryGreaterOrEqual(Box::new(a)),
            Kernel1::IntervalStart(closed) => Value::IntervalStart(Box::new(a), closed),
            Kernel1::IntervalEnd(closed) => Value::IntervalEnd(Box::new(a), closed),
          };
        }
        Op::Kernel2 { kernel, dst, a, b } => {
          let (a, b) = (self.read(a), self.read(b));
          self.slots[dst as usize] = match kernel {
            Kernel2::In => kernels::in_operator(a, b),
            Kernel2::Out => match a {
              Value::Boolean(true) => b,
              _ => value_null!(),
            },
            Kernel2::Range => kernels::range_of(a, b),
          };
        }
        Op::Jump { to } => pc = to as usize,
        Op::JumpUnlessTrue { c, to } => {
          if !matches!(self.read(c), Value::Boolean(true)) {
            pc = to as usize;
          }
        }
        Op::AndLeft { dst, a, to } => {
          let a = self.read(a);
          if matches!(a, Value::Boolean(false)) {
            pc = to as usize;
          }
          self.slots[dst as usize] = a;
        }
        Op::AndRight { dst, b } => {
          let (a, b) = (self.take(dst as usize), self.read(b));
          self.slots[dst as usize] = match (a, b) {
            (Value::Boolean(true), b @ Value::Boolean(_)) => b,
            (Value::Boolean(true), _) => value_null!(),
            (_, Value::Boolean(false)) => Value::Boolean(false),
            _ => value_null!(),
          };
        }
        Op::OrLeft { dst, a, to } => {
          let a = self.read(a);
          if matches!(a, Value::Boolean(true)) {
            pc = to as usize;
          }
          self.slots[dst as usize] = a;
        }
        Op::OrRight { dst, b } => {
          let (a, b) = (self.take(dst as usize), self.read(b));
          self.slots[dst as usize] = match (a, b) {
            (Value::Boolean(false), b @ Value::Boolean(_)) => b,
            (Value::Boolean(false), _) => value_null!(),
            (_, Value::Boolean(true)) => Value::Boolean(true),
            _ => value_null!(),
          };
        }
        Op::ListNew { len } => self.lists.push(Vec::with_capacity(len as usize)),
        Op::ListPush { src } => {
          let value = self.read(src);
          if let Some(list) = self.lists.last_mut() {
            list.push(value);
          }
        }
        Op::ListEnd { kind, dst } => {
          let items = self.lists.pop().unwrap_or_default();
          self.slots[dst as usize] = match kind {
            ListKind::List => kernels::list_of(items),
            ListKind::ExpressionList => Value::ExpressionList(items),
            ListKind::NegatedList => Value::NegatedCommaList(items),
          };
        }
        Op::Return { src } => return self.read(src),
      }
    }
  }
}
