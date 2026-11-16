//! The relations between the type of a value and a type, decided on the value, against the same
//! relations on the type that [Value::type_of] builds.

use crate::closure::Closure;
use crate::context::FeelContext;
use crate::values::{FeelList, Value};
use crate::{FeelScope, FeelType, FunctionBody, FunctionDefinition, IntervalType, Name, value_number};
use std::collections::BTreeMap;
use std::sync::Arc;

/// A small deterministic generator (xorshift).
struct Rng(u64);

impl Rng {
  fn below(&mut self, n: u64) -> u64 {
    self.0 ^= self.0 << 13;
    self.0 ^= self.0 >> 7;
    self.0 ^= self.0 << 17;
    self.0 % n
  }

  fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
    &items[self.below(items.len() as u64) as usize]
  }

  fn name(&mut self) -> Name {
    Name::from(*self.pick(&["a", "b", "c"]))
  }
}

fn function(parameters: Vec<FeelType>, result_type: FeelType) -> Value {
  Value::from(FunctionDefinition::new(
    parameters.into_iter().enumerate().map(|(i, feel_type)| (Name::from(format!("p{i}")), feel_type)).collect(),
    FunctionBody::LiteralExpression(Arc::new(Box::new(|_: &FeelScope| value_number!(1)))),
    false,
    Closure::default(),
    FeelContext::default(),
    result_type,
  ))
}

/// A random type of at most about `depth` levels, built from the simple types and `Any`.
fn feel_type(rng: &mut Rng, depth: u32) -> FeelType {
  let simple = [FeelType::Any, FeelType::Null, FeelType::Number, FeelType::String, FeelType::Boolean, FeelType::Date];
  if depth == 0 {
    return rng.pick(&simple).clone();
  }
  match rng.below(9) {
    0 => FeelType::List(Box::new(feel_type(rng, depth - 1))),
    1 => FeelType::Range(Box::new(feel_type(rng, depth - 1))),
    2 => {
      let mut entries = BTreeMap::new();
      for _ in 0..rng.below(3) {
        entries.insert(rng.name(), feel_type(rng, depth - 1));
      }
      FeelType::Context(entries)
    }
    3 => FeelType::Function((0..rng.below(3)).map(|_| feel_type(rng, depth - 1)).collect(), Box::new(feel_type(rng, depth - 1))),
    _ => rng.pick(&simple).clone(),
  }
}

/// Values built from the values before them: scalars, lists and contexts of earlier values (so
/// earlier lists are shared), ranges, functions, types, interval ends and unary tests.
fn values(rng: &mut Rng) -> Vec<Value> {
  let mut values = vec![
    value_number!(1),
    value_number!(2),
    Value::String("a".into()),
    Value::Null(None),
    Value::Boolean(true),
    Value::Irrelevant,
  ];
  for _ in 0..16 {
    let value = match rng.below(12) {
      0 => {
        let mut ctx = FeelContext::default();
        for _ in 0..rng.below(3) {
          ctx.set_entry(&rng.name(), rng.pick(&values).clone());
        }
        Value::from(ctx)
      }
      1 => Value::Range(
        Box::new(rng.pick(&values).clone()),
        IntervalType::Closed,
        Box::new(rng.pick(&values).clone()),
        IntervalType::Opened,
      ),
      2 => {
        let depth = rng.below(3) as u32;
        let parameters = (0..rng.below(3)).map(|_| feel_type(rng, depth)).collect();
        function(parameters, feel_type(rng, depth))
      }
      3 => Value::FeelType(feel_type(rng, 2)),
      4 => Value::IntervalStart(Box::new(rng.pick(&values).clone()), IntervalType::Closed),
      5 => Value::UnaryLess(Box::new(rng.pick(&values).clone())),
      _ => Value::List((0..rng.below(4)).map(|_| rng.pick(&values).clone()).collect::<FeelList>()),
    };
    values.push(value);
  }
  values
}

/// Every relation answers as on the built type, for random values that share their lists and for
/// types built from values (which match often) and at random.
#[test]
fn the_relations_decided_on_values_are_those_of_the_built_types() {
  let mut rng = Rng(0x5851_f42d_4c95_7f2d);
  let (mut same, mut instances) = (0, 0);
  for _ in 0..300 {
    let values = values(&mut rng);
    let mut types: Vec<FeelType> = values.iter().map(Value::type_of).collect();
    types.extend((0..10).map(|_| feel_type(&mut rng, 3)));
    for value in &values {
      let value_type = value.type_of();
      for feel_type in &types {
        assert_eq!(value_type == *feel_type, value.has_type(feel_type), "type of {value:?} is {feel_type:?}");
        assert_eq!(value_type.instance_of(feel_type), value.type_instance_of(feel_type), "{value:?} instance of {feel_type:?}");
        same += usize::from(value_type == *feel_type);
        instances += usize::from(value_type.instance_of(feel_type));
      }
      for other in &values {
        assert_eq!(value_type == other.type_of(), value.has_type_of(other), "type of {value:?} is that of {other:?}");
      }
    }
  }
  // the cases cover the answer true of each relation, not only false
  assert!(same > 10_000 && instances > 1_000, "{same} types equal, {instances} instances");
}

/// The list `[x, x]` built from the one before it `n - 1` times, starting with `[1]`; its type is
/// a list nested `n` levels deep around `number`.
pub(super) fn doubled(n: u32) -> Value {
  let mut list = Value::List(FeelList::from(vec![value_number!(1)]));
  for _ in 1..n {
    list = Value::List(FeelList::from(vec![list.clone(), list]));
  }
  list
}

pub(super) fn nested_list_type(levels: u32, item: FeelType) -> FeelType {
  (0..levels).fold(item, |item, _| FeelType::List(Box::new(item)))
}

/// A list that holds the same list on 2^59 paths is related to a type at once.
#[test]
fn the_relations_visit_a_shared_list_once() {
  let list = doubled(60);
  assert!(list.has_type(&nested_list_type(60, FeelType::Number)));
  assert!(!list.has_type(&nested_list_type(60, FeelType::String)));
  assert!(list.has_type_of(&doubled(60)));
  assert!(!list.has_type_of(&doubled(59)));
  assert!(list.type_instance_of(&nested_list_type(60, FeelType::Number)));
  assert!(list.type_instance_of(&nested_list_type(1, FeelType::Any)));
  assert!(!list.type_instance_of(&nested_list_type(59, FeelType::Number)));
  // `b` is doubled as `a` is, except that its last item is a string: the items of `b` and of
  // every list above it are not alike, so their type is `list<Any>`
  let (mut a, mut b) = (doubled(1), Value::List(FeelList::from(vec![Value::String("a".into())])));
  for _ in 1..59 {
    (a, b) = (Value::List(FeelList::from(vec![a.clone(), a.clone()])), Value::List(FeelList::from(vec![a, b])));
  }
  let top = Value::List(FeelList::from(vec![a, b]));
  assert!(top.has_type(&nested_list_type(1, FeelType::Any)));
  assert!(top.type_instance_of(&nested_list_type(1, FeelType::Any)));
  assert!(!top.type_instance_of(&nested_list_type(60, FeelType::Number)));
}
