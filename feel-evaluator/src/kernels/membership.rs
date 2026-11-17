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
/// list, a unary comparison or `-`).
pub(crate) fn in_operator(lhv: Value, rhv: Value) -> Value {
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
    | inner @ Value::Context(_) => eval_in_equal(&mut Equality::default(), &lhv, &inner),
    Value::Range(l, l_type, r, r_type) => eval_in_range(&lhv, &l, l_type, &r, r_type),
    Value::List(r_inner) => {
      if let Value::List(l_inner) = lhv {
        eval_in_list_in_list(&mut Equality::default(), &l_inner, &r_inner)
      } else {
        eval_in_list(&mut Equality::default(), &lhv, &r_inner)
      }
    }
    Value::ExpressionList(inner) => eval_in_list(&mut Equality::default(), &lhv, &inner),
    Value::NegatedCommaList(inner) => eval_in_negated_list(&mut Equality::default(), &lhv, &inner),
    Value::UnaryEqual(inner) => eval_in_equal(&mut Equality::default(), &lhv, &inner),
    Value::UnaryNotEqual(inner) => eval_in_not_equal(&mut Equality::default(), &lhv, &inner),
    Value::UnaryLess(inner) => eval_in_unary_less(&lhv, inner.borrow()),
    Value::UnaryLessOrEqual(inner) => eval_in_unary_less_or_equal(&lhv, inner.borrow()),
    Value::UnaryGreater(inner) => eval_in_unary_greater(&lhv, inner.borrow()),
    Value::UnaryGreaterOrEqual(inner) => eval_in_unary_greater_or_equal(&lhv, inner.borrow()),
    Value::Irrelevant => VALUE_TRUE,
    _ => value_null!("unexpected argument type in 'in' operator: {}", arg(&TypeOf(&rhv))),
  }
}

fn eval_in_list(equality: &mut Equality, left: &Value, items: &[Value]) -> Value {
  let mut unary_not_equal = vec![];
  for item in items {
    match item {
      inner @ Value::Null(_)
      | inner @ Value::String(_)
      | inner @ Value::Number(_)
      | inner @ Value::Boolean(_)
      | inner @ Value::Date(_)
      | inner @ Value::Time(_)
      | inner @ Value::DateTime(_)
      | inner @ Value::YearsAndMonthsDuration(_)
      | inner @ Value::DaysAndTimeDuration(_)
      | inner @ Value::Context(_) => {
        if let Value::Boolean(true) = eval_in_equal(equality, left, inner) {
          return VALUE_TRUE;
        }
      }
      Value::UnaryEqual(inner) => {
        if let Value::Boolean(true) = eval_in_equal(equality, left, inner.borrow()) {
          return VALUE_TRUE;
        }
      }
      Value::UnaryNotEqual(inner) => {
        if let Value::Boolean(true) = eval_in_equal(equality, left, inner.borrow()) {
          unary_not_equal.push(false);
        } else {
          unary_not_equal.push(true);
        }
      }
      Value::UnaryLess(inner) => {
        if let Value::Boolean(true) = eval_in_unary_less(left, inner.borrow()) {
          return VALUE_TRUE;
        }
      }
      Value::UnaryLessOrEqual(inner) => {
        if let Value::Boolean(true) = eval_in_unary_less_or_equal(left, inner.borrow()) {
          return VALUE_TRUE;
        }
      }
      Value::UnaryGreater(inner) => {
        if let Value::Boolean(true) = eval_in_unary_greater(left, inner.borrow()) {
          return VALUE_TRUE;
        }
      }
      Value::UnaryGreaterOrEqual(inner) => {
        if let Value::Boolean(true) = eval_in_unary_greater_or_equal(left, inner.borrow()) {
          return VALUE_TRUE;
        }
      }
      Value::List(inner) => {
        if let Value::Boolean(true) = eval_in_list(equality, left, inner) {
          return VALUE_TRUE;
        }
      }
      Value::Range(l, l_closed, r, r_closed) => {
        if let Value::Boolean(true) = eval_in_range(left, l, *l_closed, r, *r_closed) {
          return VALUE_TRUE;
        }
      }
      other => return value_null!("unexpected type in list: {}", arg(&TypeOf(other))),
    }
  }
  if !unary_not_equal.is_empty() && unary_not_equal.iter().all(|value| *value) {
    return VALUE_TRUE;
  }
  VALUE_FALSE
}

fn eval_in_negated_list(equality: &mut Equality, left: &Value, items: &[Value]) -> Value {
  let mut unary_not_equal = vec![];
  for item in items {
    match item {
      inner @ Value::Null(_)
      | inner @ Value::String(_)
      | inner @ Value::Number(_)
      | inner @ Value::Boolean(_)
      | inner @ Value::Date(_)
      | inner @ Value::Time(_)
      | inner @ Value::DateTime(_)
      | inner @ Value::DaysAndTimeDuration(_)
      | inner @ Value::YearsAndMonthsDuration(_) => {
        if let Value::Boolean(true) = eval_in_equal(equality, left, inner) {
          return VALUE_FALSE;
        }
      }
      Value::UnaryEqual(inner) => {
        if let Value::Boolean(true) = eval_in_equal(equality, left, inner) {
          return VALUE_FALSE;
        }
      }
      Value::UnaryNotEqual(inner) => {
        if let Value::Boolean(true) = eval_in_equal(equality, left, inner.borrow()) {
          unary_not_equal.push(false);
        } else {
          unary_not_equal.push(true);
        }
      }
      Value::UnaryLess(inner) => {
        if let Value::Boolean(true) = eval_in_unary_less(left, inner.borrow()) {
          return VALUE_FALSE;
        }
      }
      Value::UnaryLessOrEqual(inner) => {
        if let Value::Boolean(true) = eval_in_unary_less_or_equal(left, inner.borrow()) {
          return VALUE_FALSE;
        }
      }
      Value::UnaryGreater(inner) => {
        if let Value::Boolean(true) = eval_in_unary_greater(left, inner.borrow()) {
          return VALUE_FALSE;
        }
      }
      Value::UnaryGreaterOrEqual(inner) => {
        if let Value::Boolean(true) = eval_in_unary_greater_or_equal(left, inner.borrow()) {
          return VALUE_FALSE;
        }
      }
      Value::List(inner) => {
        if let Value::Boolean(true) = eval_in_list(equality, left, inner) {
          return VALUE_FALSE;
        }
      }
      Value::Range(l, l_closed, r, r_closed) => {
        if let Value::Boolean(true) = eval_in_range(left, l, *l_closed, r, *r_closed) {
          return VALUE_FALSE;
        }
      }
      other => return value_null!("unexpected type in negated list: {}", arg(&TypeOf(other))),
    }
  }
  if !unary_not_equal.is_empty() && unary_not_equal.iter().all(|value| *value) {
    return VALUE_FALSE;
  }
  VALUE_TRUE
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
