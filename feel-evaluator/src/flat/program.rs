//! # Programs
//!
//! A program is the code of an expression for the run loop ([super::vm]): register instructions
//! over the numbered slots of a frame, and the tables they refer to. It is plain data, built once
//! by [ProgramBuilder] and checked by [Program::verify], and shared by every evaluation of the
//! expression.

use crate::kernels::Arithmetic;
use rheon_feel::Name;
use rheon_feel::values::Value;
use std::collections::HashMap;
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

/// The comparisons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Compare {
  Eq,
  Ne,
  Lt,
  Le,
  Gt,
  Ge,
}

/// An instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Op {
  /// `dst` is the value of `src`.
  Move { dst: Slot, src: Opnd },
  /// `dst` is null with the error `messages[msg]`, recorded at every evaluation.
  Fail { dst: Slot, msg: Ix },
  /// Records the warning `messages[msg]`.
  Warn { msg: Ix },
  /// `dst` is the value of the free name `names[name]` in the scope of the host, else the
  /// built-in function of that name, else null with an error.
  LoadName { dst: Slot, name: Ix },
  /// `dst` is the value of the qualified name `searches[search]` in the scope of the host, else
  /// null with an error.
  Search { dst: Slot, search: Ix },
  /// `dst` is the entry `names[name]` of the value of `a` (`a.name`).
  Path { dst: Slot, a: Opnd, name: Ix },
  /// `dst` is the values of the entries `names[name]` in the value of `a` and in every context
  /// in it (`a...name`).
  Descendant { dst: Slot, a: Opnd, name: Ix },
  /// `dst` is the FEEL arithmetic `op` on the values of `a` and `b`.
  Arith { op: Arithmetic, dst: Slot, a: Opnd, b: Opnd },
  /// `dst` is the B-FEEL arithmetic `op` on the values of `a` and `b`.
  ArithB { op: Arithmetic, dst: Slot, a: Opnd, b: Opnd },
  /// `dst` is the negation of the value of `a`.
  Neg { dst: Slot, a: Opnd },
  /// `dst` is the comparison `op` of the values of `a` and `b`.
  Compare { op: Compare, dst: Slot, a: Opnd, b: Opnd },
  /// `dst` is whether the value of the temporary `args` is between those of `args + 1` and
  /// `args + 2`.
  Between { dst: Slot, args: Slot },
  /// B-FEEL: a null in `dst` becomes `ne` (`false` for `=` and the others, `true` for `!=`).
  BoolDefault { dst: Slot, ne: bool },
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
  /// The names that instructions read.
  pub(crate) names: Box<[Name]>,
  /// The segments of the qualified names that instructions search.
  pub(crate) searches: Box<[Box<[Name]>]>,
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
    for (at, op) in self.code.iter().enumerate() {
      let check = Check { program: self, at: at as Pc };
      match *op {
        Op::Fail { dst, msg } => {
          check.slot(dst)?;
          check.index(msg, self.messages.len(), "message out of range")?;
        }
        Op::Warn { msg } => check.index(msg, self.messages.len(), "message out of range")?,
        Op::LoadName { dst, name } => {
          check.slot(dst)?;
          check.index(name, self.names.len(), "name out of range")?;
        }
        Op::Search { dst, search } => {
          check.slot(dst)?;
          check.index(search, self.searches.len(), "search out of range")?;
        }
        Op::Path { dst, a, name } | Op::Descendant { dst, a, name } => {
          check.slot(dst)?;
          check.opnd(a)?;
          check.index(name, self.names.len(), "name out of range")?;
        }
        Op::Move { dst, src: a } | Op::Neg { dst, a } => {
          check.slot(dst)?;
          check.opnd(a)?;
        }
        Op::Arith { dst, a, b, .. } | Op::ArithB { dst, a, b, .. } | Op::Compare { dst, a, b, .. } => {
          check.slot(dst)?;
          check.opnd(a)?;
          check.opnd(b)?;
        }
        Op::Between { dst, args } => {
          check.slot(dst)?;
          check.slot(args)?;
          check.slot(args + 2)?;
        }
        Op::BoolDefault { dst, .. } => check.slot(dst)?,
        Op::Return { src } => check.opnd(src)?,
      }
    }
    match self.code.last() {
      Some(Op::Return { .. }) => Ok(()),
      _ => Err(VerifyError {
        at: self.code.len() as Pc,
        what: "the code does not end with a return",
      }),
    }
  }
}

/// The checks of one instruction of a program.
struct Check<'p> {
  program: &'p Program,
  at: Pc,
}

impl Check<'_> {
  fn fail(&self, what: &'static str) -> Result<(), VerifyError> {
    Err(VerifyError { at: self.at, what })
  }

  fn index(&self, index: Ix, len: usize, what: &'static str) -> Result<(), VerifyError> {
    if (index as usize) < len { Ok(()) } else { self.fail(what) }
  }

  fn slot(&self, slot: Slot) -> Result<(), VerifyError> {
    if slot < self.program.slots { Ok(()) } else { self.fail("slot out of range") }
  }

  fn opnd(&self, opnd: Opnd) -> Result<(), VerifyError> {
    match opnd.decode() {
      Operand::Slot(slot) | Operand::Take(slot) => self.slot(slot),
      Operand::Const(index) => self.index(index, self.program.consts.len(), "constant out of range"),
    }
  }
}

/// Builds a [Program].
#[derive(Default)]
pub(crate) struct ProgramBuilder {
  code: Vec<Op>,
  consts: Vec<Value>,
  messages: Vec<Box<str>>,
  names: Vec<Name>,
  named: HashMap<Name, Ix>,
  searches: Vec<Box<[Name]>>,
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

  /// The index of `name` in the names of the program.
  pub(crate) fn name(&mut self, name: &Name) -> Ix {
    if let Some(&index) = self.named.get(name) {
      return index;
    }
    self.names.push(name.clone());
    let index = self.names.len() as Ix - 1;
    self.named.insert(name.clone(), index);
    index
  }

  /// The index of the qualified name of `segments` in the searches of the program.
  pub(crate) fn search(&mut self, segments: Box<[Name]>) -> Ix {
    self.searches.push(segments);
    self.searches.len() as Ix - 1
  }

  /// The program, with a frame of `slots` slots; checked in builds with debug assertions.
  pub(crate) fn finish(self, slots: u32) -> Arc<Program> {
    let program = Program {
      code: self.code.into(),
      consts: self.consts.into(),
      messages: self.messages.into(),
      names: self.names.into(),
      searches: self.searches.into(),
      slots,
    };
    #[cfg(debug_assertions)]
    if let Err(error) = program.verify() {
      panic!("invalid program: {error:?}\n{:?}", program.code);
    }
    Arc::new(program)
  }
}
