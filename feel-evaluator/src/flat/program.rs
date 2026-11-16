//! # Programs
//!
//! A program is the code of an expression for the run loop ([super::vm]): register instructions
//! over the numbered slots of a frame, and the tables they refer to. It is plain data, built once
//! by [ProgramBuilder] and checked by [Program::verify], and shared by every evaluation of the
//! expression.

use crate::Dialect;
use crate::kernels::Arithmetic;
use rheon_feel::values::Value;
use rheon_feel::{IntervalType, Name};
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

/// The kernels of one operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kernel1 {
  /// The unary tests `= e`, `!= e`, `< e`, `<= e`, `> e`, `>= e`.
  UnaryEq,
  UnaryNe,
  UnaryLt,
  UnaryLe,
  UnaryGt,
  UnaryGe,
  /// The start and the end of a range literal.
  IntervalStart(IntervalType),
  IntervalEnd(IntervalType),
}

/// The kernels of two operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kernel2 {
  /// `a in b`.
  In,
  /// `a not in b`: `a` when the first operand (the value of `a in b`) is `true`, else null.
  Out,
  /// The range of a range literal from its start and end.
  Range,
}

/// The kinds of lists that [Op::ListEnd] makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ListKind {
  /// A list literal, nested no deeper than values may nest.
  List,
  /// The unary tests of a comma list.
  ExpressionList,
  /// The unary tests of a negated comma list, `not(...)`.
  NegatedList,
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
  /// `dst` is the value of the qualified name `name_lists[search]` in the scope of the host, else
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
  /// `dst` is the kernel `kernel` on the value of `a`.
  Kernel1 { kernel: Kernel1, dst: Slot, a: Opnd },
  /// `dst` is the kernel `kernel` on the values of `a` and `b`.
  Kernel2 { kernel: Kernel2, dst: Slot, a: Opnd, b: Opnd },
  /// Continues at `to`.
  Jump { to: Pc },
  /// Continues at `to` unless the value of `c` is `true`.
  JumpUnlessTrue { c: Opnd, to: Pc },
  /// The left operand of `and`: when its value is `false`, `dst` is `false` and the run continues
  /// at `to`; otherwise `dst` holds it.
  AndLeft { dst: Slot, a: Opnd, to: Pc },
  /// The right operand of `and`: `dst` is the conjunction of the left operand in `dst` and `b`.
  AndRight { dst: Slot, b: Opnd },
  /// The left operand of `or`: when its value is `true`, `dst` is `true` and the run continues at
  /// `to`; otherwise `dst` holds it.
  OrLeft { dst: Slot, a: Opnd, to: Pc },
  /// The right operand of `or`: `dst` is the disjunction of the left operand in `dst` and `b`.
  OrRight { dst: Slot, b: Opnd },
  /// Opens a list of `len` items.
  ListNew { len: u32 },
  /// Adds the value of `src` to the innermost open list.
  ListPush { src: Opnd },
  /// `dst` is the innermost open list, closed, as a value of `kind`.
  ListEnd { kind: ListKind, dst: Slot },
  /// Enters an invocation ([rheon_feel::diagnostics::enter_call]): beyond the call depth limit `dst` is null
  /// and the run continues at `skip`, past the matching [Op::Leave], so nothing of the
  /// invocation is evaluated.
  Enter { dst: Slot, skip: Pc },
  /// `dst` is the invocation `calls[call]`.
  Call { dst: Slot, call: Ix },
  /// Leaves the innermost invocation: its value in `dst` is made finite and nested no deeper
  /// than values may nest.
  Leave { dst: Slot },
  /// Ends the run with the value of `src`.
  Return { src: Opnd },
}

const _: () = assert!(std::mem::size_of::<Op>() <= 16);

/// An invocation: the function, the arguments in the temporaries `args .. args + argc`, in the
/// order written, and for named arguments the index of their names in [Program::name_lists]
/// ([NONE] for positional arguments).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CallInfo {
  pub(crate) function: Opnd,
  pub(crate) args: Slot,
  pub(crate) argc: u32,
  pub(crate) names: Ix,
}

/// No index: positional arguments.
pub(crate) const NONE: Ix = Ix::MAX;

/// The code of an expression and its tables.
pub(crate) struct Program {
  pub(crate) code: Box<[Op]>,
  pub(crate) consts: Box<[Value]>,
  /// The texts of the errors and warnings that instructions record.
  pub(crate) messages: Box<[Box<str>]>,
  /// The names that instructions read.
  pub(crate) names: Box<[Name]>,
  /// The lists of names that instructions read: the segments of qualified names and the names
  /// of arguments.
  pub(crate) name_lists: Box<[Box<[Name]>]>,
  /// The invocations.
  pub(crate) calls: Box<[CallInfo]>,
  /// FEEL or B-FEEL.
  pub(crate) dialect: Dialect,
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
    self.verify_structure()?;
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
          check.index(search, self.name_lists.len(), "name list out of range")?;
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
        Op::BoolDefault { dst, .. } | Op::ListEnd { dst, .. } => check.slot(dst)?,
        Op::Kernel1 { dst, a, .. } | Op::AndLeft { dst, a, .. } | Op::OrLeft { dst, a, .. } | Op::AndRight { dst, b: a } | Op::OrRight { dst, b: a } => {
          check.slot(dst)?;
          check.opnd(a)?;
        }
        Op::Kernel2 { dst, a, b, .. } => {
          check.slot(dst)?;
          check.opnd(a)?;
          check.opnd(b)?;
        }
        Op::JumpUnlessTrue { c, .. } => check.opnd(c)?,
        Op::Jump { .. } | Op::ListNew { .. } => {}
        Op::ListPush { src } => check.opnd(src)?,
        Op::Enter { dst, .. } | Op::Leave { dst } => check.slot(dst)?,
        Op::Call { dst, call } => {
          check.slot(dst)?;
          check.index(call, self.calls.len(), "invocation out of range")?;
          let info = self.calls[call as usize];
          check.opnd(info.function)?;
          if info.argc > 0 {
            check.slot(info.args)?;
            check.slot(info.args + (info.argc - 1))?;
          }
          if info.names != NONE {
            check.index(info.names, self.name_lists.len(), "name list out of range")?;
            if self.name_lists[info.names as usize].len() != info.argc as usize {
              return check.fail("names and arguments differ in number");
            }
          }
        }
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

impl Program {
  /// Checks that every jump goes forward to an instruction of the code, and that lists and
  /// invocations are opened and closed in nesting order on every path: the code is structured,
  /// so what is open at an instruction is what is open there in the order of the code, the same
  /// from every jump. An [Op::Enter] that refuses continues past its [Op::Leave], with what was
  /// open before it.
  fn verify_structure(&self) -> Result<(), VerifyError> {
    // the open lists and the open invocations before each instruction
    let mut open = Vec::with_capacity(self.code.len() + 1);
    let (mut lists, mut calls) = (0_u32, 0_u32);
    for (at, op) in self.code.iter().enumerate() {
      open.push((lists, calls));
      let fail = |what| Err(VerifyError { at: at as Pc, what });
      match *op {
        Op::ListNew { .. } => lists += 1,
        Op::ListPush { .. } if lists == 0 => return fail("no list is open"),
        Op::ListEnd { .. } if lists == 0 => return fail("no list is open"),
        Op::ListEnd { .. } => lists -= 1,
        Op::Enter { .. } => calls += 1,
        Op::Leave { .. } if calls == 0 => return fail("no invocation is open"),
        Op::Leave { .. } => calls -= 1,
        Op::Return { .. } if lists != 0 || calls != 0 => return fail("a list or an invocation is open at the return"),
        _ => {}
      }
    }
    for (at, op) in self.code.iter().enumerate() {
      let (target, state) = match *op {
        Op::Jump { to } | Op::JumpUnlessTrue { to, .. } | Op::AndLeft { to, .. } | Op::OrLeft { to, .. } => (to, open.get(at + 1).copied()),
        Op::Enter { skip, .. } => (skip, Some(open[at])),
        _ => continue,
      };
      let fail = |what| Err(VerifyError { at: at as Pc, what });
      if target as usize <= at || target as usize >= self.code.len() {
        return fail("jump out of range");
      }
      if Some(open[target as usize]) != state {
        return fail("jump across a list or an invocation");
      }
      if matches!(op, Op::Enter { .. }) && !matches!(self.code[target as usize - 1], Op::Leave { .. }) {
        return fail("an invocation that refuses does not continue past a leave");
      }
    }
    Ok(())
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
  name_lists: Vec<Box<[Name]>>,
  calls: Vec<CallInfo>,
}

impl ProgramBuilder {
  pub(crate) fn emit(&mut self, op: Op) -> Pc {
    self.code.push(op);
    self.code.len() as Pc - 1
  }

  /// Points the jump at `at` to the next instruction.
  pub(crate) fn patch(&mut self, at: Pc) {
    let here = self.code.len() as Pc;
    match &mut self.code[at as usize] {
      Op::Jump { to } | Op::JumpUnlessTrue { to, .. } | Op::AndLeft { to, .. } | Op::OrLeft { to, .. } | Op::Enter { skip: to, .. } => *to = here,
      _ => {}
    }
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

  /// The index of the list of names `names` in the program.
  pub(crate) fn name_list(&mut self, names: Box<[Name]>) -> Ix {
    self.name_lists.push(names);
    self.name_lists.len() as Ix - 1
  }

  /// The index of the invocation `call` in the program.
  pub(crate) fn call(&mut self, call: CallInfo) -> Ix {
    self.calls.push(call);
    self.calls.len() as Ix - 1
  }

  /// The program in `dialect`, with a frame of `slots` slots; checked in builds with debug
  /// assertions.
  pub(crate) fn finish(self, slots: u32, dialect: Dialect) -> Arc<Program> {
    let program = Program {
      code: self.code.into(),
      consts: self.consts.into(),
      messages: self.messages.into(),
      names: self.names.into(),
      name_lists: self.name_lists.into(),
      calls: self.calls.into(),
      slots,
      dialect,
    };
    #[cfg(debug_assertions)]
    if let Err(error) = program.verify() {
      panic!("invalid program: {error:?}\n{:?}", program.code);
    }
    Arc::new(program)
  }
}
