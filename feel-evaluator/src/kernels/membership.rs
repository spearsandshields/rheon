//! Membership: `in` against every right-hand form (DMN 1.6 Table 56 and the unary tests of
//! §10.3.2.9).

use crate::equality::Equality;
use rheon_feel::IntervalType;
use rheon_feel::message::arg;
use rheon_feel::value_null;
use rheon_feel::values::{TypeOf, VALUE_FALSE, VALUE_TRUE, Value};
use std::borrow::Borrow;
use std::collections::HashSet;

/// FEEL `lhv in rhv`: the right-hand value selects the test (a value, a range, a list, a negated
/// list, a unary comparison or `-`). With `two_valued` (B-FEEL), a test of a list that cannot be
/// compared with the tested value is not satisfied.
pub(crate) fn in_operator(lhv: Value, rhv: Value, two_valued: bool) -> Value {
  let tests = if two_valued { Tests::No } else { Tests::Yes };
  match rhv {
    inner @ Value::Null(_)
    | inner @ Value::Number(_)
    | inner @ Value::String(_)
    | inner @ Value::Boolean(_)
    | inner @ Value::Date(_)
    | inner @ Value::Time(_)
    | inner @ Value::DateTime(_)
    | inner @ Value::YearsAndMonthsDuration(_)
    | inner @ Value::DaysAndTimeDuration(_)
    // a single value: `e1 in e2` is `e1 = e2` (DMN 1.6 Table 56, rule 49.c), also where `=` gives
    // null because the values are of different kinds
    | inner @ Value::Context(_) => super::compare::equal(&lhv, &inner),
    Value::Range(l, l_type, r, r_type) => eval_in_range(&lhv, &l, l_type, &r, r_type),
    Value::List(r_inner) => {
      if let Value::List(l_inner) = lhv {
        eval_in_list_in_list(&mut Equality::default(), &l_inner, &r_inner)
      } else {
        eval_in_list(&mut Equality::default(), &lhv, &r_inner, Tests::No)
      }
    }
    Value::ExpressionList(inner) => eval_in_list(&mut Equality::default(), &lhv, &inner, tests),
    Value::NegatedCommaList(inner) => eval_in_negated_list(&mut Equality::default(), &lhv, &inner, tests),
    Value::UnaryEqual(inner) => eval_in_equal(&mut Equality::default(), &lhv, &inner),
    Value::UnaryNotEqual(inner) => eval_in_not_equal(&mut Equality::default(), &lhv, &inner),
    Value::UnaryLess(inner) => eval_in_unary_less(&lhv, inner.borrow()),
    Value::UnaryLessOrEqual(inner) => eval_in_unary_less_or_equal(&lhv, inner.borrow()),
    Value::UnaryGreater(inner) => eval_in_unary_greater(&lhv, inner.borrow()),
    Value::UnaryGreaterOrEqual(inner) => eval_in_unary_greater_or_equal(&lhv, inner.borrow()),
    Value::Irrelevant => VALUE_TRUE,
    Value::TestAnswer(answer) => *answer,
    _ => value_null!("unexpected argument type in 'in' operator: {}", arg(&TypeOf(&rhv))),
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
  not_equal: Vec<bool>,
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

  /// `true` when the list has tests `!= e` and the tested value differs from each endpoint.
  fn differs_from_all(&self) -> bool {
    !self.not_equal.is_empty() && self.not_equal.iter().all(|value| *value)
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
      Value::UnaryNotEqual(inner) => {
        answers.not_equal.push(!matches!(eval_in_equal(equality, left, inner.borrow()), Value::Boolean(true)));
        false
      }
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
  if answers.differs_from_all() {
    return VALUE_TRUE;
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
      Value::UnaryNotEqual(inner) => {
        answers.not_equal.push(!matches!(eval_in_equal(equality, left, inner.borrow()), Value::Boolean(true)));
        false
      }
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
  if answers.differs_from_all() {
    return VALUE_FALSE;
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

fn eval_in_range(lhv: &Value, start: &Value, start_type: IntervalType, end: &Value, end_type: IntervalType) -> Value {
  match lhv {
    Value::Number(value) => match start {
      Value::Number(lv) => match end {
        Value::Number(rv) => {
          let start_ok = if start_type.closed() { value >= lv } else { value > lv };
          let end_ok = if end_type.closed() { value <= rv } else { value < rv };
          Value::Boolean(start_ok && end_ok)
        }
        Value::Null(_) if end_type.undefined() => {
          let start_in_range = if start_type.closed() { value >= lv } else { value > lv };
          Value::Boolean(start_in_range)
        }
        Value::Null(_) => value_null!(),
        _ => value_null!("eval_in_range"),
      },
      Value::Null(_) if start_type.undefined() => match end {
        Value::Number(rv) => {
          let end_in_range = if end_type.closed() { value <= rv } else { value < rv };
          Value::Boolean(end_in_range)
        }
        _ => value_null!("eval_in_range"),
      },
      Value::Null(_) => match end {
        Value::Number(_) => value_null!(),
        _ => value_null!("eval_in_range"),
      },
      _ => value_null!("eval_in_range"),
    },
    Value::String(value) => match start {
      Value::String(lv) => match end {
        Value::String(rv) => {
          let start_ok = if start_type.closed() { value >= lv } else { value > lv };
          let end_ok = if end_type.closed() { value <= rv } else { value < rv };
          Value::Boolean(start_ok && end_ok)
        }
        Value::Null(_) if end_type.opened() => Value::Boolean(if start_type.closed() { value >= lv } else { value > lv }),
        _ => value_null!("eval_in_range"),
      },
      Value::Null(_) if start_type.opened() => match end {
        Value::String(rv) => Value::Boolean(if end_type.closed() { value <= rv } else { value < rv }),
        _ => value_null!("eval_in_range"),
      },
      _ => value_null!("eval_in_range"),
    },
    Value::Date(value) => match start {
      Value::Date(lv) => match end {
        Value::Date(rv) => {
          let start_ok = if start_type.closed() { value >= lv } else { value > lv };
          let end_ok = if end_type.closed() { value <= rv } else { value < rv };
          Value::Boolean(start_ok && end_ok)
        }
        Value::Null(_) if end_type.opened() => Value::Boolean(if start_type.closed() { value >= lv } else { value > lv }),
        _ => value_null!("eval_in_range"),
      },
      Value::Null(_) if start_type.opened() => match end {
        Value::Date(rv) => Value::Boolean(if end_type.closed() { value <= rv } else { value < rv }),
        _ => value_null!("eval_in_range"),
      },
      _ => value_null!("eval_in_range"),
    },
    Value::Time(value) => match start {
      Value::Time(lv) => match end {
        Value::Time(rv) => {
          let start_ok = if start_type.closed() { value >= lv } else { value > lv };
          let end_ok = if end_type.closed() { value <= rv } else { value < rv };
          Value::Boolean(start_ok && end_ok)
        }
        Value::Null(_) if end_type.opened() => Value::Boolean(if start_type.closed() { value >= lv } else { value > lv }),
        _ => value_null!("eval_in_range"),
      },
      Value::Null(_) if start_type.opened() => match end {
        Value::Time(rv) => Value::Boolean(if end_type.closed() { value <= rv } else { value < rv }),
        _ => value_null!("eval_in_range"),
      },
      _ => value_null!("eval_in_range"),
    },
    Value::DateTime(value) => match start {
      Value::DateTime(lv) => match end {
        Value::DateTime(rv) => {
          let start_ok = if start_type.closed() { value >= lv } else { value > lv };
          let end_ok = if end_type.closed() { value <= rv } else { value < rv };
          Value::Boolean(start_ok && end_ok)
        }
        Value::Null(_) if end_type.opened() => Value::Boolean(if start_type.closed() { value >= lv } else { value > lv }),
        _ => value_null!("eval_in_range"),
      },
      Value::Null(_) if start_type.opened() => match end {
        Value::DateTime(rv) => Value::Boolean(if end_type.closed() { value <= rv } else { value < rv }),
        _ => value_null!("eval_in_range"),
      },
      _ => value_null!("eval_in_range"),
    },
    Value::YearsAndMonthsDuration(value) => match start {
      Value::YearsAndMonthsDuration(lv) => match end {
        Value::YearsAndMonthsDuration(rv) => {
          let start_ok = if start_type.closed() { value >= lv } else { value > lv };
          let end_ok = if end_type.closed() { value <= rv } else { value < rv };
          Value::Boolean(start_ok && end_ok)
        }
        Value::Null(_) if end_type.opened() => Value::Boolean(if start_type.closed() { value >= lv } else { value > lv }),
        _ => value_null!("eval_in_range"),
      },
      Value::Null(_) if start_type.opened() => match end {
        Value::YearsAndMonthsDuration(rv) => Value::Boolean(if end_type.closed() { value <= rv } else { value < rv }),
        _ => value_null!("eval_in_range"),
      },
      _ => value_null!("eval_in_range"),
    },
    Value::DaysAndTimeDuration(value) => match start {
      Value::DaysAndTimeDuration(lv) => match end {
        Value::DaysAndTimeDuration(rv) => {
          let start_ok = if start_type.closed() { value >= lv } else { value > lv };
          let end_ok = if end_type.closed() { value <= rv } else { value < rv };
          Value::Boolean(start_ok && end_ok)
        }
        Value::Null(_) if end_type.opened() => Value::Boolean(if start_type.closed() { value >= lv } else { value > lv }),
        _ => value_null!("eval_in_range"),
      },
      Value::Null(_) if start_type.opened() => match end {
        Value::DaysAndTimeDuration(rv) => Value::Boolean(if end_type.closed() { value <= rv } else { value < rv }),
        _ => value_null!("eval_in_range"),
      },
      _ => value_null!("eval_in_range"),
    },
    _ => value_null!("eval_in_range"),
  }
}

fn eval_in_equal(equality: &mut Equality, left: &Value, right: &Value) -> Value {
  if let Some(true) = equality.compare(left, right) { VALUE_TRUE } else { VALUE_FALSE }
}

fn eval_in_not_equal(equality: &mut Equality, left: &Value, right: &Value) -> Value {
  if let Some(false) = equality.compare(left, right) { VALUE_TRUE } else { VALUE_FALSE }
}

fn eval_in_unary_less(left: &Value, right: &Value) -> Value {
  match right {
    Value::Number(r) => {
      if let Value::Number(l) = left {
        return Value::Boolean(*l < *r);
      }
    }
    Value::String(r) => {
      if let Value::String(l) = left {
        return Value::Boolean(l < r);
      }
    }
    Value::Date(r) => {
      if let Value::Date(l) = left {
        return Value::Boolean(l < r);
      }
    }
    Value::Time(r) => {
      if let Value::Time(l) = left {
        return Value::Boolean(l < r);
      }
    }
    Value::DateTime(r) => {
      if let Value::DateTime(l) = left {
        return Value::Boolean(l < r);
      }
    }
    Value::YearsAndMonthsDuration(r) => {
      if let Value::YearsAndMonthsDuration(l) = left {
        return Value::Boolean(l < r);
      }
    }
    Value::DaysAndTimeDuration(r) => {
      if let Value::DaysAndTimeDuration(l) = left {
        return Value::Boolean(l < r);
      }
    }
    _ => {}
  }
  value_null!("eval_in_unary_less")
}

fn eval_in_unary_less_or_equal(left: &Value, right: &Value) -> Value {
  match right {
    Value::Number(r) => {
      if let Value::Number(l) = left {
        return Value::Boolean(*l <= *r);
      }
    }
    Value::String(r) => {
      if let Value::String(l) = left {
        return Value::Boolean(l <= r);
      }
    }
    Value::Date(r) => {
      if let Value::Date(l) = left {
        return Value::Boolean(l <= r);
      }
    }
    Value::Time(r) => {
      if let Value::Time(l) = left {
        return Value::Boolean(l <= r);
      }
    }
    Value::DateTime(r) => {
      if let Value::DateTime(l) = left {
        return Value::Boolean(l <= r);
      }
    }
    Value::YearsAndMonthsDuration(r) => {
      if let Value::YearsAndMonthsDuration(l) = left {
        return Value::Boolean(l <= r);
      }
    }
    Value::DaysAndTimeDuration(r) => {
      if let Value::DaysAndTimeDuration(l) = left {
        return Value::Boolean(l <= r);
      }
    }
    _ => {}
  }
  value_null!("eval_in_unary_less_or_equal")
}

fn eval_in_unary_greater(left: &Value, right: &Value) -> Value {
  match right {
    Value::Number(r) => {
      if let Value::Number(l) = left {
        return Value::Boolean(*l > *r);
      }
    }
    Value::String(r) => {
      if let Value::String(l) = left {
        return Value::Boolean(l > r);
      }
    }
    Value::Date(r) => {
      if let Value::Date(l) = left {
        return Value::Boolean(l > r);
      }
    }
    Value::Time(r) => {
      if let Value::Time(l) = left {
        return Value::Boolean(l > r);
      }
    }
    Value::DateTime(r) => {
      if let Value::DateTime(l) = left {
        return Value::Boolean(l > r);
      }
    }
    Value::YearsAndMonthsDuration(r) => {
      if let Value::YearsAndMonthsDuration(l) = left {
        return Value::Boolean(l > r);
      }
    }
    Value::DaysAndTimeDuration(r) => {
      if let Value::DaysAndTimeDuration(l) = left {
        return Value::Boolean(l > r);
      }
    }
    _ => {}
  }
  value_null!("eval_in_unary_greater")
}

fn eval_in_unary_greater_or_equal(left: &Value, right: &Value) -> Value {
  match right {
    Value::Number(r) => {
      if let Value::Number(l) = left {
        return Value::Boolean(*l >= *r);
      }
    }
    Value::String(r) => {
      if let Value::String(left_value) = left {
        return Value::Boolean(left_value >= r);
      }
    }
    Value::Date(r) => {
      if let Value::Date(l) = left {
        return Value::Boolean(l >= r);
      }
    }
    Value::Time(r) => {
      if let Value::Time(l) = left {
        return Value::Boolean(l >= r);
      }
    }
    Value::DateTime(r) => {
      if let Value::DateTime(l) = left {
        return Value::Boolean(l >= r);
      }
    }
    Value::YearsAndMonthsDuration(r) => {
      if let Value::YearsAndMonthsDuration(l) = left {
        return Value::Boolean(l >= r);
      }
    }
    Value::DaysAndTimeDuration(r) => {
      if let Value::DaysAndTimeDuration(l) = left {
        return Value::Boolean(l >= r);
      }
    }
    _ => {}
  }
  value_null!("eval_in_unary_greater_or_equal")
}
