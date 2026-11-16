//! The nesting depth that lists and contexts keep, against a reference that walks the whole value.

use crate::closure::Closure;
use crate::context::FeelContext;
use crate::values::{FeelList, Value};
use crate::{FeelScope, FeelType, FunctionBody, FunctionDefinition, IntervalType, Name, QualifiedName, value_null, value_number};
use std::sync::Arc;

/// The nesting depth by its definition, walking every value it holds.
fn reference(value: &Value) -> u32 {
  fn deepest<'a>(values: impl Iterator<Item = &'a Value>) -> u32 {
    values.map(reference).max().unwrap_or(0) + 1
  }
  match value {
    Value::List(items) => deepest(items.iter()),
    Value::Context(ctx) => deepest(ctx.iter().map(|(_, value)| value)),
    Value::FunctionDefinition(function) => reference(&Value::from(function.closure_ctx.clone())) + 1,
    Value::Range(start, _, end, _) => deepest([start.as_ref(), end.as_ref()].into_iter()),
    Value::UnaryLess(operand) | Value::IntervalStart(operand, _) => reference(operand) + 1,
    _ => 0,
  }
}

/// A small deterministic generator (xorshift).
struct Rng(u64);

impl Rng {
  fn below(&mut self, n: u64) -> u64 {
    self.0 ^= self.0 << 13;
    self.0 ^= self.0 >> 7;
    self.0 ^= self.0 << 17;
    self.0 % n
  }

  fn name(&mut self) -> Name {
    ["a", "b", "c", "d", "e"][self.below(5) as usize].into()
  }

  fn names(&mut self) -> Vec<Name> {
    (0..1 + self.below(3)).map(|_| self.name()).collect()
  }
}

fn function(closure_ctx: FeelContext) -> Value {
  Value::from(FunctionDefinition::new(
    vec![],
    FunctionBody::LiteralExpression(Arc::new(Box::new(|_: &FeelScope| value_number!(1)))),
    false,
    Closure::default(),
    closure_ctx,
    FeelType::Any,
  ))
}

/// A value of at most about `depth` levels, scalars more often than containers.
fn value(rng: &mut Rng, depth: u32) -> Value {
  if depth == 0 {
    return value_number!(1);
  }
  match rng.below(12) {
    0 => value_null!(),
    1 => value_number!(2),
    2 => Value::String("s".into()),
    3 => Value::List(FeelList::new()),
    4 | 5 => Value::List((0..rng.below(4)).map(|_| value(rng, depth - 1)).collect()),
    6 => {
      let mut list = FeelList::new();
      for _ in 0..rng.below(4) {
        list.push(value(rng, depth - 1));
      }
      Value::List(list)
    }
    7 | 8 => Value::from(context(rng, depth - 1)),
    9 => function(context(rng, depth - 1)),
    10 => Value::Range(Box::new(value(rng, depth - 1)), IntervalType::Closed, Box::new(value_number!(1)), IntervalType::Opened),
    _ => Value::UnaryLess(Box::new(value(rng, depth - 1))),
  }
}

fn context(rng: &mut Rng, depth: u32) -> FeelContext {
  let mut ctx = FeelContext::default();
  for _ in 0..rng.below(4) {
    let name = rng.name();
    ctx.set_entry(&name, value(rng, depth));
  }
  ctx
}

/// Changes `ctx` by one of the methods that write an entry.
fn change(rng: &mut Rng, ctx: &mut FeelContext) {
  let name = rng.name();
  // values at most five levels deep, so that a change makes the context deeper as often as shallower
  let depth = rng.below(6) as u32;
  match rng.below(12) {
    0 => ctx.set_entry(&name, value(rng, depth)),
    1 => ctx.insert(name, value(rng, depth)),
    2 => {
      ctx.assign_entry(&name, value(rng, depth));
    }
    3 | 4 => {
      ctx.remove_entry(&name);
    }
    5 => ctx.set_null(name),
    6 => ctx.zip(&context(rng, depth)),
    7 => ctx.overwrite(&context(rng, depth)),
    8 => ctx.move_entry(name, rng.name()),
    9 => ctx.create_entries(&rng.names(), value(rng, depth)),
    10 => ctx.create_entry(&QualifiedName::new(&rng.names().iter().collect::<Vec<_>>()), value(rng, depth)),
    _ => {
      // an error leaves the depth as exact as a success
      let _ = ctx.apply_entries(&rng.names(), value(rng, depth));
    }
  }
}

#[test]
fn a_context_keeps_its_depth_through_every_change() {
  for seed in 1..=300u64 {
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let mut ctx = context(&mut rng, 4);
    for step in 0..200 {
      change(&mut rng, &mut ctx);
      let expected = reference(&Value::from(ctx.clone()));
      assert_eq!(expected, ctx.nesting_depth(), "seed {seed}, step {step}: {ctx}");
      assert_eq!(expected, Value::from(ctx.clone()).nesting_depth());
    }
  }
}

#[test]
fn a_list_keeps_its_depth_however_it_is_built() {
  for seed in 1..=300u64 {
    let mut rng = Rng(seed.wrapping_mul(0x2545_F491_4F6C_DD1D));
    let items = (0..rng.below(6)).map(|_| value(&mut rng, 5)).collect::<Vec<_>>();
    let mut pushed = FeelList::new();
    for item in &items {
      pushed.push(item.clone());
      assert_eq!(reference(&Value::List(pushed.clone())), pushed.nesting_depth());
    }
    let from_vec = FeelList::from(items.clone());
    let collected = items.into_iter().collect::<FeelList>();
    let expected = reference(&Value::List(from_vec.clone()));
    assert_eq!(expected, from_vec.nesting_depth(), "seed {seed}");
    assert_eq!(expected, collected.nesting_depth());
    assert_eq!(expected, pushed.nesting_depth());
    assert_eq!(expected, Value::List(pushed).nesting_depth());
  }
}

#[test]
fn the_depth_of_a_context_falls_when_its_deepest_entry_goes() {
  let a = Name::from("a");
  let b = Name::from("b");
  let deep = || Value::List(vec![Value::List(vec![value_number!(1)].into())].into());
  let mut ctx = FeelContext::default();
  assert_eq!(1, ctx.nesting_depth());
  ctx.set_entry(&a, deep());
  ctx.set_entry(&b, Value::List(FeelList::new()));
  assert_eq!(3, ctx.nesting_depth());
  // replaced by a shallower value
  assert_eq!(Some(deep()), ctx.assign_entry(&a, value_number!(1)));
  assert_eq!(2, ctx.nesting_depth());
  // removed
  ctx.remove_entry(&b);
  assert_eq!(1, ctx.nesting_depth());
  // a nested entry replaced through a path
  ctx.create_entries(&[b.clone(), a.clone()], deep());
  assert_eq!(4, ctx.nesting_depth());
  ctx.create_entries(&[b.clone(), a.clone()], value_number!(1));
  assert_eq!(2, ctx.nesting_depth());
  ctx.apply_entries(&[b.clone(), a.clone()], deep()).unwrap();
  assert_eq!(4, ctx.nesting_depth());
  ctx.apply_entries(&[b.clone(), b.clone()], value_number!(1)).unwrap();
  assert_eq!(4, ctx.nesting_depth());
  ctx.apply_entries(&[b.clone(), a.clone()], value_null!()).unwrap();
  assert_eq!(2, ctx.nesting_depth());
  // moved under a new parent: one level deeper
  ctx.move_entry(b.clone(), Name::from("c"));
  assert_eq!(3, ctx.nesting_depth());
}

#[test]
fn the_depth_of_scalars_and_operands() {
  assert_eq!(0, value_number!(1).nesting_depth());
  assert_eq!(0, value_null!().nesting_depth());
  assert_eq!(1, Value::List(FeelList::new()).nesting_depth());
  assert_eq!(1, Value::from(FeelContext::default()).nesting_depth());
  assert_eq!(2, function(FeelContext::default()).nesting_depth());
  assert_eq!(
    1,
    Value::Range(Box::new(value_number!(1)), IntervalType::Closed, Box::new(value_number!(2)), IntervalType::Closed).nesting_depth()
  );
  assert_eq!(2, Value::UnaryLess(Box::new(Value::List(FeelList::new()))).nesting_depth());
  assert_eq!(2, Value::ExpressionList(vec![Value::List(FeelList::new())]).nesting_depth());
}
