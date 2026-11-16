//! # Programs
//!
//! A program is the code of an expression for the run loop ([super::vm]): register instructions
//! over the numbered slots of a frame, and the tables they refer to. It is plain data, built once
//! by [ProgramBuilder] and checked by [Program::verify], and shared by every evaluation of the
//! expression.

pub(crate) use super::resolve::NONE;
use super::resolve::{Layout, Place, Region};
use crate::Dialect;
use crate::kernels::Arithmetic;
use rheon_feel::values::Value;
use rheon_feel::{IntervalType, Name};
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

  pub(crate) fn slot(slot: Slot) -> Self {
    Self::new(0, slot)
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
  /// The types `list<t>` and `range<t>`.
  ListType,
  RangeType,
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
  /// `a instance of b`.
  InstanceOf,
  /// An entry `name: type` of a context type.
  ContextTypeEntry,
  /// The type `function<t1, …> -> r`.
  FunctionType,
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
  /// The parameter types of a function type.
  ParameterTypes,
}

/// The kinds of the domains of iteration variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Domain {
  /// The numbers or dates from the first operand to the second (`a..b`).
  Interval,
  /// The numbers or dates of a range literal of a `for` (`[a..b]`), from its start and end.
  Range,
  /// The items of the value of the first operand, or the value itself when it is no list.
  Of,
}

/// The kinds of iterations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoopKind {
  /// `for`: the list of the values of the body.
  For,
  /// `some`: whether the body is `true` for some combination of the variables.
  Some,
  /// `every`: whether the body is `true` for every combination of the variables.
  Every,
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
  /// `dst` is the value of the qualified name of `searches[search]`, searched in the scopes of its
  /// position and then in the scope of the host, else null with an error.
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
  /// Opens a context type.
  TypeNew,
  /// Adds the value of `src`, an entry, to the innermost open context type; a value that is no
  /// entry closes it, `dst` is its null and the run continues at `fail`.
  TypeAdd { src: Opnd, dst: Slot, fail: Pc },
  /// `dst` is the innermost open context type, closed.
  TypeEnd { dst: Slot },
  /// Enters an invocation ([rheon_feel::diagnostics::enter_call]): beyond the call depth limit `dst` is null
  /// and the run continues at `skip`, past the matching [Op::Leave], so nothing of the
  /// invocation is evaluated.
  Enter { dst: Slot, skip: Pc },
  /// `dst` is the invocation `calls[call]`.
  Call { dst: Slot, call: Ix },
  /// Leaves the innermost invocation: its value in `dst` is made finite and nested no deeper
  /// than values may nest.
  Leave { dst: Slot },
  /// The value in the slot `s` is nested no deeper than values may nest.
  BoundDepth { s: Slot },
  /// Sets the slots of the first `visible` entries of `layouts[layout]` to null.
  Clear { layout: Ix, visible: u32 },
  /// `dst` is the context of the entries of `layouts[layout]`, whose values are moved out of their
  /// slots, nested no deeper than values may nest.
  Context { dst: Slot, layout: Ix },
  /// Opens the iteration `loops[info]`, whose walk is the record `record`: no variable has a
  /// value yet.
  LoopOpen { record: u32, info: Ix },
  /// Adds the domain of the next variable to the walk of `record`, from the values of `a` and `b`,
  /// and continues at the walk's step; a domain that fails ends the walk with its null.
  LevelPush { record: u32, domain: Domain, a: Opnd, b: Opnd },
  /// The next combination of the values of the variables of the walk of `record`: the next value
  /// of the innermost variable that has one, whose domain or body follows; the walk ends when no
  /// variable has one, or when the evaluation was stopped.
  Step { record: u32 },
  /// The value of `src` of the body for one combination of the walk of `record`: a `for` adds it
  /// to its results, a `some` or an `every` ends at a value that decides; then the next step.
  Visit { record: u32, src: Opnd },
  /// Closes the walk of `record`: `dst` is the value of the iteration.
  LoopClose { record: u32, dst: Slot },
  /// `dst` is the results so far of the `for` whose walk is `record` (`partial`).
  LoadPartial { dst: Slot, record: u32 },
  /// Opens the filter `filters[info]` of the value of `list`, whose items are the record
  /// `record`: a null is the filter's value at once; otherwise the predicate that follows is
  /// evaluated in the scope around the filter, tentatively.
  FilterOpen { record: u32, info: Ix, list: Opnd },
  /// The value of `c` of the predicate of the filter of `record`: a number evaluated around the
  /// filter selects an item; otherwise the predicate is evaluated for each item, which `c` keeps
  /// or not, until the items end or a value that is no boolean, number or null ends the filter.
  FilterStep { record: u32, c: Opnd },
  /// Closes the filter of `record`: `dst` of its info is the filter's value.
  FilterClose { record: u32 },
  /// While the filter of the record of `probes[probe]` visits its items: when the item is a
  /// context with the entry of its name, `dst` is its value and the run continues at `found`.
  ProbeItem { dst: Slot, probe: Ix, found: Pc },
  /// While the filter of `record` visits its items: `dst` is the item (its entry `item`, when it
  /// is a context that has one) and the run continues at `found`.
  ItemSelf { dst: Slot, record: u32, found: Pc },
  /// Ends the run with the value of `src`.
  Return { src: Opnd },
}

const _: () = assert!(std::mem::size_of::<Op>() <= 16);

/// An invocation: the function, the arguments in the temporaries `args .. args + argc`, in the
/// order written, for named arguments the index of their names in [Program::name_lists] ([NONE]
/// for positional arguments), and the innermost region at its position ([NONE] outside every
/// region), whose scopes a function of the host sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CallInfo {
  pub(crate) function: Opnd,
  pub(crate) args: Slot,
  pub(crate) argc: u32,
  pub(crate) names: Ix,
  pub(crate) region: u32,
}

/// An iteration: its kind, where its walk takes the next step, where it ends (at its
/// [Op::LoopClose]), and for each variable in order its slot and where the code that follows a
/// new value of it starts (the domain of the next variable, or the body).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopInfo {
  pub(crate) kind: LoopKind,
  pub(crate) step: Pc,
  pub(crate) exit: Pc,
  pub(crate) levels: Box<[(Slot, Pc)]>,
}

/// A name read first from the item of the filter whose items are the record `record`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProbeInfo {
  pub(crate) record: u32,
  pub(crate) name: Ix,
}

/// A filter: where its value goes, where its predicate starts, and where it closes (at its
/// [Op::FilterClose]) and ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FilterInfo {
  pub(crate) dst: Slot,
  pub(crate) body: Pc,
  pub(crate) close: Pc,
  pub(crate) after: Pc,
}

/// A qualified-name search: the segments in [Program::name_lists], the index of the first one in
/// [Program::names] ([NONE] without segments), and the innermost region at its position ([NONE]
/// outside every region).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SearchInfo {
  pub(crate) segments: Ix,
  pub(crate) first: u32,
  pub(crate) region: u32,
}

/// The code of an expression and its tables (vectors as they were built: a program is built once
/// per expression, and copying each table into an allocation of its exact size would cost more
/// than the space it saves).
pub(crate) struct Program {
  pub(crate) code: Vec<Op>,
  pub(crate) consts: Vec<Value>,
  /// The texts of the errors and warnings that instructions record.
  pub(crate) messages: Vec<Box<str>>,
  /// The names of the expression.
  pub(crate) names: Vec<Name>,
  /// The lists of names that instructions read: the segments of qualified names and the names
  /// of arguments.
  pub(crate) name_lists: Vec<Box<[Name]>>,
  /// The invocations.
  pub(crate) calls: Vec<CallInfo>,
  /// The qualified-name searches.
  pub(crate) searches: Vec<SearchInfo>,
  /// The iterations.
  pub(crate) loops: Vec<LoopInfo>,
  /// The filters.
  pub(crate) filters: Vec<FilterInfo>,
  /// The names read first from the items of filters.
  pub(crate) probes: Vec<ProbeInfo>,
  /// The binders of each binding construct, as name resolution placed them.
  pub(crate) layouts: Vec<Layout>,
  /// The scopes at the positions of the code, as name resolution found them.
  pub(crate) regions: Vec<Region>,
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
          check.index(search, self.searches.len(), "search out of range")?;
          let info = self.searches[search as usize];
          check.index(info.segments, self.name_lists.len(), "name list out of range")?;
          if info.first != NONE {
            check.index(info.first, self.names.len(), "name out of range")?;
          }
          check.region(info.region)?;
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
        Op::Jump { .. } | Op::ListNew { .. } | Op::TypeNew => {}
        Op::TypeAdd { src, dst, .. } => {
          check.slot(dst)?;
          check.opnd(src)?;
        }
        Op::TypeEnd { dst } => check.slot(dst)?,
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
          check.region(info.region)?;
        }
        Op::BoundDepth { s } => check.slot(s)?,
        Op::Clear { layout, visible } => {
          check.layout(layout)?;
          if visible as usize > self.layouts[layout as usize].entries.len() {
            return check.fail("more entries than the layout has");
          }
        }
        Op::Context { dst, layout } => {
          check.slot(dst)?;
          check.layout(layout)?;
        }
        Op::LoopOpen { info, .. } => {
          check.index(info, self.loops.len(), "iteration out of range")?;
          for &(slot, _) in self.loops[info as usize].levels.iter() {
            check.slot(slot)?;
          }
        }
        Op::LevelPush { a, b, .. } => {
          check.opnd(a)?;
          check.opnd(b)?;
        }
        Op::Visit { src, .. } => check.opnd(src)?,
        Op::Step { .. } => {}
        Op::LoopClose { dst, .. } | Op::LoadPartial { dst, .. } | Op::ItemSelf { dst, .. } => check.slot(dst)?,
        Op::FilterOpen { info, list, .. } => {
          check.index(info, self.filters.len(), "filter out of range")?;
          check.slot(self.filters[info as usize].dst)?;
          check.opnd(list)?;
        }
        Op::FilterStep { c, .. } => check.opnd(c)?,
        Op::FilterClose { .. } => {}
        Op::ProbeItem { dst, probe, .. } => {
          check.slot(dst)?;
          check.index(probe, self.probes.len(), "probe out of range")?;
          check.index(self.probes[probe as usize].name, self.names.len(), "name out of range")?;
        }
        Op::Return { src } => check.opnd(src)?,
      }
    }
    for (at, region) in self.regions.iter().enumerate() {
      let check = Check { program: self, at: at as Pc };
      if region.parent != NONE && region.parent as usize >= at {
        return check.fail("a region is not held by an earlier one");
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
  /// Checks that every jump goes forward to an instruction of the code, and that lists,
  /// invocations, context types and walks are opened and closed in nesting order on every path:
  /// the code is structured, so what is open at an instruction is what is open there in the
  /// order of the code, the same from every jump. An [Op::Enter] that refuses continues past its
  /// [Op::Leave], with what was open before it. A walk is the record of its nesting depth, and the
  /// only jumps back are those of a walk to its step, inside it.
  fn verify_structure(&self) -> Result<(), VerifyError> {
    // the open lists, invocations, context types and walks before each instruction
    let mut open = Vec::with_capacity(self.code.len() + 1);
    let (mut lists, mut calls, mut types) = (0_u32, 0_u32, 0_u32);
    // the open walks and filters: whether a walk, the index of its info, where it opens
    let mut records: Vec<(bool, u32, Pc)> = vec![];
    for (at, op) in self.code.iter().enumerate() {
      open.push((lists, calls, types, records.len() as u32));
      let fail = |what| Err(VerifyError { at: at as Pc, what });
      match *op {
        Op::LoopOpen { record, info } | Op::FilterOpen { record, info, .. } => {
          let walk = matches!(op, Op::LoopOpen { .. });
          if info as usize >= if walk { self.loops.len() } else { self.filters.len() } {
            return fail("iteration or filter out of range");
          }
          if record as usize != records.len() {
            return fail("a walk or a filter that is not the record of its depth");
          }
          records.push((walk, info, at as Pc));
        }
        Op::LevelPush { record, .. } | Op::Step { record } | Op::Visit { record, .. } | Op::LoadPartial { record, .. }
          if records.get(record as usize).is_none_or(|record| !record.0) =>
        {
          return fail("no walk of the record is open");
        }
        Op::FilterStep { record, .. } | Op::ItemSelf { record, .. } if records.get(record as usize).is_none_or(|record| record.0) => {
          return fail("no filter of the record is open");
        }
        Op::ProbeItem { probe, .. }
          if self
            .probes
            .get(probe as usize)
            .is_none_or(|probe| records.get(probe.record as usize).is_none_or(|record| record.0)) =>
        {
          return fail("no filter of the record is open");
        }
        Op::LoopClose { record, .. } | Op::FilterClose { record } => {
          let walk = matches!(op, Op::LoopClose { .. });
          if records.len() != record as usize + 1 || records[record as usize].0 != walk {
            return fail("a walk or a filter closed out of order");
          }
          let (_, info, opened) = records.pop().unwrap_or_default();
          let inside = |pc: Pc| pc > opened && pc <= at as Pc;
          let inside = if walk {
            let info = &self.loops[info as usize];
            info.exit == at as Pc && inside(info.step) && info.levels.iter().all(|&(_, bind)| inside(bind))
          } else {
            let info = &self.filters[info as usize];
            info.close == at as Pc && info.after == at as Pc + 1 && info.body == opened + 1
          };
          if !inside {
            return fail("a walk or a filter whose steps are not inside it");
          }
        }
        Op::ListNew { .. } => lists += 1,
        Op::ListPush { .. } if lists == 0 => return fail("no list is open"),
        Op::ListEnd { .. } if lists == 0 => return fail("no list is open"),
        Op::ListEnd { .. } => lists -= 1,
        Op::Enter { .. } => calls += 1,
        Op::Leave { .. } if calls == 0 => return fail("no invocation is open"),
        Op::Leave { .. } => calls -= 1,
        Op::TypeNew => types += 1,
        Op::TypeAdd { .. } if types == 0 => return fail("no context type is open"),
        Op::TypeEnd { .. } if types == 0 => return fail("no context type is open"),
        Op::TypeEnd { .. } => types -= 1,
        Op::Return { .. } if lists != 0 || calls != 0 || types != 0 || !records.is_empty() => return fail("a list, an invocation, a context type or a walk is open at the return"),
        _ => {}
      }
    }
    if !records.is_empty() {
      return Err(VerifyError {
        at: self.code.len() as Pc,
        what: "a walk is not closed",
      });
    }
    // the targets of the walks and filters: what is open there is what is open inside them
    for (at, op) in self.code.iter().enumerate() {
      let targets: Vec<Pc> = match *op {
        Op::LoopOpen { info, .. } => {
          let info = &self.loops[info as usize];
          std::iter::once(info.step)
            .chain(info.levels.iter().map(|&(_, bind)| bind))
            .chain(std::iter::once(info.exit))
            .collect()
        }
        Op::FilterOpen { info, .. } => vec![self.filters[info as usize].close],
        _ => continue,
      };
      if targets.iter().any(|&target| open.get(target as usize) != Some(&open[at + 1])) {
        return Err(VerifyError {
          at: at as Pc,
          what: "a walk or a filter that jumps across a list, an invocation or a context type",
        });
      }
    }
    for (at, op) in self.code.iter().enumerate() {
      let (target, state) = match *op {
        Op::Jump { to } | Op::JumpUnlessTrue { to, .. } | Op::AndLeft { to, .. } | Op::OrLeft { to, .. } => (to, open.get(at + 1).copied()),
        Op::Enter { skip, .. } => (skip, Some(open[at])),
        Op::ProbeItem { found, .. } | Op::ItemSelf { found, .. } => (found, open.get(at + 1).copied()),
        // a null list skips the filter
        Op::FilterOpen { info, .. } => (self.filters[info as usize].after, Some(open[at])),
        // a failure closes the context type
        Op::TypeAdd { fail, .. } => (fail, Some((open[at].0, open[at].1, open[at].2 - 1, open[at].3))),
        _ => continue,
      };
      let fail = |what| Err(VerifyError { at: at as Pc, what });
      if target as usize <= at || target as usize >= self.code.len() {
        return fail("jump out of range");
      }
      if Some(open[target as usize]) != state {
        return fail("jump across a list, an invocation or a context type");
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

  /// A region, or [NONE].
  fn region(&self, region: u32) -> Result<(), VerifyError> {
    if region == NONE {
      Ok(())
    } else {
      self.index(region, self.program.regions.len(), "region out of range")
    }
  }

  /// A layout whose binders are all in slots of the frame and whose names are names of the
  /// program.
  fn layout(&self, layout: Ix) -> Result<(), VerifyError> {
    self.index(layout, self.program.layouts.len(), "layout out of range")?;
    for &(name, place) in self.program.layouts[layout as usize].entries.iter() {
      self.index(name, self.program.names.len(), "name out of range")?;
      match place {
        Place::Slot(slot) => self.slot(slot)?,
        Place::Partial(_) => return self.fail("a binder that is no slot"),
      }
    }
    Ok(())
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
  name_lists: Vec<Box<[Name]>>,
  calls: Vec<CallInfo>,
  searches: Vec<SearchInfo>,
  loops: Vec<LoopInfo>,
  filters: Vec<FilterInfo>,
  probes: Vec<ProbeInfo>,
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
      Op::Jump { to }
      | Op::JumpUnlessTrue { to, .. }
      | Op::AndLeft { to, .. }
      | Op::OrLeft { to, .. }
      | Op::Enter { skip: to, .. }
      | Op::TypeAdd { fail: to, .. }
      | Op::ProbeItem { found: to, .. }
      | Op::ItemSelf { found: to, .. } => *to = here,
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

  /// A builder for the program of an expression of `nodes` nodes: about one instruction per node.
  pub(crate) fn with_capacity(nodes: usize) -> Self {
    Self {
      code: Vec::with_capacity(nodes + 1),
      ..Self::default()
    }
  }

  /// The names of the expression, which instructions refer to by their index.
  pub(crate) fn names(&mut self, names: Vec<Name>) {
    self.names = names;
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

  /// The index of the iteration `info` in the program.
  pub(crate) fn iteration(&mut self, info: LoopInfo) -> Ix {
    self.loops.push(info);
    self.loops.len() as Ix - 1
  }

  /// The iteration `info`, as built so far.
  pub(crate) fn iteration_mut(&mut self, info: Ix) -> &mut LoopInfo {
    &mut self.loops[info as usize]
  }

  /// The index of the filter `info` in the program.
  pub(crate) fn filter(&mut self, info: FilterInfo) -> Ix {
    self.filters.push(info);
    self.filters.len() as Ix - 1
  }

  /// The index of the probe `probe` in the program.
  pub(crate) fn probe(&mut self, probe: ProbeInfo) -> Ix {
    self.probes.push(probe);
    self.probes.len() as Ix - 1
  }

  /// The filter `info`, as built so far.
  pub(crate) fn filter_mut(&mut self, info: Ix) -> &mut FilterInfo {
    &mut self.filters[info as usize]
  }

  /// The position of the next instruction.
  pub(crate) fn here(&self) -> Pc {
    self.code.len() as Pc
  }

  /// The index of the search `search` in the program.
  pub(crate) fn search(&mut self, search: SearchInfo) -> Ix {
    self.searches.push(search);
    self.searches.len() as Ix - 1
  }

  /// The program in `dialect`, with a frame of `slots` slots and the layouts and regions of its
  /// name resolution; checked in builds with debug assertions.
  pub(crate) fn finish(self, slots: u32, dialect: Dialect, layouts: Vec<Layout>, regions: Vec<Region>) -> Arc<Program> {
    let program = Program {
      code: self.code,
      consts: self.consts,
      messages: self.messages,
      names: self.names,
      name_lists: self.name_lists,
      calls: self.calls,
      searches: self.searches,
      loops: self.loops,
      filters: self.filters,
      probes: self.probes,
      layouts,
      regions,
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
