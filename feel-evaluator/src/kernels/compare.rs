//! Comparisons: `<`, `<=`, `>`, `>=`, `=`, `!=` and `between` (DMN 1.6 Tables 53 to 55).

use crate::equality::eval_ternary_equality;
use rheon_feel::message::arg;
use rheon_feel::value_null;
use rheon_feel::values::{TypeOf, Value};
use rheon_feel_temporal::FeelTime;

/// FEEL `lhs = rhs` (DMN 1.6 Table 53, Table 54): a boolean, or null with an error when the operands
/// cannot be compared.
pub(crate) fn equal(lhs: &Value, rhs: &Value) -> Value {
  if let Some(result) = eval_ternary_equality(lhs, rhs) {
    Value::Boolean(result)
  } else {
    value_null!("equal err '{}' =?= '{}'", arg(lhs), arg(rhs))
  }
}

/// FEEL `lhs != rhs`: DMN 1.6 §10.3.2.15, "By definition, FEEL(e1 != e2) is FEEL(not(e 1= e2))",
/// so the operands that `=` cannot compare give its null and its error.
pub(crate) fn not_equal(lhs: &Value, rhs: &Value) -> Value {
  match equal(lhs, rhs) {
    Value::Boolean(result) => Value::Boolean(!result),
    null => null,
  }
}

/// The null of a comparison of two times that have no order: a zone gives one of them no single
/// offset (the hour that a clock change of the zone skips or repeats, or a zone the time zone
/// database does not know), and the two are not written in the same zone.
fn times_without_an_order(symbol: &str, lhs: &FeelTime, rhs: &FeelTime) -> Value {
  value_null!(
    "comparison '{}' expected two times that have an order, found {} and {}: the zone of one gives it no single offset, and they are not written in the same zone",
    symbol,
    lhs,
    rhs
  )
}

/// The order of two values (DMN 1.7 Beta 1, Table 53): the one kernel behind `<`, `<=`, `>`,
/// `>=`, `min`, `max` and the comparisons the range functions and the unary tests make.
pub(crate) enum Order {
  /// Two values of one ordered kind, and how they compare.
  Ordered(std::cmp::Ordering),
  /// Two times, or two date and times, that have no order: the zone of one gives it no single
  /// offset, and the two are not written in the same zone. (A value without an offset takes the
  /// offset of the other, Table 55.)
  Indeterminate,
  /// Values that are not both of one ordered kind.
  Unordered,
}

/// The [Order] of `lhv` and `rhv`.
pub(crate) fn order(lhv: &Value, rhv: &Value) -> Order {
  let ordering = match (lhv, rhv) {
    (Value::Number(lh), Value::Number(rh)) => lh.partial_cmp(rh),
    (Value::String(lh), Value::String(rh)) => lh.partial_cmp(rh),
    (Value::Date(lh), Value::Date(rh)) => lh.partial_cmp(rh),
    (Value::DateTime(lh), Value::DateTime(rh)) => lh.partial_cmp(rh),
    (Value::Time(lh), Value::Time(rh)) => lh.partial_cmp(rh),
    (Value::DaysAndTimeDuration(lh), Value::DaysAndTimeDuration(rh)) => lh.partial_cmp(rh),
    (Value::YearsAndMonthsDuration(lh), Value::YearsAndMonthsDuration(rh)) => lh.partial_cmp(rh),
    _ => return Order::Unordered,
  };
  ordering.map_or(Order::Indeterminate, Order::Ordered)
}

/// The null of an ordering comparison on two values of one temporal kind that have no order.
#[cold]
#[inline(never)]
fn without_an_order(symbol: &str, lhv: &Value, rhv: &Value) -> Value {
  match (lhv, rhv) {
    (Value::Time(lh), Value::Time(rh)) => times_without_an_order(symbol, lh, rh),
    _ => value_null!(
      "comparison '{}' expected two date and times that have an order, found {} and {}: the zone of one gives it no single offset there, and they are not written in the same zone",
      symbol,
      arg(lhv),
      arg(rhv)
    ),
  }
}

/// An ordering comparison on two values of one ordered kind ([order]); otherwise null with an
/// error that names the operator, the type it expected and the type it found.
macro_rules! ordering {
  ($(#[$meta:meta])* $name:ident, $test:ident, $symbol:literal) => {
    $(#[$meta])*
    pub(crate) fn $name(lhv: Value, rhv: Value) -> Value {
      match order(&lhv, &rhv) {
        Order::Ordered(ordering) => Value::Boolean(ordering.$test()),
        Order::Indeterminate => without_an_order($symbol, &lhv, &rhv),
        Order::Unordered => unordered($symbol, &lhv, &rhv),
      }
    }
  };
}

ordering!(
  /// FEEL `lhv < rhv` on two values of one ordered kind; otherwise null with an error.
  compare_lt, is_lt, "<"
);
ordering!(
  /// FEEL `lhv <= rhv` on two values of one ordered kind; otherwise null with an error.
  compare_le, is_le, "<="
);
ordering!(
  /// FEEL `lhv > rhv` on two values of one ordered kind; otherwise null with an error.
  compare_gt, is_gt, ">"
);
ordering!(
  /// FEEL `lhv >= rhv` on two values of one ordered kind; otherwise null with an error.
  compare_ge, is_ge, ">="
);

/// The null of an ordering comparison on operands it does not order (DMN 1.6 Table 53: both of
/// one of the ordered kinds). When the left operand is of an ordered kind the error names it as
/// the type expected on the right; otherwise it lists the ordered kinds.
#[cold]
#[inline(never)]
fn unordered(symbol: &str, lhv: &Value, rhv: &Value) -> Value {
  let ordered = matches!(
    lhv,
    Value::Number(_) | Value::String(_) | Value::Date(_) | Value::DateTime(_) | Value::Time(_) | Value::DaysAndTimeDuration(_) | Value::YearsAndMonthsDuration(_)
  );
  if ordered {
    value_null!(
      "comparison '{}' expected {} on the right, as on the left, found {}",
      symbol,
      arg(&TypeOf(lhv)),
      arg(&TypeOf(rhv))
    )
  } else {
    value_null!(
      "comparison '{}' expected a number, string, date, time, date and time, or duration on the left, found {}",
      symbol,
      arg(&TypeOf(lhv))
    )
  }
}

/// FEEL `lhv between mhv and rhv` (DMN 1.7 Beta 1, Table 55): `lhv >= mhv and lhv <= rhv` by the
/// ordering kernel and the `and` of Table 51, the three values evaluated before. One comparison
/// that is false decides it, whatever the other one is (`5 between 10 and null` is false); where
/// neither decides, the null names the operand that has no order with `lhv`, and a null `lhv` is
/// the result.
pub(crate) fn between(lhv: Value, mhv: Value, rhv: Value) -> Value {
  let low = order(&lhv, &mhv);
  let high = order(&lhv, &rhv);
  match (&low, &high) {
    (Order::Ordered(low), _) if low.is_lt() => return Value::Boolean(false),
    (_, Order::Ordered(high)) if high.is_gt() => return Value::Boolean(false),
    (Order::Ordered(_), Order::Ordered(_)) => return Value::Boolean(true),
    _ => {}
  }
  if lhv.is_null() {
    return lhv;
  }
  let ordered = matches!(
    lhv,
    Value::Number(_) | Value::String(_) | Value::Date(_) | Value::DateTime(_) | Value::Time(_) | Value::DaysAndTimeDuration(_) | Value::YearsAndMonthsDuration(_)
  );
  if !ordered {
    return value_null!("unexpected value type in 'between' operator: {}", arg(&TypeOf(&lhv)));
  }
  // the operand without an answer: the second one unless it has one
  let (operand, which, order) = match low {
    Order::Ordered(_) => (&rhv, "third", high),
    _ => (&mhv, "second", low),
  };
  match order {
    Order::Indeterminate => without_an_order("between", &lhv, operand),
    _ => value_null!(
      "expected {} as a {} argument in 'between' operator, actual value type is {}",
      arg(&TypeOf(&lhv)),
      which,
      arg(&TypeOf(operand))
    ),
  }
}
