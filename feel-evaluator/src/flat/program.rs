//! # Programs
//!
//! A program is the code of an expression for the run loop ([super::vm]): register instructions
//! over the numbered slots of a frame, and the tables they refer to. It is plain data, built once
//! by [ProgramBuilder] and checked by [Program::verify], and shared by every evaluation of the
//! expression.

use rheon_feel::values::Value;
use std::sync::Arc;

/// A slot of a frame.
pub(crate) type Slot = u32;
/// A position in the code.
pub(crate) type Pc = u32;
/// An index into a table of the program.
pub(crate) type Ix = u32;

/// An operand of an instruction: a slot read by a copy, a slot whose value is moved out (a
/// temporary, which has one reader), or a constant. The kind is in the top two bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Opnd(u32);

/// The kinds of operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Operand {
  /// The value of a slot, copied.
  Slot(Slot),
  /// The value of a temporary, moved out: the slot is null after the read.
  Take(Slot),
  /// A constant of the program.
  Const(Ix),
}

impl Opnd {
  const SHIFT: u32 = 30;
  const INDEX: u32 = (1 << Self::SHIFT) - 1;

  fn new(kind: u32, index: u32) -> Self {
    // an index of 2^30 or more would need more memory than an evaluation may use
    assert!(index <= Self::INDEX, "operand index {index} out of range");
    Self(kind << Self::SHIFT | index)
  }

  pub(crate) fn take(slot: Slot) -> Self {
    Self::new(1, slot)
  }

  pub(crate) fn konst(index: Ix) -> Self {
    Self::new(2, index)
  }

  pub(crate) fn decode(self) -> Operand {
    let index = self.0 & Self::INDEX;
    match self.0 >> Self::SHIFT {
      0 => Operand::Slot(index),
      1 => Operand::Take(index),
      _ => Operand::Const(index),
    }
  }
}

/// An instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Op {
  /// `dst` is null with the error `messages[msg]`, recorded at every evaluation.
  Fail { dst: Slot, msg: Ix },
  /// Records the warning `messages[msg]`.
  Warn { msg: Ix },
  /// Ends the run with the value of `src`.
  Return { src: Opnd },
}

const _: () = assert!(std::mem::size_of::<Op>() <= 16);

/// The code of an expression and its tables.
pub(crate) struct Program {
  pub(crate) code: Box<[Op]>,
  pub(crate) consts: Box<[Value]>,
  /// The texts of the errors and warnings that instructions record.
  pub(crate) messages: Box<[Box<str>]>,
  /// The slots of the frame.
  pub(crate) slots: u32,
}

/// A program that [Program::verify] rejects: what is wrong, at which instruction.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct VerifyError {
  pub(crate) at: Pc,
  pub(crate) what: &'static str,
}

impl Program {
  /// Checks that every operand, slot and index of every instruction is in range and that the
  /// code ends with a return, so that the run loop never indexes out of range.
  pub(crate) fn verify(&self) -> Result<(), VerifyError> {
    let at = |at: usize, what| VerifyError { at: at as Pc, what };
    let slot = |at_: usize, slot: Slot| if slot < self.slots { Ok(()) } else { Err(at(at_, "slot out of range")) };
    let opnd = |at_: usize, opnd: Opnd| match opnd.decode() {
      Operand::Slot(index) | Operand::Take(index) => slot(at_, index),
      Operand::Const(index) if (index as usize) < self.consts.len() => Ok(()),
      Operand::Const(_) => Err(at(at_, "constant out of range")),
    };
    let message = |at_: usize, msg: Ix| {
      if (msg as usize) < self.messages.len() {
        Ok(())
      } else {
        Err(at(at_, "message out of range"))
      }
    };
    for (index, op) in self.code.iter().enumerate() {
      match *op {
        Op::Fail { dst, msg } => {
          slot(index, dst)?;
          message(index, msg)?;
        }
        Op::Warn { msg } => message(index, msg)?,
        Op::Return { src } => opnd(index, src)?,
      }
    }
    match self.code.last() {
      Some(Op::Return { .. }) => Ok(()),
      _ => Err(at(self.code.len(), "the code does not end with a return")),
    }
  }
}

/// Builds a [Program].
#[derive(Default)]
pub(crate) struct ProgramBuilder {
  code: Vec<Op>,
  consts: Vec<Value>,
  messages: Vec<Box<str>>,
}

impl ProgramBuilder {
  pub(crate) fn emit(&mut self, op: Op) -> Pc {
    self.code.push(op);
    self.code.len() as Pc - 1
  }

  pub(crate) fn konst(&mut self, value: Value) -> Opnd {
    self.consts.push(value);
    Opnd::konst(self.consts.len() as Ix - 1)
  }

  pub(crate) fn message(&mut self, text: impl Into<Box<str>>) -> Ix {
    self.messages.push(text.into());
    self.messages.len() as Ix - 1
  }

  /// The program, with a frame of `slots` slots; checked in builds with debug assertions.
  pub(crate) fn finish(self, slots: u32) -> Arc<Program> {
    let program = Program {
      code: self.code.into(),
      consts: self.consts.into(),
      messages: self.messages.into(),
      slots,
    };
    #[cfg(debug_assertions)]
    if let Err(error) = program.verify() {
      panic!("invalid program: {error:?}\n{:?}", program.code);
    }
    Arc::new(program)
  }
}
