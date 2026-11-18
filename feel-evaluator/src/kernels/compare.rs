//! Comparisons: `<`, `<=`, `>`, `>=`, `=`, `!=` and `between` (DMN 1.6 Tables 53 to 55).

use crate::equality::eval_ternary_equality;
use rheon_feel::message::arg;
use rheon_feel::value_null;
use rheon_feel::values::{TypeOf, Value};

/// Prepares null value with error message for second argument in `between` operator.
macro_rules! between_null2 {
  ($expected:literal, $actual:expr) => {
    value_null!(
      "expected {} as a second argument in 'between' operator, actual value type is {}",
      $expected,
      arg(&TypeOf($actual))
    )
  };
}

/// Prepares null value with error message for third argument in `between` operator.
macro_rules! between_null3 {
  ($expected:literal, $actual:expr) => {
    value_null!(
      "expected {} as a third argument in 'between' operator, actual value type is {}",
      $expected,
      arg(&TypeOf($actual))
    )
  };
}

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

/// An ordering comparison on two values of one ordered kind; otherwise null with an error that
/// names the operator, the type it expected and the type it found.
macro_rules! ordering {
  ($(#[$meta:meta])* $name:ident, $operator:tt, $symbol:literal) => {
    $(#[$meta])*
    pub(crate) fn $name(lhv: Value, rhv: Value) -> Value {
      match (&lhv, &rhv) {
        (Value::Number(lh), Value::Number(rh)) => Value::Boolean(lh $operator rh),
        (Value::String(lh), Value::String(rh)) => Value::Boolean(lh $operator rh),
        (Value::Date(lh), Value::Date(rh)) => Value::Boolean(lh $operator rh),
        (Value::DateTime(lh), Value::DateTime(rh)) => Value::Boolean(lh $operator rh),
        (Value::Time(lh), Value::Time(rh)) => Value::Boolean(lh $operator rh),
        (Value::DaysAndTimeDuration(lh), Value::DaysAndTimeDuration(rh)) => Value::Boolean(lh $operator rh),
        (Value::YearsAndMonthsDuration(lh), Value::YearsAndMonthsDuration(rh)) => Value::Boolean(lh $operator rh),
        _ => unordered($symbol, &lhv, &rhv),
      }
    }
  };
}

ordering!(
  /// FEEL `lhv < rhv` on two values of one ordered kind; otherwise null with an error.
  compare_lt, <, "<"
);
ordering!(
  /// FEEL `lhv <= rhv` on two values of one ordered kind; otherwise null with an error.
  compare_le, <=, "<="
);
ordering!(
  /// FEEL `lhv > rhv` on two values of one ordered kind; otherwise null with an error.
  compare_gt, >, ">"
);
ordering!(
  /// FEEL `lhv >= rhv` on two values of one ordered kind; otherwise null with an error.
  compare_ge, >=, ">="
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

/// FEEL `lhv between mhv and rhv` (DMN 1.6 Table 55): the three values are evaluated before
/// any is checked; a null `lhv` is the result, and values of other kinds give an error.
pub(crate) fn between(lhv: Value, mhv: Value, rhv: Value) -> Value {
  match lhv {
    Value::Number(lh) => {
      if let Value::Number(mh) = mhv {
        if let Value::Number(rh) = rhv {
          Value::Boolean(mh <= lh && lh <= rh)
        } else {
          between_null3!("number", &rhv)
        }
      } else {
        between_null2!("number", &mhv)
      }
    }
    Value::String(lh) => {
      if let Value::String(mh) = mhv {
        if let Value::String(rh) = rhv {
          Value::Boolean(mh <= lh && lh <= rh)
        } else {
          between_null3!("string", &rhv)
        }
      } else {
        between_null2!("string", &mhv)
      }
    }
    Value::Date(lh) => {
      if let Value::Date(mh) = mhv {
        if let Value::Date(rh) = rhv {
          Value::Boolean(mh <= lh && lh <= rh)
        } else {
          between_null3!("date", &rhv)
        }
      } else {
        between_null2!("date", &mhv)
      }
    }
    Value::Time(lh) => {
      if let Value::Time(mh) = mhv {
        if let Value::Time(rh) = rhv {
          Value::Boolean(mh <= lh && lh <= rh)
        } else {
          between_null3!("time", &rhv)
        }
      } else {
        between_null2!("time", &mhv)
      }
    }
    Value::DateTime(lh) => {
      if let Value::DateTime(mh) = mhv {
        if let Value::DateTime(rh) = rhv {
          Value::Boolean(mh <= lh && lh <= rh)
        } else {
          between_null3!("date and time", &rhv)
        }
      } else {
        between_null2!("date and time", &mhv)
      }
    }
    Value::DaysAndTimeDuration(lh) => {
      if let Value::DaysAndTimeDuration(mh) = mhv {
        if let Value::DaysAndTimeDuration(rh) = rhv {
          Value::Boolean(mh <= lh && lh <= rh)
        } else {
          between_null3!("days and time duration", &rhv)
        }
      } else {
        between_null2!("days and time duration", &mhv)
      }
    }
    Value::YearsAndMonthsDuration(lh) => {
      if let Value::YearsAndMonthsDuration(mh) = mhv {
        if let Value::YearsAndMonthsDuration(rh) = rhv {
          Value::Boolean(mh <= lh && lh <= rh)
        } else {
          between_null3!("years and months duration", &rhv)
        }
      } else {
        between_null2!("years and months duration", &mhv)
      }
    }
    other => {
      if other.is_null() {
        other
      } else {
        value_null!("unexpected value type in 'between' operator: {}", arg(&TypeOf(&other)))
      }
    }
  }
}
