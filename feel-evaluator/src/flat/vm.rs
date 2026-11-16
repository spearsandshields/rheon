//! # The run loop
//!
//! One evaluation of a [Program]: its frame of slots, and one loop that runs the instructions in
//! order. Every value is computed by the kernels that the evaluator of closures calls, in the
//! order it calls them, and every error, warning and limit is recorded in the evaluation's
//! diagnostics as that evaluator records it.

use super::program::{Op, Operand, Opnd, Program};
use crate::kernels;
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
}

impl<'p> Vm<'p> {
  fn new(program: &'p Program, host: &'p FeelScope) -> Self {
    Self {
      program,
      host,
      slots: vec![Value::Null(None); program.slots as usize],
    }
  }

  /// The value of an operand; a temporary is left null.
  fn read(&mut self, opnd: Opnd) -> Value {
    match opnd.decode() {
      Operand::Slot(slot) => self.slots[slot as usize].clone(),
      Operand::Take(slot) => std::mem::replace(&mut self.slots[slot as usize], Value::Null(None)),
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
        Op::Return { src } => return self.read(src),
      }
    }
  }
}
