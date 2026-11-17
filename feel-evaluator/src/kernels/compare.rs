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

/// FEEL `lhv < rhv` on two values of one ordered kind; otherwise null with an error.
pub(crate) fn compare_lt(lhv: Value, rhv: Value) -> Value {
  match lhv {
    Value::Number(lh) => match rhv {
      Value::Number(rh) => Value::Boolean(lh < rh),
      _ => value_null!("eval_less_then_number"),
    },
    Value::String(lh) => match rhv {
      Value::String(rh) => Value::Boolean(lh < rh),
      _ => value_null!("eval_less_then_string"),
    },
    Value::Date(lh) => match rhv {
      Value::Date(rh) => Value::Boolean(lh < rh),
      _ => value_null!("eval_less_then_date"),
    },
    Value::DateTime(lh) => match rhv {
      Value::DateTime(rh) => Value::Boolean(lh < rh),
      _ => value_null!("eval_less_then_date_time"),
    },
    Value::Time(lh) => match rhv {
      Value::Time(rh) => Value::Boolean(lh < rh),
      _ => value_null!("eval_less_then_time"),
    },
    Value::DaysAndTimeDuration(lh) => match rhv {
      Value::DaysAndTimeDuration(rh) => Value::Boolean(lh < rh),
      _ => value_null!("eval_less_then_days_and_time_duration"),
    },
    Value::YearsAndMonthsDuration(lh) => match rhv {
      Value::YearsAndMonthsDuration(rh) => Value::Boolean(lh < rh),
      _ => value_null!("eval_less_then_years_and_months_duration"),
    },
    _ => value_null!("eval_less_then"),
  }
}

/// FEEL `lhv <= rhv` on two values of one ordered kind; otherwise null with an error.
pub(crate) fn compare_le(lhv: Value, rhv: Value) -> Value {
  match lhv {
    Value::Number(lh) => match rhv {
      Value::Number(rh) => Value::Boolean(lh <= rh),
      _ => value_null!("eval_less_or_equal_number"),
    },
    Value::String(lh) => match rhv {
      Value::String(rh) => Value::Boolean(lh <= rh),
      _ => value_null!("eval_less_or_equal_string"),
    },
    Value::Date(lh) => match rhv {
      Value::Date(rh) => Value::Boolean(lh <= rh),
      _ => value_null!("eval_less_or_equal_date"),
    },
    Value::DateTime(lh) => match rhv {
      Value::DateTime(rh) => Value::Boolean(lh <= rh),
      _ => value_null!("eval_less_or_equal_date_time"),
    },
    Value::Time(lh) => match rhv {
      Value::Time(rh) => Value::Boolean(lh <= rh),
      _ => value_null!("eval_less_or_equal_time"),
    },
    Value::DaysAndTimeDuration(lh) => match rhv {
      Value::DaysAndTimeDuration(rh) => Value::Boolean(lh <= rh),
      _ => value_null!("eval_less_or_equal_days_and_time_duration"),
    },
    Value::YearsAndMonthsDuration(lh) => match rhv {
      Value::YearsAndMonthsDuration(rh) => Value::Boolean(lh <= rh),
      _ => value_null!("eval_less_or_equal_years_and_months_duration"),
    },
    _ => value_null!("eval_less_or_equal"),
  }
}

/// FEEL `lhv > rhv` on two values of one ordered kind; otherwise null with an error.
pub(crate) fn compare_gt(lhv: Value, rhv: Value) -> Value {
  match lhv {
    Value::Number(lh) => match rhv {
      Value::Number(rh) => Value::Boolean(lh > rh),
      _ => value_null!("eval_greater_then_number"),
    },
    Value::String(lh) => match rhv {
      Value::String(rh) => Value::Boolean(lh > rh),
      _ => value_null!("eval_greater_then_string"),
    },
    Value::Date(lh) => match rhv {
      Value::Date(rh) => Value::Boolean(lh > rh),
      _ => value_null!("eval_greater_then_date"),
    },
    Value::DateTime(lh) => match rhv {
      Value::DateTime(rh) => Value::Boolean(lh > rh),
      _ => value_null!("eval_greater_then_date_time"),
    },
    Value::Time(lh) => match rhv {
      Value::Time(rh) => Value::Boolean(lh > rh),
      _ => value_null!("eval_greater_then_time"),
    },
    Value::DaysAndTimeDuration(lh) => match rhv {
      Value::DaysAndTimeDuration(rh) => Value::Boolean(lh > rh),
      _ => value_null!("eval_greater_days_and_time_duration"),
    },
    Value::YearsAndMonthsDuration(lh) => match rhv {
      Value::YearsAndMonthsDuration(rh) => Value::Boolean(lh > rh),
      _ => value_null!("eval_greater_years_and_months_duration"),
    },
    _ => value_null!("eval_greater_then"),
  }
}

/// FEEL `lhv >= rhv` on two values of one ordered kind; otherwise null with an error.
pub(crate) fn compare_ge(lhv: Value, rhv: Value) -> Value {
  match lhv {
    Value::Number(lh) => match rhv {
      Value::Number(rh) => Value::Boolean(lh >= rh),
      _ => value_null!("eval_greater_or_equal_number"),
    },
    Value::String(lh) => match rhv {
      Value::String(rh) => Value::Boolean(lh >= rh),
      _ => value_null!("eval_greater_or_equal_string"),
    },
    Value::Date(lh) => match rhv {
      Value::Date(rh) => Value::Boolean(lh >= rh),
      _ => value_null!("eval_greater_or_equal_date"),
    },
    Value::DateTime(lh) => match rhv {
      Value::DateTime(rh) => Value::Boolean(lh >= rh),
      _ => value_null!("eval_greater_or_equal_date_time"),
    },
    Value::Time(lh) => match rhv {
      Value::Time(rh) => Value::Boolean(lh >= rh),
      _ => value_null!("eval_greater_or_equal_time"),
    },
    Value::DaysAndTimeDuration(lh) => match rhv {
      Value::DaysAndTimeDuration(rh) => Value::Boolean(lh >= rh),
      _ => value_null!("eval_greater_or_equal_days_and_time_duration"),
    },
    Value::YearsAndMonthsDuration(lh) => match rhv {
      Value::YearsAndMonthsDuration(rh) => Value::Boolean(lh >= rh),
      _ => value_null!("eval_greater_or_equal_years_and_months_duration"),
    },
    _ => value_null!("eval_greater_or_equal"),
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
