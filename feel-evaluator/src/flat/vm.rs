//! # The run loop
//!
//! One evaluation of a [Program]: its frame of slots, and one loop that runs the instructions in
//! order. Every value is computed by the kernels that the evaluator of closures calls, in the
//! order it calls them, and every error, warning and limit is recorded in the evaluation's
//! diagnostics as that evaluator records it.

use super::program::{Compare, Kernel1, Kernel2, ListKind, NONE, Op, Operand, Opnd, Program};
use super::resolve::{Place, RegionKind};
use crate::MAX_CALL_DEPTH;
use crate::bfeel;
use crate::invocation::{Arguments, invoke};
use crate::kernels::{self, Arithmetic, finite};
use rheon_feel::context::FeelContext;
use rheon_feel::values::Value;
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
        Op::Return { src } => return self.read(src),
      }
    }
  }

  /// The value of a binder.
  fn place(&self, place: Place) -> Value {
    match place {
      Place::Slot(slot) => self.slots[slot as usize].clone(),
      Place::Partial(_) => Value::Null(None),
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
        RegionKind::Item { .. } | RegionKind::Closure => {}
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
      if let RegionKind::Layout { layout, visible } = region.kind
        && let Some(place) = self.program.layouts[layout as usize].find(info.first, visible)
        && let Some(value) = search_in(&self.place(place), &segments[1..])
      {
        return Some(value);
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
