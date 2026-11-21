//! Membership: `in` against every right-hand form (DMN 1.6 Table 56 and the unary tests of
//! §10.3.2.9).

use super::compare::{Order, compare_ge, compare_gt, compare_le, compare_lt, order};
use crate::equality::Equality;
use rheon_feel::IntervalType;
use rheon_feel::message::arg;
use rheon_feel::value_null;
use rheon_feel::values::{TypeOf, VALUE_FALSE, VALUE_TRUE, Value};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::HashSet;

/// FEEL `lhv in rhv`: the right-hand value selects the test (a value, a range, a list, a negated
/// list, a unary comparison or `-`). With `two_valued` (B-FEEL), a test of a list that cannot be
/// compared with the tested value is not satisfied.
pub(crate) fn in_operator(lhv: &Value, rhv: &Value, two_valued: bool) -> Value {
  let tests = if two_valued { Tests::No } else { Tests::Yes };
  match rhv {
    Value::Null(_)
    | Value::Number(_)
    | Value::String(_)
    | Value::Boolean(_)
    | Value::Date(_)
    | Value::Time(_)
    | Value::DateTime(_)
    | Value::YearsAndMonthsDuration(_)
    | Value::DaysAndTimeDuration(_)
    // a single value: `e1 in e2` is `e1 = e2` (DMN 1.6 Table 56, rule 49.c), also where `=` gives
    // null because the values are of different kinds
    | Value::Context(_) => super::compare::equal(lhv, rhv),
    Value::Range(l, l_type, r, r_type) => eval_in_range(lhv, l, *l_type, r, *r_type),
    Value::List(r_inner) => {
      if let Value::List(l_inner) = lhv {
        eval_in_list_in_list(&mut Equality::default(), l_inner, r_inner)
      } else {
        eval_in_list(&mut Equality::default(), lhv, r_inner, Tests::No)
      }
    }
    Value::ExpressionList(inner) => eval_in_list(&mut Equality::default(), lhv, inner, tests),
    Value::NegatedCommaList(inner) => eval_in_negated_list(&mut Equality::default(), lhv, inner, tests),
    // `e1 in = e2` is `e1 = e2`, and `e1 in != e2` is `e1 != e2` (Table 56)
    Value::UnaryEqual(inner) => super::compare::equal(lhv, inner),
    Value::UnaryNotEqual(inner) => super::compare::not_equal(lhv, inner),
    Value::UnaryLess(inner) => eval_in_unary_less(lhv, inner),
    Value::UnaryLessOrEqual(inner) => eval_in_unary_less_or_equal(lhv, inner),
    Value::UnaryGreater(inner) => eval_in_unary_greater(lhv, inner),
    Value::UnaryGreaterOrEqual(inner) => eval_in_unary_greater_or_equal(lhv, inner),
    Value::Irrelevant => VALUE_TRUE,
    Value::TestAnswer(answer) => Value::clone(answer),
    _ => value_null!("unexpected argument type in 'in' operator: {}", arg(&TypeOf(rhv))),
  }
}

/// Whether the items of a list are tests that `or` joins (`e1 in (e2, e3)` and unary tests, DMN
/// 1.6 Table 56), or the items of a list value, where an item that cannot be compared is
/// another item.
#[derive(Clone, Copy, PartialEq)]
enum Tests {
  Yes,
  No,
}

/// What the tests of a list answer when none is true: the first test without an answer.
enum Unknown<'a> {
  /// A value of another kind than the tested one: the null of `=`, built when it is the answer.
  Equal(&'a Value),
  /// The null of a comparison or a range.
  Null(Value),
}

/// The answers of the tests of a list seen so far, none of them true.
#[derive(Default)]
struct Answers<'a> {
  unknown: Option<Unknown<'a>>,
}

impl<'a> Answers<'a> {
  /// Takes the answer of a test; `true` when the test is true.
  fn test(&mut self, answer: Value) -> bool {
    match answer {
      Value::Boolean(true) => return true,
      null @ Value::Null(_) if self.unknown.is_none() => self.unknown = Some(Unknown::Null(null)),
      _ => {}
    }
    false
  }

  /// Takes the comparison of the tested value with a value of the list; `true` when they are equal.
  fn equal(&mut self, equality: &mut Equality, left: &Value, item: &'a Value) -> bool {
    match equality.compare(left, item) {
      Some(true) => return true,
      None if self.unknown.is_none() => self.unknown = Some(Unknown::Equal(item)),
      _ => {}
    }
    false
  }

  /// Takes the test `!= item` of the tested value; `true` when they differ. Values that `=` cannot
  /// compare give the null of `=`, as `!=` does (10.3.2.15).
  fn not_equal(&mut self, equality: &mut Equality, left: &Value, item: &'a Value) -> bool {
    match equality.compare(left, item) {
      Some(false) => return true,
      None if self.unknown.is_none() => self.unknown = Some(Unknown::Equal(item)),
      _ => {}
    }
    false
  }

  /// The null of the first test without an answer, when there is one and the items are tests:
  /// `or` over tests of which none is true and one is null is null.
  fn unknown(self, left: &Value, tests: Tests) -> Option<Value> {
    match self.unknown {
      Some(Unknown::Equal(item)) if tests == Tests::Yes => Some(super::compare::equal(left, item)),
      Some(Unknown::Null(null)) if tests == Tests::Yes => Some(null),
      _ => None,
    }
  }
}

fn eval_in_list(equality: &mut Equality, left: &Value, items: &[Value], tests: Tests) -> Value {
  let mut answers = Answers::default();
  for item in items {
    let found = match item {
      inner @ Value::Null(_)
      | inner @ Value::String(_)
      | inner @ Value::Number(_)
      | inner @ Value::Boolean(_)
      | inner @ Value::Date(_)
      | inner @ Value::Time(_)
      | inner @ Value::DateTime(_)
      | inner @ Value::YearsAndMonthsDuration(_)
      | inner @ Value::DaysAndTimeDuration(_)
      | inner @ Value::Context(_) => answers.equal(equality, left, inner),
      Value::UnaryEqual(inner) => answers.equal(equality, left, inner.borrow()),
      // `!= e` is a test like the others (`or` joins them, Table 56): true when the values differ
      Value::UnaryNotEqual(inner) => answers.not_equal(equality, left, inner.borrow()),
      Value::UnaryLess(inner) => answers.test(eval_in_unary_less(left, inner.borrow())),
      Value::UnaryLessOrEqual(inner) => answers.test(eval_in_unary_less_or_equal(left, inner.borrow())),
      Value::UnaryGreater(inner) => answers.test(eval_in_unary_greater(left, inner.borrow())),
      Value::UnaryGreaterOrEqual(inner) => answers.test(eval_in_unary_greater_or_equal(left, inner.borrow())),
      // a list value among the tests: the tested value is one of its items or it is not
      Value::List(inner) => answers.test(eval_in_list(equality, left, inner, Tests::No)),
      Value::Range(l, l_closed, r, r_closed) => answers.test(eval_in_range(left, l, *l_closed, r, *r_closed)),
      // a boolean expression using `?`: its own answer
      Value::TestAnswer(answer) => answers.test(answer.as_ref().clone()),
      other => return value_null!("unexpected type in list: {}", arg(&TypeOf(other))),
    };
    if found {
      return VALUE_TRUE;
    }
  }
  answers.unknown(left, tests).unwrap_or(VALUE_FALSE)
}

/// `not(tests)`: the negation of what the tests answer, so null where they answer null.
fn eval_in_negated_list(equality: &mut Equality, left: &Value, items: &[Value], tests: Tests) -> Value {
  let mut answers = Answers::default();
  for item in items {
    let found = match item {
      inner @ Value::Null(_)
      | inner @ Value::String(_)
      | inner @ Value::Number(_)
      | inner @ Value::Boolean(_)
      | inner @ Value::Date(_)
      | inner @ Value::Time(_)
      | inner @ Value::DateTime(_)
      | inner @ Value::DaysAndTimeDuration(_)
      | inner @ Value::YearsAndMonthsDuration(_) => answers.equal(equality, left, inner),
      Value::UnaryEqual(inner) => answers.equal(equality, left, inner.borrow()),
      // `!= e` is a test like the others (`or` joins them, Table 56): true when the values differ
      Value::UnaryNotEqual(inner) => answers.not_equal(equality, left, inner.borrow()),
      Value::UnaryLess(inner) => answers.test(eval_in_unary_less(left, inner.borrow())),
      Value::UnaryLessOrEqual(inner) => answers.test(eval_in_unary_less_or_equal(left, inner.borrow())),
      Value::UnaryGreater(inner) => answers.test(eval_in_unary_greater(left, inner.borrow())),
      Value::UnaryGreaterOrEqual(inner) => answers.test(eval_in_unary_greater_or_equal(left, inner.borrow())),
      Value::List(inner) => answers.test(eval_in_list(equality, left, inner, Tests::No)),
      Value::Range(l, l_closed, r, r_closed) => answers.test(eval_in_range(left, l, *l_closed, r, *r_closed)),
      // a boolean expression using `?`: its own answer
      Value::TestAnswer(answer) => answers.test(answer.as_ref().clone()),
      other => return value_null!("unexpected type in negated list: {}", arg(&TypeOf(other))),
    };
    if found {
      return VALUE_FALSE;
    }
  }
  answers.unknown(left, tests).unwrap_or(VALUE_TRUE)
}

/// Checks if all elements from `list` are present in `items`.
fn eval_in_list_in_list(equality: &mut Equality, l_items: &[Value], r_items: &[Value]) -> Value {
  for item in r_items {
    if let Value::List(rhs) = item {
      let mut available: HashSet<usize> = (0..rhs.len()).collect();
      for l in l_items {
        let mut found = false;
        for (index, r) in rhs.iter().enumerate() {
          if available.contains(&index)
            && let Value::Boolean(true) = eval_in_equal(equality, l, r)
          {
            available.remove(&index);
            found = true;
            break;
          }
        }
        if !found {
          return VALUE_FALSE;
        }
      }
      return VALUE_TRUE;
    }
  }
  VALUE_FALSE
}

/// `e1 in` a range (DMN 1.7 Beta 1, Table 56): `e1 > e2 and e1 < e3`, with `>=` and `<=` for included
/// endpoints, by the ordering kernel and the `and` of Table 51; an undefined endpoint has no
/// comparison. So a value that has no order against an endpoint, or an endpoint written as null,
/// gives null where the other comparison does not decide the answer.
fn eval_in_range(lhv: &Value, start: &Value, start_type: IntervalType, end: &Value, end_type: IntervalType) -> Value {
  let accept_start = if start_type.closed() { Ordering::is_ge } else { Ordering::is_gt };
  let low = if start_type.undefined() { Some(true) } else { side(lhv, start, accept_start) };
  if low == Some(false) {
    return VALUE_FALSE;
  }
  let accept_end = if end_type.closed() { Ordering::is_le } else { Ordering::is_lt };
  let high = if end_type.undefined() { Some(true) } else { side(lhv, end, accept_end) };
  match (low, high) {
    (_, Some(false)) => VALUE_FALSE,
    (Some(true), Some(true)) => VALUE_TRUE,
    // the null of the comparison that has no answer, with its reason
    (None, _) if start_type.closed() => compare_ge(lhv.clone(), start.clone()),
    (None, _) => compare_gt(lhv.clone(), start.clone()),
    (_, _) if end_type.closed() => compare_le(lhv.clone(), end.clone()),
    (_, _) => compare_lt(lhv.clone(), end.clone()),
  }
}

/// The answer of comparing the tested value with an endpoint by the ordering kernel, or None when
/// the comparison has none (values of other kinds, or without an order).
fn side(lhv: &Value, endpoint: &Value, accept: fn(Ordering) -> bool) -> Option<bool> {
  match order(lhv, endpoint) {
    Order::Ordered(ordering) => Some(accept(ordering)),
    Order::Indeterminate | Order::Unordered => None,
  }
}

fn eval_in_equal(equality: &mut Equality, left: &Value, right: &Value) -> Value {
  if let Some(true) = equality.compare(left, right) { VALUE_TRUE } else { VALUE_FALSE }
}

/// `e1 in < e2` is `e1 < e2` (Table 56), and so for the other comparisons.
fn eval_in_unary_less(left: &Value, right: &Value) -> Value {
  compare_lt(left.clone(), right.clone())
}

fn eval_in_unary_less_or_equal(left: &Value, right: &Value) -> Value {
  compare_le(left.clone(), right.clone())
}

fn eval_in_unary_greater(left: &Value, right: &Value) -> Value {
  compare_gt(left.clone(), right.clone())
}

fn eval_in_unary_greater_or_equal(left: &Value, right: &Value) -> Value {
  compare_ge(left.clone(), right.clone())
}
