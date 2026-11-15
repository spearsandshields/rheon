//! Walks over values that hold the same list on many paths: a list that a loop doubles `n` times
//! through `partial` has 2^(n-1) paths to its first item in a size linear in `n`, and every walk
//! that answers a boolean visits each shared list (or pair of lists) once.

use super::*;
use crate::equality::eval_ternary_equality;
use rheon_feel::context::FeelContext;
use rheon_feel::values::FeelList;
use rheon_feel::{Name, value_number};

/// The list `[x, x]` built from the one before it `n - 1` times, starting with `[first]`.
fn doubled(n: u32, first: &str) -> String {
  format!("(for i in 1..{n} return if i = 1 then [{first}] else [partial[-1], partial[-1]])[-1]")
}

#[test]
fn equality_compares_each_pair_of_shared_lists_once() {
  let x = doubled(60, "1");
  te_warnings(&format!("{{x: {x}, y: {x}, r: [x = x, x = y, x != y]}}.r"), "[true, true, false]", &[]);
  // `a` and `b` are doubled alike, except that the last item of `b` holds 2 where `a` holds 1
  let pairs = "(for i in 1..60 return if i = 1 then [[1], [2]] else [[partial[-1][1], partial[-1][1]], [partial[-1][1], partial[-1][2]]])[-1]";
  te_warnings(
    &format!("{{p: {pairs}, a: p[1], b: p[2], r: [a = a, a = b, b = b, [a, b] = [a, a]]}}.r"),
    "[true, false, true, false]",
    &[],
  );
  // items that can not be compared make the lists unequal, however often they are reached
  let functions = doubled(60, "f");
  te_warnings(&format!("{{f: function() 1, x: {functions}, r: [x = x, {{a: x}} = {{a: x}}]}}.r"), "[false, false]", &[]);
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
}

/// The same value with nothing shared: every list and context built anew.
fn unshared(value: &Value) -> Value {
  match value {
    Value::List(items) => Value::List(items.iter().map(unshared).collect()),
    Value::Context(ctx) => {
      let mut copy = FeelContext::default();
      for (name, value) in ctx.iter() {
        copy.set_entry(name, unshared(value));
      }
      Value::Context(copy)
    }
    other => other.clone(),
  }
}

/// Values built from the values before them, each a scalar or a list or a context of earlier
/// values (so earlier lists are shared), and some of them copies with nothing shared.
fn values(rng: &mut Rng) -> Vec<Value> {
  let mut values = vec![value_number!(1), value_number!(2), Value::String("a".into()), Value::Null(None), Value::Boolean(true)];
  for _ in 0..14 {
    let pick = |rng: &mut Rng, values: &[Value]| values[rng.below(values.len() as u64) as usize].clone();
    let value = match rng.below(5) {
      0 => unshared(&pick(rng, &values)),
      1 => {
        let mut ctx = FeelContext::default();
        for _ in 0..rng.below(3) {
          ctx.set_entry(&Name::from(["a", "b"][rng.below(2) as usize]), pick(rng, &values));
        }
        Value::Context(ctx)
      }
      _ => Value::List((0..rng.below(4)).map(|_| pick(rng, &values)).collect::<FeelList>()),
    };
    values.push(value);
  }
  values
}

/// Equality on values that share lists answers as on the same values with nothing shared.
#[test]
fn equality_of_shared_values_is_that_of_the_unshared_values() {
  let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
  let mut equal_apart = 0;
  for _ in 0..200 {
    let values = values(&mut rng);
    for (i, lhs) in values.iter().enumerate() {
      for (j, rhs) in values.iter().enumerate() {
        let answer = eval_ternary_equality(lhs, rhs);
        assert_eq!(eval_ternary_equality(&unshared(lhs), &unshared(rhs)), answer, "{lhs} = {rhs}");
        equal_apart += usize::from(i != j && answer == Some(true));
      }
    }
  }
  // the pairs cover equal values built apart, not only a value compared with itself
  assert!(equal_apart > 1_000, "{equal_apart} pairs of values built apart are equal");
}

/// The built-ins that compare the items of a list with one another or with a match, and `in`,
/// answer on values that share lists as on the same values with nothing shared: one walk serves
/// all comparisons of a call.
#[test]
fn list_scans_over_shared_values_answer_as_over_unshared_values() {
  use crate::bifs::core::{distinct_values, index_of, intersection, list_contains, union};
  let mut rng = Rng(0x2545_f491_4f6c_dd1d);
  let in_list = build_evaluator(&rheon_feel_parser::parse_expression(&FeelScope::default(), "x in l", false).unwrap());
  let is_in = |x: &Value, l: &Value| {
    let mut ctx = FeelContext::default();
    ctx.set_entry(&Name::from("x"), x.clone());
    ctx.set_entry(&Name::from("l"), l.clone());
    in_list(&FeelScope::from(ctx))
  };
  for _ in 0..100 {
    let values = values(&mut rng);
    let (front, back) = values.split_at(values.len() / 2);
    let shared = [Value::List(front.iter().cloned().collect()), Value::List(back.iter().cloned().collect())];
    let apart = [unshared(&shared[0]), unshared(&shared[1])];
    assert_eq!(distinct_values(&apart[0]), distinct_values(&shared[0]));
    assert_eq!(union(&apart), union(&shared));
    assert_eq!(intersection(&apart), intersection(&shared));
    for value in &values {
      let value_apart = unshared(value);
      for (list, list_apart) in shared.iter().zip(&apart) {
        assert_eq!(index_of(list_apart, &value_apart), index_of(list, value), "index of({list}, {value})");
        assert_eq!(list_contains(list_apart, &value_apart), list_contains(list, value), "list contains({list}, {value})");
        assert_eq!(is_in(&value_apart, list_apart), is_in(value, list), "{value} in {list}");
      }
    }
  }
}

#[test]
fn instance_of_reads_a_shared_list_once() {
  let x = doubled(60, "1");
  let list_of = |levels: usize, item: &str| format!("{}{item}{}", "list<".repeat(levels), ">".repeat(levels));
  te_warnings(
    &format!(
      "{{x: {x}, r: [x instance of list<Any>, x instance of {}, x instance of {}, x instance of {}, {{a: x}} instance of context<a: {}>]}}.r",
      list_of(60, "number"),
      list_of(60, "string"),
      list_of(59, "number"),
      list_of(60, "number")
    ),
    "[true, true, false, false, true]",
    &[],
  );
}
