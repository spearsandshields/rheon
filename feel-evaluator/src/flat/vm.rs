//! # The run loop
//!
//! One evaluation of a [Program]: its frame of slots, and one loop that runs the instructions in
//! order. Every value is computed by the kernels that the evaluator of closures calls, in the
//! order it calls them, and every error, warning and limit is recorded in the evaluation's
//! diagnostics as that evaluator records it.

use super::program::{Compare, Domain, Kernel1, Kernel2, ListKind, LoopKind, NONE, Op, Operand, Opnd, Pc, Program};
use super::resolve::{Place, RegionKind};
use crate::MAX_CALL_DEPTH;
use crate::bfeel;
use crate::filters::{ITEM, item_at, outcome};
use crate::invocation::{Arguments, invoke};
use crate::iterations::Sequence;
use crate::kernels::{self, Arithmetic, finite};
use rheon_feel::context::FeelContext;
use rheon_feel::values::{FeelList, Value};
use rheon_feel::{FeelScope, FeelType, Name, diagnostics, value_null};
use std::collections::BTreeMap;
use std::sync::Arc;

/// The slots of a frame that a run keeps on the stack of the host instead of allocating them.
const INLINE_SLOTS: usize = 4;

impl Program {
  /// Evaluates the program in `scope`.
  pub(crate) fn run(self: &Arc<Self>, scope: &FeelScope) -> Value {
    let size = self.slots as usize;
    if size <= INLINE_SLOTS {
      let mut slots: [Value; INLINE_SLOTS] = std::array::from_fn(|_| Value::Null(None));
      Vm::new(self, scope, &mut slots[..size]).execute()
    } else {
      let mut slots = vec![Value::Null(None); size];
      Vm::new(self, scope, &mut slots).execute()
    }
  }
}

/// The state of one run of a program.
struct Vm<'p> {
  program: &'p Program,
  host: &'p FeelScope,
  slots: &'p mut [Value],
  /// The lists being built, innermost last.
  lists: Vec<Vec<Value>>,
  /// The context types being built, innermost last.
  types: Vec<BTreeMap<Name, FeelType>>,
  /// The invocations entered, innermost last.
  calls: Vec<diagnostics::Call>,
  /// The open walks, innermost last: the record of a walk is its index.
  records: Vec<Record>,
  /// The domains of the variables that have a value in the open walks, innermost last.
  domains: Vec<Sequence>,
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

impl<'p> Vm<'p> {
  fn new(program: &'p Program, host: &'p FeelScope, slots: &'p mut [Value]) -> Self {
    Self {
      program,
      host,
      slots,
      lists: vec![],
      types: vec![],
      calls: vec![],
      records: vec![],
      domains: vec![],
    }
  }

  /// The walk of `record`, open by the structure of the code ([Program::verify]).
  fn walk(&mut self, record: u32) -> &mut Walk {
    match &mut self.records[record as usize] {
      Record::Walk(walk) => walk,
      Record::Filter(_) => unreachable!("the record of a walk is a filter"),
    }
  }

  /// The filter of `record`, open by the structure of the code ([Program::verify]).
  fn filter(&self, record: u32) -> &Filter {
    match &self.records[record as usize] {
      Record::Filter(filter) => filter,
      Record::Walk(_) => unreachable!("the record of a filter is a walk"),
    }
  }

  /// The filter of `record`, to change.
  fn filter_mut(&mut self, record: u32) -> &mut Filter {
    match &mut self.records[record as usize] {
      Record::Filter(filter) => filter,
      Record::Walk(_) => unreachable!("the record of a filter is a walk"),
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
        Op::Search { dst, search } => self.slots[dst as usize] = self.search(search as usize).unwrap_or_else(kernels::free_qualified_name),
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
            Kernel1::ListType => kernels::list_type(a),
            Kernel1::RangeType => kernels::range_type(a),
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
            Kernel2::InstanceOf => kernels::instance_of(a, b),
            Kernel2::ContextTypeEntry => kernels::context_type_entry(a, b),
            Kernel2::FunctionType => kernels::function_type(a, b),
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
            ListKind::ParameterTypes => Value::ParameterTypes(items),
          };
        }
        Op::TypeNew => self.types.push(BTreeMap::new()),
        Op::TypeAdd { src, dst, fail } => {
          let entry = self.read(src);
          if let Some(entries) = self.types.last_mut()
            && let Err(null) = kernels::add_context_type_entry(entries, entry)
          {
            self.types.pop();
            self.slots[dst as usize] = null;
            pc = fail as usize;
          }
        }
        Op::TypeEnd { dst } => {
          let entries = self.types.pop().unwrap_or_default();
          self.slots[dst as usize] = Value::FeelType(FeelType::Context(entries));
        }
        Op::Enter { dst, skip } => match diagnostics::enter_call(MAX_CALL_DEPTH) {
          Some(call) => self.calls.push(call),
          None => {
            self.slots[dst as usize] = Value::Null(None);
            pc = skip as usize;
          }
        },
        Op::Call { dst, call } => {
          let call = &program.calls[call as usize];
          let function = self.read(call.function);
          let values: Vec<Value> = (call.args..call.args + call.argc).map(|slot| self.take(slot as usize)).collect();
          let arguments = if call.names == NONE {
            Arguments::Positional(values)
          } else {
            Arguments::Named(program.name_lists[call.names as usize].iter().cloned().zip(values).collect())
          };
          // a function of the host reads the scopes of the position through the scope it gets
          self.slots[dst as usize] = if call.region != NONE && matches!(function, Value::FunctionDefinition(_)) {
            let depth = self.materialize(call.region);
            let value = invoke(self.host, function, arguments, program.dialect);
            self.host.truncate(depth);
            value
          } else {
            invoke(self.host, function, arguments, program.dialect)
          };
        }
        Op::Leave { dst } => {
          let value = diagnostics::bound_depth(finite(self.take(dst as usize)));
          if let Some(call) = self.calls.pop() {
            self.slots[dst as usize] = call.finish(value);
          }
        }
        Op::BoundDepth { s } => {
          let value = self.take(s as usize);
          self.slots[s as usize] = diagnostics::bound_depth(value);
        }
        Op::Clear { layout, visible } => {
          for &(_, place) in &program.layouts[layout as usize].entries[..visible as usize] {
            if let Place::Slot(slot) = place {
              self.slots[slot as usize] = Value::Null(None);
            }
          }
        }
        Op::Context { dst, layout } => {
          let mut context = FeelContext::default();
          for &(name, place) in program.layouts[layout as usize].entries.iter() {
            if let Place::Slot(slot) = place {
              let value = self.take(slot as usize);
              context.set_entry(&program.names[name as usize], value);
            }
          }
          self.slots[dst as usize] = diagnostics::bound_depth(context.into());
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
        Op::Step { record } => pc = self.step(record) as usize,
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
              self.slots[filter.dst as usize] = value_null!();
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
          pc = self.filter_step(record, c) as usize;
        }
        Op::FilterClose { record: _ } => {
          if let Some(Record::Filter(filter)) = self.records.pop() {
            let info = &program.filters[filter.info as usize];
            let value = filter.value.unwrap_or(Value::Null(None));
            self.slots[info.dst as usize] = if filter.wrap { diagnostics::bound_depth(value) } else { value };
          }
        }
        Op::ProbeItem { dst, probe, found } => {
          let probe = program.probes[probe as usize];
          if let Some(Value::Context(item)) = self.filter(probe.record).item()
            && let Some(value) = item.get_entry(&program.names[probe.name as usize])
          {
            self.slots[dst as usize] = value.clone();
            pc = found as usize;
          }
        }
        Op::ItemSelf { dst, record, found } => {
          let filter = self.filter(record);
          if let Some(item) = filter.item() {
            let value = match item {
              Value::Context(context) if filter.item_entry => context.get_entry(&ITEM).cloned().unwrap_or(Value::Null(None)),
              item => item.clone(),
            };
            self.slots[dst as usize] = value;
            pc = found as usize;
          }
        }
        Op::LoopClose { record: _, dst } => {
          if let Some(Record::Walk(walk)) = self.records.pop() {
            let info = &program.loops[walk.info as usize];
            self.domains.truncate(walk.base);
            for &(slot, _) in info.levels.iter() {
              self.slots[slot as usize] = Value::Null(None);
            }
            self.slots[dst as usize] = walk.value.unwrap_or_else(|| match info.kind {
              LoopKind::For => Value::List(walk.results),
              LoopKind::Some => Value::Boolean(false),
              LoopKind::Every => Value::Boolean(true),
            });
          }
        }
        Op::LoadPartial { dst, record } => self.slots[dst as usize] = Value::List(self.walk(record).results.clone()),
        Op::Return { src } => return self.read(src),
      }
    }
  }

  /// The step of the filter of `record` after its predicate gave `c`, as the filter of the
  /// evaluator of closures takes it: evaluated in the scope around the filter, a number selects
  /// the item at that index; any other value is not counted (what it recorded is removed) and the
  /// predicate is evaluated for each item, unless the evaluation was stopped, keeping the items
  /// for which it is `true`, up to a value that is no boolean, number or null, which ends the
  /// filter with its null. Returns where the run continues: the predicate, or the filter's close.
  fn filter_step(&mut self, record: u32, c: Value) -> Pc {
    let info = &self.program.filters[self.filter(record).info as usize];
    let filter = self.filter_mut(record);
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
    if diagnostics::stopped() {
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

  /// The next step of the walk of `record`, as the walk of the evaluator of closures takes it:
  /// while a variable has a domain, unless the evaluation was stopped, the next value of the
  /// innermost one, where the code that follows it starts; a domain that has no value left is
  /// removed, with the value of its variable. Without a domain the walk ends.
  fn step(&mut self, record: u32) -> Pc {
    let program = self.program;
    let Self { records, domains, slots, .. } = self;
    let Record::Walk(walk) = &mut records[record as usize] else {
      unreachable!("the record of a walk is a filter");
    };
    let info = &program.loops[walk.info as usize];
    loop {
      let Some(level) = (domains.len() - walk.base).checked_sub(1) else {
        return info.exit;
      };
      if diagnostics::stopped() {
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

  /// The value of a binder.
  fn place(&self, place: Place) -> Value {
    match place {
      Place::Slot(slot) => self.slots[slot as usize].clone(),
      Place::Partial(record) => match &self.records[record as usize] {
        Record::Walk(walk) => Value::List(walk.results.clone()),
        Record::Filter(_) => Value::Null(None),
      },
    }
  }

  /// Pushes on the scope of the host the scopes that the evaluator of closures has at a position
  /// whose innermost region is `region`, outermost first; returns the depth of the scope before.
  /// A scope without entries is left out: it changes no read.
  #[cold]
  #[inline(never)]
  fn materialize(&self, region: u32) -> usize {
    let depth = self.host.depth();
    let mut chain = vec![];
    let mut at = region;
    while at != NONE {
      chain.push(at);
      at = self.program.regions[at as usize].parent;
    }
    for &at in chain.iter().rev() {
      match self.program.regions[at as usize].kind {
        RegionKind::Layout { layout, visible } => {
          let mut context = FeelContext::default();
          for &(name, place) in self.program.layouts[layout as usize].visible(visible) {
            context.set_entry(&self.program.names[name as usize], self.place(place));
          }
          if !context.is_empty() {
            self.host.push(context);
          }
        }
        // while the filter visits its items: the item's entries, then the item
        RegionKind::Item { record } => {
          let filter = self.filter(record);
          if let Some(item) = filter.item() {
            if let Value::Context(context) = item {
              self.host.push_shared(context.clone());
            }
            if !filter.item_entry {
              let mut context = FeelContext::default();
              context.set_entry(&ITEM, item.clone());
              self.host.push(context);
            }
          }
        }
        RegionKind::Closure => {}
      }
    }
    depth
  }

  /// The value of the qualified name of `searches[search]`: in each scope of its position,
  /// innermost first, the entry of the first segment and in it the entries of the others, as the
  /// scopes of the evaluator of closures search it; a search that fails in one scope goes on in
  /// the scope below it, down to the scope of the host.
  #[cold]
  #[inline(never)]
  fn search(&self, search: usize) -> Option<Value> {
    let info = self.program.searches[search];
    let segments = &self.program.name_lists[info.segments as usize];
    let mut at = if info.first == NONE { NONE } else { info.region };
    while at != NONE {
      let region = self.program.regions[at as usize];
      let found = match region.kind {
        RegionKind::Layout { layout, visible } => self.program.layouts[layout as usize]
          .find(info.first, visible)
          .and_then(|place| search_in(&self.place(place), &segments[1..])),
        // while the filter visits its items: the item, then the item's entries
        RegionKind::Item { record } => {
          let filter = self.filter(record);
          filter.item().and_then(|item| {
            let special = (!filter.item_entry && segments[0] == *ITEM).then(|| search_in(item, &segments[1..])).flatten();
            special.or_else(|| match item {
              Value::Context(context) => context.search_deep(segments).cloned(),
              _ => None,
            })
          })
        }
        RegionKind::Closure => None,
      };
      if found.is_some() {
        return found;
      }
      at = region.parent;
    }
    self.host.search(segments)
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
