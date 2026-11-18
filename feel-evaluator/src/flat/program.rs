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
use rheon_feel::closure::Closure;
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
  /// The body of an external function from its mapping.
  ExternalBody,
  /// The value of a unary test that uses `?`: when it is a boolean, the answer of the test; when
  /// it is null and the test is a boolean expression (`true` here), the null answer; otherwise
  /// the value, which the tested value is compared with.
  TestAnswer(bool),
}

/// The kernels of two operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kernel2 {
  /// `a in b`.
  In,
  /// `a in b` in B-FEEL, where a comparison is true or false: a test that cannot be compared with
  /// the tested value is not satisfied.
  InTwoValued,
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
  /// A formal parameter of a function definition from its name and its type.
  FormalParameter,
}

/// What [Op::FoldNew] opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fold {
  /// A context type, from its entries.
  ContextType,
  /// The formal parameters of a function definition.
  Parameters,
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
  /// `dst` is the value of the free name `names[name]` below the scopes of the body (in the
  /// scope of the host for the root expression), else the built-in function of that name, else
  /// null with an error; in a body, the value is kept in the activation's cell `cell` once read.
  LoadName { dst: Slot, name: Ix, cell: u32 },
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
  /// Opens a fold of `kind`: a context type or formal parameters.
  FoldNew { kind: Fold },
  /// Adds the value of `src`, an entry of a context type or a formal parameter, to the innermost
  /// open fold; a value that is none closes it, `dst` is its null and the run continues at `fail`.
  FoldAdd { src: Opnd, dst: Slot, fail: Pc },
  /// `dst` is the innermost open fold, closed.
  FoldEnd { dst: Slot },
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
  /// Starts the home of the context literal of `layouts[layout]`: what the functions written
  /// in the literal see of its entries.
  HomeOpen { layout: Ix },
  /// The entry `entry` of the innermost home has the value in the slot `s`.
  HomeSet { entry: u32, s: Slot },
  /// Ends the innermost home.
  HomeClose,
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
  /// `dst` is the function of the definition `closures[closure]`.
  Closure { dst: Slot, closure: Ix },
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

/// A function definition: the proto of its body, the value of its formal parameters, the value
/// of its external body (`None` for a body of FEEL), and the innermost region at its position
/// ([NONE] outside every region), where the names it closes over are searched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClosureInfo {
  pub(crate) proto: u32,
  pub(crate) parameters: Opnd,
  pub(crate) body: Option<Opnd>,
  pub(crate) region: u32,
}

/// The body of a function definition: where its code is, the slots of its frame and the cells of
/// its free names, the slot of
/// each parameter in their order, its outermost region, the names it closes over and, for each
/// of them in their order, the index of its first segment in [Program::names] ([NONE] for a name
/// that no binder of the program has).
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Proto {
  pub(crate) entry: Pc,
  pub(crate) end: Pc,
  pub(crate) slots: u32,
  /// The name and the cell of each free name it reads, by name.
  pub(crate) cells: Box<[(u32, u32)]>,
  pub(crate) params: Box<[Slot]>,
  pub(crate) region: u32,
  pub(crate) closure: Closure,
  pub(crate) captures: Box<[u32]>,
}

/// A qualified-name search: the segments in [Program::name_lists], the index of the first one in
/// [Program::names] ([NONE] without segments), and the innermost region at its position ([NONE]
/// outside every region, and for a name that no scope of the body has).
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
  /// The function definitions.
  pub(crate) closures: Vec<ClosureInfo>,
  /// The bodies of the function definitions, whose code follows that of the root expression.
  pub(crate) protos: Vec<Proto>,
  /// The binders of each binding construct, as name resolution placed them.
  pub(crate) layouts: Vec<Layout>,
  /// The scopes at the positions of the code, as name resolution found them.
  pub(crate) regions: Vec<Region>,
  /// FEEL or B-FEEL.
  pub(crate) dialect: Dialect,
  /// The slots of the frame of the root expression.
  pub(crate) slots: u32,
  /// The cells of the root expression: one for each free name it reads.
  pub(crate) root_cells: u32,
  /// The end of the code of the root expression.
  pub(crate) root_end: Pc,
}

/// A program that [Program::verify] rejects: what is wrong, at which instruction.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct VerifyError {
  pub(crate) at: Pc,
  pub(crate) what: &'static str,
}

impl Program {
  /// Checks that the code of the root expression and of every body follow each other, that every
  /// operand, slot and index of every instruction is in range for its body and that every body's
  /// code ends with a return, so that the run loop never indexes out of range.
  pub(crate) fn verify(&self) -> Result<(), VerifyError> {
    let mut bodies: Vec<(Pc, Pc, u32, u32)> = std::iter::once((0, self.root_end, self.slots, self.root_cells))
      .chain(self.protos.iter().map(|proto| (proto.entry, proto.end, proto.slots, proto.cells.len() as u32)))
      .collect();
    bodies.sort_unstable();
    let mut next = 0;
    for &(entry, end, ..) in &bodies {
      if entry != next || end <= entry {
        return Err(VerifyError {
          at: entry,
          what: "the code of a body does not follow the one before",
        });
      }
      next = end;
    }
    if next as usize != self.code.len() {
      return Err(VerifyError {
        at: next,
        what: "code after the last body",
      });
    }
    for proto in &self.protos {
      let check = Check {
        program: self,
        at: proto.entry,
        slots: proto.slots,
      };
      for &slot in proto.params.iter() {
        check.slot(slot)?;
      }
      check.region(proto.region)?;
      if proto.captures.len() != proto.closure.iter().count() {
        return check.fail("a capture per closed-over name");
      }
      for &first in proto.captures.iter() {
        if first != NONE {
          check.index(first, self.names.len(), "name out of range")?;
        }
      }
      for &(name, cell) in proto.cells.iter() {
        check.index(name, self.names.len(), "name out of range")?;
        check.index(cell, proto.cells.len(), "cell out of range")?;
      }
      if !proto.cells.is_sorted() {
        return check.fail("cells not by name");
      }
    }
    for &(entry, end, slots, cells) in &bodies {
      self.verify_body(entry, end, slots, cells)?;
    }
    for (at, region) in self.regions.iter().enumerate() {
      let check = Check {
        program: self,
        at: at as Pc,
        slots: self.slots,
      };
      if region.parent != NONE && region.parent as usize >= at {
        return check.fail("a region is not held by an earlier one");
      }
    }
    Ok(())
  }

  /// [Program::verify] for the body whose code is `entry .. end`, whose frame has `slots` slots
  /// and whose activation `cells` cells.
  fn verify_body(&self, entry: Pc, end: Pc, slots: u32, cells: u32) -> Result<(), VerifyError> {
    self.verify_structure(entry, end)?;
    for (at, op) in self.code[entry as usize..end as usize].iter().enumerate() {
      let check = Check {
        program: self,
        at: entry + at as Pc,
        slots,
      };
      match *op {
        Op::Fail { dst, msg } => {
          check.slot(dst)?;
          check.index(msg, self.messages.len(), "message out of range")?;
        }
        Op::Warn { msg } => check.index(msg, self.messages.len(), "message out of range")?,
        Op::LoadName { dst, name, cell } => {
          check.slot(dst)?;
          check.index(name, self.names.len(), "name out of range")?;
          if cell >= cells {
            return check.fail("cell out of range");
          }
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
        Op::Jump { .. } | Op::ListNew { .. } | Op::FoldNew { .. } => {}
        Op::FoldAdd { src, dst, .. } => {
          check.slot(dst)?;
          check.opnd(src)?;
        }
        Op::FoldEnd { dst } => check.slot(dst)?,
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
        Op::HomeOpen { layout } => check.layout(layout)?,
        Op::HomeSet { s, .. } => check.slot(s)?,
        Op::HomeClose => {}
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
        Op::Closure { dst, closure } => {
          check.slot(dst)?;
          check.index(closure, self.closures.len(), "function definition out of range")?;
          let info = self.closures[closure as usize];
          check.index(info.proto, self.protos.len(), "proto out of range")?;
          check.opnd(info.parameters)?;
          if let Some(body) = info.body {
            check.opnd(body)?;
          }
          check.region(info.region)?;
        }
        Op::Return { src } => check.opnd(src)?,
      }
    }
    match self.code[..end as usize].last() {
      Some(Op::Return { .. }) => Ok(()),
      _ => Err(VerifyError {
        at: end,
        what: "the code of a body does not end with a return",
      }),
    }
  }
}

impl Program {
  /// Checks that every jump of the body whose code is `entry .. end` goes forward to an
  /// instruction of the body, and that lists, invocations, folds and walks are opened and closed
  /// in nesting order on every path: the code is structured, so what is open at an instruction is
  /// what is open there in the order of the code, the same from every jump. An [Op::Enter] that
  /// refuses continues past its [Op::Leave], with what was open before it. A walk is the record of
  /// its nesting depth in the body, and the only jumps back are those of a walk to its step,
  /// inside it.
  fn verify_structure(&self, entry: Pc, end: Pc) -> Result<(), VerifyError> {
    let code = &self.code[entry as usize..end as usize];
    // the open lists, invocations, folds and walks before each instruction of the body
    let mut open = Vec::with_capacity(code.len() + 1);
    let (mut lists, mut calls, mut folds) = (0_u32, 0_u32, 0_u32);
    // the open walks and filters: whether a walk, the index of its info, where it opens
    let mut records: Vec<(bool, u32, Pc)> = vec![];
    for (at, op) in code.iter().enumerate() {
      let at = entry + at as Pc;
      open.push((lists, calls, folds, records.len() as u32));
      let fail = |what| Err(VerifyError { at, what });
      match *op {
        Op::LoopOpen { record, info } | Op::FilterOpen { record, info, .. } => {
          let walk = matches!(op, Op::LoopOpen { .. });
          if info as usize >= if walk { self.loops.len() } else { self.filters.len() } {
            return fail("iteration or filter out of range");
          }
          if record as usize != records.len() {
            return fail("a walk or a filter that is not the record of its depth");
          }
          records.push((walk, info, at));
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
          let inside = |pc: Pc| pc > opened && pc <= at;
          let inside = if walk {
            let info = &self.loops[info as usize];
            info.exit == at && inside(info.step) && info.levels.iter().all(|&(_, bind)| inside(bind))
          } else {
            let info = &self.filters[info as usize];
            info.close == at && info.after == at + 1 && info.body == opened + 1
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
        Op::FoldNew { .. } => folds += 1,
        Op::FoldAdd { .. } if folds == 0 => return fail("no fold is open"),
        Op::FoldEnd { .. } if folds == 0 => return fail("no fold is open"),
        Op::FoldEnd { .. } => folds -= 1,
        Op::Return { .. } if lists != 0 || calls != 0 || folds != 0 || !records.is_empty() => return fail("a list, an invocation, a fold or a walk is open at the return"),
        _ => {}
      }
    }
    if !records.is_empty() {
      return Err(VerifyError {
        at: end,
        what: "a walk is not closed",
      });
    }
    // what is open at an instruction of the body
    let open_at = |pc: Pc| pc.checked_sub(entry).and_then(|index| open.get(index as usize)).copied();
    // the targets of the walks and filters: what is open there is what is open inside them
    for (at, op) in code.iter().enumerate() {
      let at = entry + at as Pc;
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
      if targets.iter().any(|&target| open_at(target).is_none() || open_at(target) != open_at(at + 1)) {
        return Err(VerifyError {
          at,
          what: "a walk or a filter that jumps across a list, an invocation or a fold",
        });
      }
    }
    for (at, op) in code.iter().enumerate() {
      let at = entry + at as Pc;
      let (target, state) = match *op {
        Op::Jump { to } | Op::JumpUnlessTrue { to, .. } | Op::AndLeft { to, .. } | Op::OrLeft { to, .. } => (to, open_at(at + 1)),
        Op::Enter { skip, .. } => (skip, open_at(at)),
        Op::ProbeItem { found, .. } | Op::ItemSelf { found, .. } => (found, open_at(at + 1)),
        // a null list skips the filter
        Op::FilterOpen { info, .. } => (self.filters[info as usize].after, open_at(at)),
        // a failure closes the fold
        Op::FoldAdd { fail, .. } => (fail, open_at(at).map(|(lists, calls, folds, records)| (lists, calls, folds - 1, records))),
        _ => continue,
      };
      let fail = |what| Err(VerifyError { at, what });
      if target <= at || target >= end {
        return fail("jump out of range");
      }
      if open_at(target) != state {
        return fail("jump across a list, an invocation or a fold");
      }
      if matches!(op, Op::Enter { .. }) && !matches!(self.code[target as usize - 1], Op::Leave { .. }) {
        return fail("an invocation that refuses does not continue past a leave");
      }
    }
    Ok(())
  }
}

/// The checks of one instruction of a body of a program whose frame has `slots` slots.
struct Check<'p> {
  program: &'p Program,
  at: Pc,
  slots: u32,
}

impl Check<'_> {
  fn fail(&self, what: &'static str) -> Result<(), VerifyError> {
    Err(VerifyError { at: self.at, what })
  }

  fn index(&self, index: Ix, len: usize, what: &'static str) -> Result<(), VerifyError> {
    if (index as usize) < len { Ok(()) } else { self.fail(what) }
  }

  fn slot(&self, slot: Slot) -> Result<(), VerifyError> {
    if slot < self.slots { Ok(()) } else { self.fail("slot out of range") }
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
  closures: Vec<ClosureInfo>,
  protos: Vec<Proto>,
  root_cells: u32,
}

impl ProgramBuilder {
  pub(crate) fn emit(&mut self, op: Op) -> Pc {
    self.code.push(op);
    self.code.len() as Pc - 1
  }

  /// The root expression reads `cells` free names, each into a cell of its own.
  pub(crate) fn root_cells(&mut self, cells: u32) {
    self.root_cells = cells;
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
      | Op::FoldAdd { fail: to, .. }
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

  /// The index of the function definition `info` in the program.
  pub(crate) fn closure(&mut self, info: ClosureInfo) -> Ix {
    self.closures.push(info);
    self.closures.len() as Ix - 1
  }

  /// The body of the function definitions of `proto`, whose code has been emitted.
  pub(crate) fn proto(&mut self, index: u32, proto: Proto) {
    let index = index as usize;
    if self.protos.len() <= index {
      self.protos.resize(index + 1, Proto::default());
    }
    self.protos[index] = proto;
  }

  /// The index of the search `search` in the program.
  pub(crate) fn search(&mut self, search: SearchInfo) -> Ix {
    self.searches.push(search);
    self.searches.len() as Ix - 1
  }

  /// The program in `dialect`, whose root expression has a frame of `slots` slots and its code
  /// up to `root_end`, with the layouts and regions of its name resolution; checked in builds
  /// with debug assertions.
  pub(crate) fn finish(self, slots: u32, root_end: Pc, dialect: Dialect, layouts: Vec<Layout>, regions: Vec<Region>) -> Arc<Program> {
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
      closures: self.closures,
      protos: self.protos,
      layouts,
      regions,
      slots,
      root_cells: self.root_cells,
      root_end,
      dialect,
    };
    #[cfg(debug_assertions)]
    if let Err(error) = program.verify() {
      panic!("invalid program: {error:?}\n{:?}", program.code);
    }
    Arc::new(program)
  }
}
