//! # The run loop
//!
//! One evaluation of a [Program]: the frames of slots of its activations, and one loop that runs
//! the instructions in order. Every value is computed by the kernels
//! ([crate::kernels]), and every error, warning and limit is recorded in the evaluation's
//! diagnostics in the order of the instructions.
//!
//! An invocation of a function whose body is code of a program is an activation of that body in
//! the same run (on the heap: no recursion of the host). Its body reads the scopes of where the
//! function is written: its parameters, the entries of the context literals it is written in
//! (its home), its closure, then the scope of the host. It reads nothing of its caller.

use super::program::{CallInfo, Compare, Domain, Fold, Kernel1, Kernel2, ListKind, LoopKind, NONE, Op, Operand, Opnd, Pc, Program, Slot};
use super::resolve::{Place, Region, RegionKind};
use crate::Dialect;
use crate::MAX_CALL_DEPTH;
use crate::bfeel;
use crate::filters::{ITEM, item_at, outcome};
use crate::invocation::{Arguments, apply_feel_built_in, arity, coerce_arguments, in_order, invoke};
use crate::iterations::Sequence;
use crate::kernels::{self, Arithmetic, finite};
use rheon_feel::bif::Bif;
use rheon_feel::context::FeelContext;
use rheon_feel::values::{FeelList, Value};
use rheon_feel::{FeelCode, FeelScope, FeelType, FunctionBody, FunctionDefinition, Home, Name, diagnostics, value_null};
use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Arc;

/// The slots of a frame that a run keeps on the stack of the host instead of allocating them.
const INLINE_SLOTS: usize = 4;

/// The base of the activation whose slots are those a run starts with.
const BOTTOM: usize = usize::MAX;

/// Calls `run` with `size` slots, all null: on the stack of the host when they are few.
fn with_slots(size: usize, run: impl FnOnce(&mut [Value]) -> Value) -> Value {
  if size <= INLINE_SLOTS {
    let mut slots: [Value; INLINE_SLOTS] = std::array::from_fn(|_| Value::Null(None));
    let value = run(&mut slots[..size]);
    for slot in slots {
      slot.discard();
    }
    value
  } else {
    run(&mut vec![Value::Null(None); size])
  }
}

impl Program {
  /// Evaluates the program in `scope`.
  pub(crate) fn run(self: &Arc<Self>, scope: &FeelScope) -> Value {
    with_slots(self.slots as usize, |slots| Vm::new(Activation::root(self), scope, slots).execute(0))
  }
}

/// The program and the proto of `body`, when it is the body of a function definition of the
/// flat evaluator.
pub(crate) fn code_of(body: &FunctionBody) -> Option<(Arc<Program>, u32)> {
  match body {
    FunctionBody::Feel(code, proto) => Arc::clone(code).into_any().downcast::<Program>().ok().map(|program| (program, *proto)),
    _ => None,
  }
}

/// A program holds the bodies of its function definitions, which the functions that a
/// definition makes keep while they live.
impl FeelCode for Program {
  /// The body `proto` in a scope that holds its arguments: each parameter is the value of its
  /// name in `scope` (as the scope of a call holds it), or, without one, the value of a free name
  /// of that name; the scope holds the closure as well, so the body reads every other name from
  /// it.
  fn evaluate(self: Arc<Self>, proto: u32, scope: &FeelScope) -> Value {
    let body = &self.protos[proto as usize];
    with_slots(body.slots as usize, |slots| {
      if let RegionKind::Layout { layout, .. } = self.regions[body.region as usize].kind {
        for &(name, place) in self.layouts[layout as usize].entries.iter() {
          if let Place::Slot(slot) = place {
            let name = &self.names[name as usize];
            slots[slot as usize] = scope.get_value(name).unwrap_or_else(|| kernels::free_name(name));
          }
        }
      }
      let entry = body.entry;
      let activation = Activation::body(Arc::clone(&self), None, proto, BOTTOM, 0, 0);
      Vm::new(activation, scope, slots).execute(entry)
    })
  }

  fn into_any(self: Arc<Self>) -> Arc<dyn std::any::Any + Send + Sync> {
    self
  }
}

/// Evaluates the body of `function`, the proto `proto` of `program`, with its parameters bound to
/// `values` (one per parameter, in their order, converted to their types): the body reads its
/// parameters, the function's home and closure, and then `scope`.
#[inline(never)]
pub(crate) fn run_function(scope: &FeelScope, function: &Arc<FunctionDefinition>, program: Arc<Program>, proto: u32, values: impl IntoIterator<Item = Value>) -> Value {
  let body = &program.protos[proto as usize];
  let (entry, slots) = (body.entry, body.slots as usize);
  with_slots(slots, |slots| {
    // parameters of one name share a slot, which holds the last of them
    for (&slot, value) in body.params.iter().zip(values) {
      slots[slot as usize] = value;
    }
    let activation = Activation::body(Arc::clone(&program), Some(Arc::clone(function)), proto, BOTTOM, 0, 0);
    Vm::new(activation, scope, slots).execute(entry)
  })
}

/// The run of the root expression or of a body of a program.
struct Activation {
  program: Arc<Program>,
  /// The function whose body runs, whose closure the body sees; `None` for the root expression
  /// and for a body whose scope holds the closure.
  function: Option<Arc<FunctionDefinition>>,
  /// The proto of the body, [NONE] for the root expression.
  proto: u32,
  /// Where its slots start in [Vm::stack], [BOTTOM] for those the run starts with.
  base: usize,
  /// Where its records start in [Vm::records].
  records: usize,
  /// Where its cells start in [Vm::cells].
  cells: usize,
  /// Where its homes start in [Vm::homes].
  homes: usize,
}

impl Activation {
  fn root(program: &Arc<Program>) -> Self {
    Self::body(Arc::clone(program), None, NONE, BOTTOM, 0, 0)
  }

  fn body(program: Arc<Program>, function: Option<Arc<FunctionDefinition>>, proto: u32, base: usize, records: usize, cells: usize) -> Self {
    Self {
      program,
      function,
      proto,
      base,
      records,
      cells,
      homes: 0,
    }
  }
}

/// An activation that invoked a function and waits for its value: where it continues, where the
/// value goes, and the invocation (whose region is the scope that the function's body sees below
/// its own).
struct Frame {
  activation: Activation,
  ret: Pc,
  dst: Slot,
  call: u32,
}

/// The value of a free name in an activation, once it was read: the chain of scopes below an
/// activation does not change while it runs (its callers wait, and the host's scopes are written
/// only by the bodies of the host, into the scopes those push).
#[derive(Clone)]
enum Cell {
  Unread,
  Read(Value),
}

/// The state of one run.
struct Vm<'p> {
  host: &'p FeelScope,
  /// The slots of the activation the run starts with.
  bottom: &'p mut [Value],
  /// The slots of the activations above it, a window each.
  stack: Vec<Value>,
  /// The activation that runs.
  act: Activation,
  /// The activations that wait for the value of a function they invoked, innermost last.
  frames: Vec<Frame>,
  /// The values of free names of the activations, a window each.
  cells: Vec<Cell>,
  /// The arguments of a named invocation, in the order of the parameters.
  named: Vec<Value>,
  /// The lists being built, innermost last.
  lists: Vec<Vec<Value>>,
  /// The context types and formal parameters being built, innermost last.
  folds: Vec<Folding>,
  /// The invocations entered, innermost last.
  calls: Vec<diagnostics::Call>,
  /// The open walks, innermost last: the record of a walk of an activation is its index there.
  records: Vec<Record>,
  /// The domains of the variables that have a value in the open walks, innermost last.
  domains: Vec<Sequence>,
  /// For each layout of the program the run starts with, the scope that invocations of
  /// functions of the host get for a region of it, once one was invoked there, and how many
  /// entries of the layout that region sees: its names are those the region sees, its values are
  /// set for each invocation and set back to null after it, so it holds no value between
  /// invocations. One scope serves every region of a layout, so what a run keeps grows with the
  /// binders of its program, not with the regions that invoked a function.
  layout_scopes: Vec<Option<(Arc<FeelContext>, u32)>>,
  /// For each region of the program the run starts with that is the item of a filter, the scope
  /// that holds `item` for invocations of functions of the host, kept the same way.
  scopes: Vec<Option<Arc<FeelContext>>>,
  /// The scopes of the invocation being made, innermost first: the activation (its depth) and
  /// the region.
  chain: Vec<(usize, u32)>,
  /// The homes of the context literals that are being evaluated, those of an activation above
  /// those of the activations that wait for it.
  homes: Vec<Arc<Home>>,
}

/// A context type or the formal parameters of a definition, being built.
enum Folding {
  ContextType(BTreeMap<Name, FeelType>),
  Parameters(Vec<(Name, FeelType)>),
}

/// An open walk or filter.
enum Record {
  /// The walk of a `for`, `some` or `every` expression.
  Walk(Walk),
  /// The items of a filter.
  Filter(Filter),
}

/// The items of a filter: while its predicate is evaluated in the scope around it (`mark` holds
/// the mark of what was recorded before), then while it visits them, the item at `at`, whether it
/// is a context with an entry `item`, the items kept, and the filter's value when it is decided.
struct Filter {
  info: u32,
  items: FeelList,
  /// A value that is no list is filtered as the list of it, and the result is one level deeper.
  wrap: bool,
  mark: Option<diagnostics::Tentative>,
  visiting: bool,
  at: usize,
  item_entry: bool,
  kept: Vec<Value>,
  value: Option<Value>,
}

impl Filter {
  /// The item visited, while the filter visits its items.
  fn item(&self) -> Option<&Value> {
    if self.visiting { self.items.get(self.at) } else { None }
  }
}

/// The walk of an iteration: the domains from `base` in [Vm::domains] are those of its variables
/// that have a value, in order; the results so far (of a `for`); and its value when a step,
/// a domain or the body has decided it.
struct Walk {
  info: u32,
  base: usize,
  results: FeelList,
  value: Option<Value>,
}

impl Drop for Vm<'_> {
  /// Leaves the invocations still entered (when a run ends early), innermost first, as their
  /// nesting requires.
  fn drop(&mut self) {
    while let Some(call) = self.calls.pop() {
      drop(call);
    }
  }
}

/// How a run continues after an instruction that may change the activation.
enum Next {
  /// At this position, in the same program.
  At(usize),
  /// At this position, in the program of the new activation.
  Switch(usize),
}

impl<'p> Vm<'p> {
  fn new(act: Activation, host: &'p FeelScope, bottom: &'p mut [Value]) -> Self {
    let cells = act
      .program
      .protos
      .get(act.proto as usize)
      .map_or(act.program.root_cells as usize, |proto| proto.cells.len());
    Self {
      host,
      bottom,
      stack: vec![],
      act,
      frames: vec![],
      cells: vec![Cell::Unread; cells],
      named: vec![],
      lists: vec![],
      folds: vec![],
      calls: vec![],
      records: vec![],
      domains: vec![],
      layout_scopes: vec![],
      scopes: vec![],
      chain: vec![],
      homes: vec![],
    }
  }

  /// The slot `slot` of the activation that runs.
  #[inline(always)]
  fn slot(&mut self, slot: Slot) -> &mut Value {
    match self.act.base {
      BOTTOM => &mut self.bottom[slot as usize],
      base => &mut self.stack[base + slot as usize],
    }
  }

  /// Gives the slot `slot` of the activation that runs the value `value`; the value it had is
  /// dropped.
  #[inline(always)]
  fn set(&mut self, slot: Slot, value: Value) {
    std::mem::replace(self.slot(slot), value).discard();
  }

  /// The slot `slot` of the activation whose slots start at `base`.
  fn slot_of(&self, base: usize, slot: Slot) -> &Value {
    match base {
      BOTTOM => &self.bottom[slot as usize],
      base => &self.stack[base + slot as usize],
    }
  }

  /// The record `record` of the activation whose records start at `records`.
  fn record(&self, records: usize, record: u32) -> &Record {
    &self.records[records + record as usize]
  }

  /// The walk of `record`, open by the structure of the code ([Program::verify]).
  fn walk(&mut self, record: u32) -> &mut Walk {
    match &mut self.records[self.act.records + record as usize] {
      Record::Walk(walk) => walk,
      Record::Filter(_) => unreachable!("the record of a walk is a filter"),
    }
  }

  /// The filter of `record` of the activation whose records start at `records`, open by the
  /// structure of the code ([Program::verify]).
  fn filter_of(&self, records: usize, record: u32) -> &Filter {
    match self.record(records, record) {
      Record::Filter(filter) => filter,
      Record::Walk(_) => unreachable!("the record of a filter is a walk"),
    }
  }

  /// The filter of `record`, to change.
  fn filter_mut(&mut self, record: u32) -> &mut Filter {
    match &mut self.records[self.act.records + record as usize] {
      Record::Filter(filter) => filter,
      Record::Walk(_) => unreachable!("the record of a filter is a walk"),
    }
  }

  /// The value of the temporary `slot`, which is left null.
  fn take(&mut self, slot: Slot) -> Value {
    std::mem::replace(self.slot(slot), Value::Null(None))
  }

  /// The value of an operand; a temporary is left null.
  fn read(&mut self, opnd: Opnd) -> Value {
    match opnd.decode() {
      Operand::Slot(slot) => self.slot(slot).copied(),
      Operand::Take(slot) => self.take(slot),
      Operand::Const(index) => self.act.program.consts[index as usize].copied(),
    }
  }

  /// Whether an operand is `true`; a temporary is left null.
  #[inline(always)]
  fn is_true(&mut self, opnd: Opnd) -> bool {
    match opnd.decode() {
      Operand::Slot(slot) => matches!(self.slot_of(self.act.base, slot), Value::Boolean(true)),
      Operand::Take(slot) => matches!(self.take(slot), Value::Boolean(true)),
      Operand::Const(index) => matches!(self.act.program.consts[index as usize], Value::Boolean(true)),
    }
  }

  /// Runs from `entry` in the activation the run starts with, to the value of its body.
  fn execute(&mut self, entry: Pc) -> Value {
    let mut pc = entry as usize;
    loop {
      match self.run(pc) {
        Ok(next) => pc = next,
        Err(value) => return value,
      }
    }
  }

  /// Runs from `pc` in the program of the activation that runs, up to the end of the run (its
  /// value) or to an activation of another program (where it continues there).
  fn run(&mut self, pc: usize) -> Result<usize, Value> {
    let program = Arc::clone(&self.act.program);
    let program: &Program = &program;
    let mut pc = pc;
    loop {
      let op = program.code[pc];
      pc += 1;
      match op {
        Op::Fail { dst, msg } => {
          let value = value_null!(&*program.messages[msg as usize]);
          self.set(dst, value);
        }
        Op::Warn { msg } => diagnostics::warn(&program.messages[msg as usize]),
        Op::LoadName { dst, name, cell } => {
          let value = self.load_name(program, name, cell);
          self.set(dst, value);
        }
        Op::Search { dst, search } => {
          let info = program.searches[search as usize];
          // a free name is in no scope of the body
          let region = if info.region == NONE { self.bottom_region() } else { info.region };
          let segments = &program.name_lists[info.segments as usize];
          let value = self.search_from(region, info.first, segments).unwrap_or_else(kernels::free_qualified_name);
          self.set(dst, value);
        }
        Op::Path { dst, a, name } => {
          let a = self.read(a);
          let value = kernels::path(a, &program.names[name as usize]);
          self.set(dst, value);
        }
        Op::Descendant { dst, a, name } => {
          let a = self.read(a);
          let value = kernels::descendants(a, &program.names[name as usize]);
          self.set(dst, value);
        }
        Op::Move { dst, src } => {
          let value = self.read(src);
          self.set(dst, value);
        }
        Op::Arith { op, dst, a, b } => {
          let (a, b) = (self.read(a), self.read(b));
          let value = op.apply(a, b);
          self.set(dst, value);
        }
        Op::ArithB { op, dst, a, b } => {
          let (a, b) = (self.read(a), self.read(b));
          let value = bfeel::arithmetic(op, a, b);
          self.set(dst, value);
        }
        Op::Neg { dst, a } => {
          let value = Arithmetic::negation(self.read(a));
          self.set(dst, value);
        }
        Op::Compare { op, dst, a, b } => {
          let (a, b) = (self.read(a), self.read(b));
          let value = match op {
            Compare::Eq => kernels::equal(&a, &b),
            Compare::Ne => kernels::not_equal(&a, &b),
            Compare::Lt => kernels::compare_lt(a, b),
            Compare::Le => kernels::compare_le(a, b),
            Compare::Gt => kernels::compare_gt(a, b),
            Compare::Ge => kernels::compare_ge(a, b),
          };
          self.set(dst, value);
        }
        Op::Between { dst, args } => {
          let (a, low, high) = (self.take(args), self.take(args + 1), self.take(args + 2));
          let value = kernels::between(a, low, high);
          self.set(dst, value);
        }
        Op::BoolDefault { dst, ne } => {
          let value = self.take(dst);
          let value = bfeel::boolean_result(value, ne);
          self.set(dst, value);
        }
        Op::Kernel1 { kernel, dst, a } => {
          let a = self.read(a);
          let value = match kernel {
            Kernel1::UnaryEq => Value::UnaryEqual(Box::new(a)),
            Kernel1::UnaryNe => Value::UnaryNotEqual(Box::new(a)),
            Kernel1::UnaryLt => Value::UnaryLess(Box::new(a)),
            Kernel1::UnaryLe => Value::UnaryLessOrEqual(Box::new(a)),
            Kernel1::UnaryGt => Value::UnaryGreater(Box::new(a)),
            Kernel1::UnaryGe => Value::UnaryGreaterOrEqual(Box::new(a)),
            Kernel1::IntervalStart(closed) => Value::IntervalStart(Box::new(a), closed),
            Kernel1::IntervalEnd(closed) => Value::IntervalEnd(Box::new(a), closed),
            Kernel1::ListType => kernels::list_type(a),
            Kernel1::RangeType => kernels::range_type(a),
            Kernel1::ExternalBody => kernels::external_body(a),
            Kernel1::TestAnswer(boolean) => match a {
              answer @ Value::Boolean(_) => Value::TestAnswer(Box::new(answer)),
              answer @ Value::Null(_) if boolean => Value::TestAnswer(Box::new(answer)),
              value => value,
            },
          };
          self.set(dst, value);
        }
        Op::Kernel2 {
          kernel: kernel @ (Kernel2::In | Kernel2::InTwoValued),
          dst,
          a,
          b,
        } => {
          let two_valued = matches!(kernel, Kernel2::InTwoValued);
          let a = self.read(a);
          // tests that are a constant of the program are read where they are
          let value = match b.decode() {
            Operand::Const(index) => kernels::in_operator(&a, &self.act.program.consts[index as usize], two_valued),
            _ => {
              let b = self.read(b);
              kernels::in_operator(&a, &b, two_valued)
            }
          };
          let value = value;
          self.set(dst, value);
        }
        Op::Kernel2 { kernel, dst, a, b } => {
          let (a, b) = (self.read(a), self.read(b));
          let value = match kernel {
            Kernel2::In | Kernel2::InTwoValued => unreachable!("an `in` is the instruction above"),
            Kernel2::Out => match a {
              Value::Boolean(true) => b,
              _ => value_null!(),
            },
            Kernel2::Range => kernels::range_of(a, b),
            Kernel2::InstanceOf => kernels::instance_of(a, b),
            Kernel2::ContextTypeEntry => kernels::context_type_entry(a, b),
            Kernel2::FunctionType => kernels::function_type(a, b),
            Kernel2::FormalParameter => kernels::formal_parameter(a, b),
          };
          self.set(dst, value);
        }
        Op::Jump { to } => pc = to as usize,
        Op::JumpUnlessTrue { c, to } => {
          if !self.is_true(c) {
            pc = to as usize;
          }
        }
        Op::AndLeft { dst, a, to } => {
          let a = self.read(a);
          if matches!(a, Value::Boolean(false)) {
            pc = to as usize;
          }
          let value = a;
          self.set(dst, value);
        }
        Op::AndRight { dst, b } => {
          let (a, b) = (self.take(dst), self.read(b));
          let value = match (a, b) {
            (Value::Boolean(true), b @ Value::Boolean(_)) => b,
            (Value::Boolean(true), _) => value_null!(),
            (_, Value::Boolean(false)) => Value::Boolean(false),
            _ => value_null!(),
          };
          self.set(dst, value);
        }
        Op::OrLeft { dst, a, to } => {
          let a = self.read(a);
          if matches!(a, Value::Boolean(true)) {
            pc = to as usize;
          }
          let value = a;
          self.set(dst, value);
        }
        Op::OrRight { dst, b } => {
          let (a, b) = (self.take(dst), self.read(b));
          let value = match (a, b) {
            (Value::Boolean(false), b @ Value::Boolean(_)) => b,
            (Value::Boolean(false), _) => value_null!(),
            (_, Value::Boolean(true)) => Value::Boolean(true),
            _ => value_null!(),
          };
          self.set(dst, value);
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
          let value = match kind {
            ListKind::List => kernels::list_of(items),
            ListKind::ExpressionList => Value::ExpressionList(items),
            ListKind::NegatedList => Value::NegatedCommaList(items),
            ListKind::ParameterTypes => Value::ParameterTypes(items),
          };
          self.set(dst, value);
        }
        Op::FoldNew { kind } => self.folds.push(match kind {
          Fold::ContextType => Folding::ContextType(BTreeMap::new()),
          Fold::Parameters => Folding::Parameters(vec![]),
        }),
        Op::FoldAdd { src, dst, fail } => {
          let value = self.read(src);
          let added = match self.folds.last_mut() {
            Some(Folding::ContextType(entries)) => kernels::add_context_type_entry(entries, value),
            Some(Folding::Parameters(parameters)) => kernels::add_formal_parameter(parameters, value),
            None => Ok(()),
          };
          if let Err(null) = added {
            self.folds.pop();
            let value = null;
            self.set(dst, value);
            pc = fail as usize;
          }
        }
        Op::FoldEnd { dst } => {
          let value = match self.folds.pop() {
            Some(Folding::ContextType(entries)) => Value::FeelType(FeelType::Context(entries)),
            Some(Folding::Parameters(parameters)) => Value::FormalParameters(parameters),
            None => Value::Null(None),
          };
          self.set(dst, value);
        }
        Op::Closure { dst, closure } => {
          let value = self.closure(closure);
          self.set(dst, value);
        }
        Op::Enter { dst, skip } => match diagnostics::enter_call(MAX_CALL_DEPTH) {
          Some(call) => self.calls.push(call),
          None => {
            let value = Value::Null(None);
            self.set(dst, value);
            pc = skip as usize;
          }
        },
        Op::Call { dst, call } => match self.call(program, pc, dst, call) {
          Next::At(next) => pc = next,
          Next::Switch(next) => return Ok(next),
        },
        Op::Leave { dst } => {
          let value = diagnostics::bound_depth(finite(self.take(dst)));
          if let Some(call) = self.calls.pop() {
            let value = call.finish(value);
            self.set(dst, value);
          }
        }
        Op::BoundDepth { s } => {
          let value = self.take(s);
          let value = diagnostics::bound_depth(value);
          self.set(s, value);
        }
        Op::HomeOpen { layout } => {
          let names = program.layouts[layout as usize]
            .entries
            .iter()
            .map(|&(name, _)| program.names[name as usize].clone())
            .collect();
          let home = Home::new(names, self.home());
          self.homes.push(home);
        }
        Op::HomeSet { entry, s } => {
          let value = self.slot(s).clone();
          if let Some(home) = self.homes.last() {
            home.set(entry as usize, value);
          }
        }
        Op::HomeClose => {
          self.homes.pop();
        }
        Op::Clear { layout, visible } => {
          for &(_, place) in &program.layouts[layout as usize].entries[..visible as usize] {
            if let Place::Slot(slot) = place {
              let value = Value::Null(None);
              self.set(slot, value);
            }
          }
        }
        Op::Context { dst, layout } => {
          let mut context = FeelContext::default();
          for &(name, place) in program.layouts[layout as usize].entries.iter() {
            if let Place::Slot(slot) = place {
              let value = self.take(slot);
              context.set_entry(&program.names[name as usize], value);
            }
          }
          if program.layouts[layout as usize].other_forms {
            context = context.with_other_key_forms(true);
          }
          let value = diagnostics::bound_depth(context.into());
          self.set(dst, value);
        }
        Op::LoopOpen { record: _, info } => {
          let base = self.domains.len();
          self.records.push(Record::Walk(Walk {
            info,
            base,
            results: FeelList::new(),
            value: None,
          }));
        }
        Op::LevelPush { record, domain, a, b } => {
          let a = self.read(a);
          let domain = match domain {
            Domain::Interval => Sequence::interval(a, self.read(b)),
            Domain::Range => Sequence::range(a, self.read(b)),
            Domain::Of => Ok(Sequence::of(a)),
          };
          let walk = self.walk(record);
          let info = &program.loops[walk.info as usize];
          pc = match domain {
            Ok(domain) => {
              self.domains.push(domain);
              info.step
            }
            Err(reason) => {
              walk.value = Some(value_null!(reason));
              info.exit
            }
          } as usize;
        }
        Op::Step { record } => pc = self.step(program, record) as usize,
        Op::Visit { record, src } => {
          let value = self.read(src);
          let walk = self.walk(record);
          let info = &program.loops[walk.info as usize];
          pc = match info.kind {
            LoopKind::For => {
              walk.results.push(value);
              // the results grow with every iteration, through `partial` by a level in each
              match diagnostics::bound_list_depth(&walk.results) {
                Ok(()) => info.step,
                Err(null) => {
                  walk.value = Some(null);
                  info.exit
                }
              }
            }
            LoopKind::Some | LoopKind::Every => {
              let decisive = info.kind == LoopKind::Some;
              match value {
                Value::Boolean(value) if value != decisive => info.step,
                Value::Boolean(_) => {
                  walk.value = Some(Value::Boolean(decisive));
                  info.exit
                }
                _ => {
                  walk.value = Some(value_null!());
                  info.exit
                }
              }
            }
          } as usize;
        }
        Op::FilterOpen { record: _, info, list } => {
          let (items, wrap) = match self.read(list) {
            Value::List(items) => (items, false),
            Value::Null(_) => {
              let filter = &program.filters[info as usize];
              let value = value_null!();
              self.set(filter.dst, value);
              pc = filter.after as usize;
              continue;
            }
            other => (FeelList::from(vec![other]), true),
          };
          self.records.push(Record::Filter(Filter {
            info,
            items,
            wrap,
            mark: Some(diagnostics::tentative_mark()),
            visiting: false,
            at: 0,
            item_entry: false,
            kept: vec![],
            value: None,
          }));
        }
        Op::FilterStep { record, c } => {
          let c = self.read(c);
          pc = self.filter_step(program, record, c) as usize;
        }
        Op::FilterClose { record: _ } => {
          if let Some(Record::Filter(filter)) = self.records.pop() {
            let info = &program.filters[filter.info as usize];
            let value = filter.value.unwrap_or(Value::Null(None));
            let value = if filter.wrap { diagnostics::bound_depth(value) } else { value };
            self.set(info.dst, value);
          }
        }
        Op::ProbeItem { dst, probe, found } => {
          let probe = program.probes[probe as usize];
          if let Some(Value::Context(item)) = self.filter_of(self.act.records, probe.record).item()
            && let Some(value) = item.get_entry(&program.names[probe.name as usize])
          {
            let value = value.clone();
            let value = value;
            self.set(dst, value);
            pc = found as usize;
          }
        }
        Op::ItemSelf { dst, record, found } => {
          let filter = self.filter_of(self.act.records, record);
          if let Some(item) = filter.item() {
            let value = match item {
              Value::Context(context) if filter.item_entry => context.get_entry(&ITEM).cloned().unwrap_or(Value::Null(None)),
              item => item.clone(),
            };
            let value = value;
            self.set(dst, value);
            pc = found as usize;
          }
        }
        Op::LoopClose { record: _, dst } => {
          if let Some(Record::Walk(walk)) = self.records.pop() {
            let info = &program.loops[walk.info as usize];
            self.domains.truncate(walk.base);
            for &(slot, _) in info.levels.iter() {
              let value = Value::Null(None);
              self.set(slot, value);
            }
            let value = walk.value.unwrap_or_else(|| match info.kind {
              LoopKind::For => Value::List(walk.results),
              LoopKind::Some => Value::Boolean(false),
              LoopKind::Every => Value::Boolean(true),
            });
            self.set(dst, value);
          }
        }
        Op::LoadPartial { dst, record } => {
          let value = Value::List(self.walk(record).results.clone());
          self.set(dst, value);
        }
        Op::Return { src } => {
          let value = self.read(src);
          match self.ret(program, value) {
            Ok(Next::At(next)) => pc = next,
            Ok(Next::Switch(next)) => return Ok(next),
            Err(value) => return Err(value),
          }
        }
      }
    }
  }

  /// The value of the free name `names[name]` of `program`, the program that runs: once read in
  /// the activation (its cell `cell`), the same. In the root expression the scope below is the
  /// host's, and a name it does not have is the built-in function of that name, or null with an
  /// error, at every read; in a body, a name that nothing has is the same, read once.
  fn load_name(&mut self, program: &Program, name: u32, cell: u32) -> Value {
    let text = &program.names[name as usize];
    let at = self.act.cells + cell as usize;
    if let Cell::Read(value) = &self.cells[at] {
      return value.clone();
    }
    if self.act.proto == NONE {
      // the root expression reads the scope of the host, which has the name or not for the whole
      // run; the error of a name that nothing has is recorded at every read
      let value = match self.host.get_value(text) {
        Some(value) => value,
        None => match kernels::free_name(text) {
          function @ Value::BuiltInFunction(_) => function,
          null => return null,
        },
      };
      self.cells[at] = Cell::Read(value.clone());
      return value;
    }
    let value = match self.below(name, text) {
      Some(value) => value,
      None => match Bif::from_str(text.as_str()) {
        Ok(bif) => Value::BuiltInFunction(bif),
        Err(_) => return kernels::free_name(text),
      },
    };
    self.cells[at] = Cell::Read(value.clone());
    value
  }

  /// The function of the definition `closures[closure]`: from the value of its formal parameters
  /// and the code of its body (or the value of its external body), closing over the values of
  /// the names it uses where it is defined.
  fn closure(&mut self, closure: u32) -> Value {
    let program = Arc::clone(&self.act.program);
    let info = program.closures[closure as usize];
    let parameters = self.read(info.parameters);
    let body = match info.body {
      Some(body) => self.read(body),
      None => Value::FunctionBody(FunctionBody::Feel(Arc::clone(&program) as Arc<dyn FeelCode>, info.proto), false),
    };
    let proto = &program.protos[info.proto as usize];
    let mut captures = proto.captures.iter();
    let function = kernels::function_definition(parameters, body, &proto.closure, |name| {
      let first = captures.next().copied().unwrap_or(NONE);
      self.search_from(info.region, first, name)
    });
    // a function sees the entries of the context literal it is written in
    match (function, self.home()) {
      (Value::FunctionDefinition(function), Some(home)) => Value::FunctionDefinition(Arc::new(function.at_home(home))),
      (function, _) => function,
    }
  }

  /// The home at the position of the run: that of the innermost context literal being
  /// evaluated in the activation that runs, else that of the function whose body runs.
  fn home(&self) -> Option<Arc<Home>> {
    match self.homes.get(self.act.homes..).and_then(|homes| homes.last()) {
      Some(home) => Some(Arc::clone(home)),
      None => self.act.function.as_ref().and_then(|function| function.home.clone()),
    }
  }

  /// The value of `name` in the homes of `function`, innermost first: the entry of a context
  /// literal that the function is written in, when the entry has its value.
  fn at_home(&self, function: &FunctionDefinition, name: &Name) -> Option<Value> {
    let mut home = function.home.as_ref();
    while let Some(entries) = home {
      if let Some(value) = entries.entry(name) {
        return Some(value);
      }
      home = entries.parent();
    }
    None
  }

  /// The program of the activation the run starts with.
  fn bottom_program(&self) -> &Arc<Program> {
    &self.frames.first().map_or(&self.act, |frame| &frame.activation).program
  }

  /// The invocation `program.calls[call]` at `pc` into `dst`: the body of a function of a program
  /// as an activation, a function of the host in a scope that holds the scopes of the
  /// invocation's position, a built-in function as it is.
  fn call(&mut self, program: &Program, pc: usize, dst: Slot, call: u32) -> Next {
    let info = program.calls[call as usize];
    let function = self.read(info.function);
    let code = match &function {
      Value::FunctionDefinition(definition) if !definition.external => code_of(&definition.body),
      _ => None,
    };
    if let (Some((callee, proto)), Value::FunctionDefinition(function)) = (code, &function) {
      return self.activate(program, pc, dst, call, &info, Arc::clone(function), callee, proto);
    }
    let (args, argc) = (info.args, info.argc as usize);
    let function = match (function, info.names, program.dialect) {
      // in FEEL a built-in function reads its positional arguments where they are
      (Value::BuiltInFunction(bif), NONE, Dialect::Feel) => {
        let value = match self.act.base {
          BOTTOM => apply_feel_built_in(bif, &self.bottom[args as usize..args as usize + argc]),
          base => apply_feel_built_in(bif, &self.stack[base + args as usize..base + args as usize + argc]),
        };
        for slot in args..args + argc as u32 {
          let value = Value::Null(None);
          self.set(slot, value);
        }
        let value = value;
        self.set(dst, value);
        return Next::At(pc);
      }
      (function, ..) => function,
    };
    let values = (args..args + argc as u32).map(|slot| self.take(slot));
    let arguments = if info.names == NONE {
      Arguments::Positional(values.collect())
    } else {
      Arguments::Named(program.name_lists[info.names as usize].iter().cloned().zip(values).collect())
    };
    // a function of the host reads the scopes of the position through the scope it gets
    let value = if info.region != NONE && matches!(function, Value::FunctionDefinition(_)) {
      let depth = self.materialize(info.region);
      let value = invoke(self.host, function, arguments, program.dialect);
      self.host.truncate(depth);
      self.release();
      value
    } else {
      invoke(self.host, function, arguments, program.dialect)
    };
    let value = value;
    self.set(dst, value);
    Next::At(pc)
  }

  /// The invocation of `function`, whose body is the proto `proto` of `callee`, as an activation:
  /// its arguments bound to its parameters (as [invoke] binds them), its slots above those of the
  /// activation that runs, which waits for its value. Arguments that do not match the parameters
  /// make `dst` null, and nothing is evaluated.
  #[allow(clippy::too_many_arguments)]
  fn activate(&mut self, program: &Program, pc: usize, dst: Slot, call: u32, info: &CallInfo, function: Arc<FunctionDefinition>, callee: Arc<Program>, proto: u32) -> Next {
    let parameters = function.parameters();
    let (args, argc) = (info.args, info.argc as usize);
    let bound = if info.names == NONE {
      // the arguments in their temporaries, converted there
      arity(parameters, argc).and_then(|()| {
        let base = self.act.base;
        let window = match base {
          BOTTOM => &mut self.bottom[args as usize..args as usize + argc],
          base => &mut self.stack[base + args as usize..base + args as usize + argc],
        };
        coerce_arguments(parameters, window)
      })
    } else {
      let mut named = std::mem::take(&mut self.named);
      let values = (args..args + argc as u32).map(|slot| self.take(slot));
      let bound = in_order(parameters, program.name_lists[info.names as usize].iter().zip(values), &mut named).and_then(|()| coerce_arguments(parameters, &mut named));
      self.named = named;
      bound
    };
    if let Err(reason) = bound {
      for slot in args..args + argc as u32 {
        let value = Value::Null(None);
        self.set(slot, value);
      }
      self.named.clear();
      let value = value_null!(reason);
      self.set(dst, value);
      return Next::At(pc);
    }
    let body = &callee.protos[proto as usize];
    let base = self.stack.len();
    self.stack.resize_with(base + body.slots as usize, || Value::Null(None));
    for (index, &slot) in body.params.iter().enumerate() {
      let value = if info.names == NONE {
        self.take(args + index as u32)
      } else {
        std::mem::replace(&mut self.named[index], Value::Null(None))
      };
      self.stack[base + slot as usize] = value;
    }
    self.named.clear();
    let cells = self.cells.len();
    self.cells.resize(cells + body.cells.len(), Cell::Unread);
    let entry = body.entry as usize;
    let same = std::ptr::eq(program, &*callee);
    let mut activation = Activation::body(callee, Some(function), proto, base, self.records.len(), cells);
    activation.homes = self.homes.len();
    let caller = std::mem::replace(&mut self.act, activation);
    self.frames.push(Frame {
      activation: caller,
      ret: pc as Pc,
      dst,
      call,
    });
    if same { Next::At(entry) } else { Next::Switch(entry) }
  }

  /// The end of the activation that runs, with `value`: the activation that waits for it gets
  /// the value converted to the function's result type; the run ends with the end of the
  /// activation it starts with.
  fn ret(&mut self, _program: &Program, value: Value) -> Result<Next, Value> {
    let Some(frame) = self.frames.pop() else {
      return Err(value);
    };
    let value = match &self.act.function {
      Some(function) => value.into_coerced(function.result_type()),
      None => value,
    };
    while self.stack.len() > self.act.base {
      if let Some(value) = self.stack.pop() {
        value.discard();
      }
    }
    self.cells.truncate(self.act.cells);
    self.homes.truncate(self.act.homes);
    let callee = std::mem::replace(&mut self.act, frame.activation);
    let value = value;
    self.set(frame.dst, value);
    let same = Arc::ptr_eq(&callee.program, &self.act.program);
    drop(callee);
    let next = frame.ret as usize;
    Ok(if same { Next::At(next) } else { Next::Switch(next) })
  }

  /// The index of `name` among the names of `program`, [NONE] when it is none of them (and so no
  /// binder of the program has it).
  fn name_index(&self, program: &Program, name: Option<&Name>) -> u32 {
    name
      .and_then(|name| program.names.iter().position(|known| known == name))
      .map_or(NONE, |index| index as u32)
  }

  /// The region below the scopes of the body that runs: its closure, or [NONE] for the root
  /// expression.
  fn bottom_region(&self) -> u32 {
    self.closure_region(&self.act)
  }

  /// The value of `name`, `program.names[index]` of the program that runs, below the scopes of
  /// the body that runs: in the home and the closure of its function, then in the scope of the
  /// host.
  #[cold]
  #[inline(never)]
  fn below(&self, index: u32, name: &Name) -> Option<Value> {
    let mut at = self.bottom_region();
    while at != NONE {
      let region = self.act.program.regions[at as usize];
      if let Some(value) = self.region_value(&self.act, region, name, index) {
        return Some(value);
      }
      at = region.parent;
    }
    self.host.get_value(name)
  }

  /// The closure region of the body of `act`, [NONE] for the root expression.
  fn closure_region(&self, act: &Activation) -> u32 {
    match act.program.protos.get(act.proto as usize) {
      Some(proto) => act.program.regions[proto.region as usize].parent,
      None => NONE,
    }
  }

  /// Visits the scopes from the region `region` of the activation that runs down, innermost
  /// first; the first value that `visit` gives.
  fn scopes_below(&self, region: u32, mut visit: impl FnMut(&Self, &Activation, Region) -> Option<Value>) -> Option<Value> {
    let mut at = region;
    while at != NONE {
      let region = self.act.program.regions[at as usize];
      if let Some(value) = visit(self, &self.act, region) {
        return Some(value);
      }
      at = region.parent;
    }
    None
  }

  /// The value of `name` (`names[index]` of the activation's program) in the scope of `region`
  /// of the activation `act`.
  fn region_value(&self, act: &Activation, region: Region, name: &Name, index: u32) -> Option<Value> {
    match region.kind {
      RegionKind::Layout { layout, visible } => act.program.layouts[layout as usize].find(index, visible).map(|place| self.place(act, place)),
      // while the filter visits its items: the item, then the item's entries
      RegionKind::Item { record } => {
        let filter = self.filter_of(act.records, record);
        let item = filter.item()?;
        if !filter.item_entry && *name == *ITEM {
          Some(item.clone())
        } else if let Value::Context(context) = item {
          context.get_entry(name).cloned()
        } else {
          None
        }
      }
      RegionKind::Closure => {
        let function = act.function.as_ref()?;
        self.at_home(function, name).or_else(|| function.closure_ctx.get_entry(name).cloned())
      }
    }
  }

  /// The step of the filter of `record` after its predicate gave `c`: evaluated in the scope around the filter, a number selects
  /// the item at that index; any other value is not counted (what it recorded is removed) and the
  /// predicate is evaluated for each item, unless the evaluation was stopped, keeping the items
  /// for which it is `true`, up to a value that is no boolean, number or null, which ends the
  /// filter with its null. Returns where the run continues: the predicate, or the filter's close.
  fn filter_step(&mut self, program: &Program, record: u32, c: Value) -> Pc {
    let filter = self.filter_mut(record);
    let info = &program.filters[filter.info as usize];
    if filter.visiting {
      match outcome(c) {
        Ok(true) => {
          let item = filter.items[filter.at].clone();
          filter.kept.push(item);
        }
        Ok(false) => {}
        Err(null) => {
          filter.value = Some(null);
          return info.close;
        }
      }
      filter.at += 1;
    } else {
      match c {
        Value::Number(index) => {
          filter.mark = None;
          filter.value = Some(if diagnostics::stopped() { Value::Null(None) } else { item_at(&filter.items, index) });
          return info.close;
        }
        _ => {
          if let Some(mark) = filter.mark.take() {
            mark.discard();
          }
          filter.visiting = true;
        }
      }
    }
    // an item is a step of the evaluation; after the last one the result is given unless the
    // evaluation was stopped
    let stopped = match filter.items.get(filter.at) {
      Some(_) => diagnostics::step(),
      None => diagnostics::stopped(),
    };
    if stopped {
      filter.value = Some(Value::Null(None));
      return info.close;
    }
    match filter.items.get(filter.at) {
      Some(item) => {
        filter.item_entry = matches!(item, Value::Context(context) if context.contains_entry(&*ITEM));
        info.body
      }
      None => {
        filter.value = Some(Value::List(std::mem::take(&mut filter.kept).into()));
        info.close
      }
    }
  }

  /// The next step of the walk of `record`:
  /// while a variable has a domain, unless the evaluation was stopped, the next value of the
  /// innermost one, where the code that follows it starts; a domain that has no value left is
  /// removed, with the value of its variable. Without a domain the walk ends.
  fn step(&mut self, program: &Program, record: u32) -> Pc {
    let base = self.act.base;
    let Self {
      records,
      domains,
      bottom,
      stack,
      act,
      ..
    } = self;
    let Record::Walk(walk) = &mut records[act.records + record as usize] else {
      unreachable!("the record of a walk is a filter");
    };
    let slots: &mut [Value] = match base {
      BOTTOM => bottom,
      base => &mut stack[base..],
    };
    let info = &program.loops[walk.info as usize];
    loop {
      let Some(level) = (domains.len() - walk.base).checked_sub(1) else {
        return info.exit;
      };
      // each visit of a level is a step of the evaluation
      if diagnostics::step() {
        walk.value = Some(Value::Null(None));
        return info.exit;
      }
      let (slot, bind) = info.levels[level];
      match domains.last_mut().and_then(Sequence::next_item) {
        Some(value) => {
          slots[slot as usize] = value;
          return bind;
        }
        None => {
          domains.pop();
          slots[slot as usize] = Value::Null(None);
        }
      }
    }
  }

  /// The value of a binder of the activation `act`.
  fn place(&self, act: &Activation, place: Place) -> Value {
    match place {
      Place::Slot(slot) => self.slot_of(act.base, slot).clone(),
      Place::Partial(record) => match self.record(act.records, record) {
        Record::Walk(walk) => Value::List(walk.results.clone()),
        Record::Filter(_) => Value::Null(None),
      },
    }
  }

  /// The activation at `depth`: the one that runs at the depth of [Vm::frames], the one of a
  /// frame below.
  fn activation(&self, depth: usize) -> &Activation {
    match self.frames.get(depth) {
      Some(frame) => &frame.activation,
      None => &self.act,
    }
  }

  /// Pushes on the scope of the host the scopes of a position
  /// whose innermost region is `region` of the activation that runs (and so those of the
  /// activations below it, where each invoked the one above), outermost first; returns the depth
  /// of the scope before. A scope without entries is left out: it changes no read. [Vm::release]
  /// ends the invocation.
  #[cold]
  #[inline(never)]
  fn materialize(&mut self, region: u32) -> usize {
    let depth = self.host.depth();
    let bottom = Arc::clone(self.bottom_program());
    let mut scopes = std::mem::take(&mut self.scopes);
    if scopes.is_empty() {
      scopes.resize(bottom.regions.len(), None);
    }
    let mut layout_scopes = std::mem::take(&mut self.layout_scopes);
    if layout_scopes.is_empty() {
      layout_scopes.resize(bottom.layouts.len(), None);
    }
    let mut chain = std::mem::take(&mut self.chain);
    chain.clear();
    let mut at = region;
    for act in (0..=self.frames.len()).rev() {
      if act < self.frames.len() {
        let frame = &self.frames[act];
        at = frame.activation.program.calls[frame.call as usize].region;
      }
      let program = &self.activation(act).program;
      while at != NONE {
        chain.push((act, at));
        at = program.regions[at as usize].parent;
      }
    }
    for &(act, at) in chain.iter().rev() {
      let activation = self.activation(act);
      let program = &activation.program;
      // the scopes of the program the run starts with are kept for the run
      let kept = Arc::ptr_eq(program, &bottom);
      match program.regions[at as usize].kind {
        RegionKind::Layout { layout, visible } => {
          let mut entries = program.layouts[layout as usize].visible(visible).peekable();
          if entries.peek().is_none() {
            continue;
          }
          // the scope the layout has from an invocation before, unless that one saw entries
          // that this region does not see
          // (a region of another program gets a scope of its own, which is not kept)
          let mut scope = match kept.then(|| layout_scopes[layout as usize].take()).flatten() {
            Some((scope, seen)) if seen <= visible => scope,
            _ => Arc::default(),
          };
          let context = Arc::make_mut(&mut scope);
          for &(name, place) in entries {
            context.assign_entry(&program.names[name as usize], self.place(activation, place));
          }
          self.host.push_shared(Arc::clone(&scope));
          if kept {
            layout_scopes[layout as usize] = Some((scope, visible));
          }
        }
        // while the filter visits its items: the item's entries, then the item
        RegionKind::Item { record } => {
          let filter = self.filter_of(activation.records, record);
          let Some(item) = filter.item() else {
            continue;
          };
          if let Value::Context(context) = item {
            self.host.push_shared(context.clone());
          }
          if !filter.item_entry {
            let item = item.clone();
            let mut scope = if kept { scopes[at as usize].take().unwrap_or_default() } else { Arc::default() };
            Arc::make_mut(&mut scope).assign_entry(&ITEM, item);
            self.host.push_shared(Arc::clone(&scope));
            if kept {
              scopes[at as usize] = Some(scope);
            }
          }
        }
        RegionKind::Closure => {
          if let Some(function) = &activation.function
            && !function.closure_ctx.is_empty()
          {
            self.host.push_shared(Arc::clone(&function.closure_ctx));
          }
        }
      }
    }
    self.chain = chain;
    self.scopes = scopes;
    self.layout_scopes = layout_scopes;
    depth
  }

  /// Ends the invocation of [Vm::materialize]: the values of the scopes it kept, which the scope of
  /// the host no longer holds, are set back to null, so that no value is held twice (a list held
  /// twice is copied when it grows).
  #[cold]
  #[inline(never)]
  fn release(&mut self) {
    let bottom = Arc::clone(self.bottom_program());
    for index in 0..self.chain.len() {
      let (act, at) = self.chain[index];
      if !Arc::ptr_eq(&self.activation(act).program, &bottom) {
        continue;
      }
      match bottom.regions[at as usize].kind {
        RegionKind::Layout { layout, visible } => {
          if let Some((scope, _)) = &mut self.layout_scopes[layout as usize] {
            let context = Arc::make_mut(scope);
            for &(name, _) in bottom.layouts[layout as usize].visible(visible) {
              context.assign_entry(&bottom.names[name as usize], Value::Null(None));
            }
          }
        }
        RegionKind::Item { .. } => {
          if let Some(scope) = &mut self.scopes[at as usize] {
            Arc::make_mut(scope).assign_entry(&ITEM, Value::Null(None));
          }
        }
        RegionKind::Closure => {}
      }
    }
  }

  /// The value of the qualified name `segments`, whose first segment is `names[first]` of the
  /// program that runs ([NONE] when no binder has it), from the region `region` of the activation
  /// that runs down: in each scope, innermost first, the entry of the first segment and in it the
  /// entries of the others; a search that
  /// fails in one scope goes on in the scope below it, down to the scope of the host.
  #[cold]
  #[inline(never)]
  fn search_from(&self, region: u32, first: u32, segments: &[Name]) -> Option<Value> {
    let Some(head) = segments.first() else {
      return self.host.search(segments);
    };
    let program = &self.act.program;
    self
      .scopes_below(region, |vm, act, region| {
        let first = if Arc::ptr_eq(&act.program, program) {
          first
        } else {
          vm.name_index(&act.program, Some(head))
        };
        match region.kind {
          RegionKind::Layout { layout, visible } => act.program.layouts[layout as usize]
            .find(first, visible)
            .and_then(|place| search_in(&vm.place(act, place), &segments[1..])),
          // while the filter visits its items: the item, then the item's entries
          RegionKind::Item { record } => {
            let filter = vm.filter_of(act.records, record);
            filter.item().and_then(|item| {
              let special = (!filter.item_entry && *head == *ITEM).then(|| search_in(item, &segments[1..])).flatten();
              special.or_else(|| match item {
                Value::Context(context) => context.search_deep(segments).cloned(),
                _ => None,
              })
            })
          }
          RegionKind::Closure => {
            let function = act.function.as_ref()?;
            vm.at_home(function, head)
              .and_then(|value| search_in(&value, &segments[1..]))
              .or_else(|| function.closure_ctx.search_deep(segments).cloned())
          }
        }
      })
      .or_else(|| self.host.search(segments))
  }
}

/// The entry at the path `names` in `value`, `value` itself for no names: each name but the last
/// names a context.
fn search_in(value: &Value, names: &[Name]) -> Option<Value> {
  match (value, names) {
    (_, []) => Some(value.clone()),
    (Value::Context(context), names) => context.search_deep(names).cloned(),
    _ => None,
  }
}
