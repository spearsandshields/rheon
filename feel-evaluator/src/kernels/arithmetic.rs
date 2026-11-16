//! Arithmetic: the five binary operators and the negation of FEEL (DMN 1.6 Tables 57 to 63), behind
//! one entry, [Arithmetic::apply].

use super::temporal_value;
use crate::macros::invalid_argument_type;
use rheon_feel::message::arg;
use rheon_feel::values::{TypeOf, Value};
use rheon_feel::{FeelNumber, value_null};
use rheon_feel_temporal::{FeelDateTime, FeelDaysAndTimeDuration, FeelTime, FeelYearsAndMonthsDuration, FeelZone};

/// Replaces non-finite numbers (decimal128 infinity or NaN, produced e.g. by `9e6144 * 10` or
/// `1 / 1e-6176`) with null: FEEL has no such numbers.
pub(crate) fn finite(value: Value) -> Value {
  match value {
    Value::Number(n) if !n.is_finite() => value_null!("result is not a finite number (decimal overflow)"),
    other => other,
  }
}

/// The arithmetic operators, named and written as their errors name and write them.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Arithmetic {
  Addition,
  Subtraction,
  Multiplication,
  Division,
  Exponentiation,
  Negation,
}

impl Arithmetic {
  /// The value of a binary operator on two operands, the one entry of FEEL and B-FEEL arithmetic:
  /// the rule of a null operand ([Arithmetic::null_operand]), then the operator's kernel, whose
  /// result is never a non-finite number. Exponentiation reports a non-finite power itself.
  ///
  /// Out of line: the frame of an operator's evaluator is on the stack while an operand nested in
  /// it evaluates, so it holds the two operands and no locals of the kernel, and a deep chain of
  /// operators keeps a small frame per level.
  #[inline(never)]
  pub(crate) fn apply(self, lhv: Value, rhv: Value) -> Value {
    if let Some(null) = self.null_operand(&[&lhv, &rhv]) {
      return null;
    }
    match self {
      Arithmetic::Addition => finite(add(lhv, rhv)),
      Arithmetic::Subtraction => finite(subtract(lhv, rhv)),
      Arithmetic::Multiplication => finite(multiply(lhv, rhv)),
      Arithmetic::Division => finite(divide(lhv, rhv)),
      Arithmetic::Exponentiation => exponentiate(lhv, rhv),
      // a negation has one operand ([Arithmetic::negation])
      Arithmetic::Negation => self.incompatible(&[&lhv, &rhv]),
    }
  }

  /// The value of the negation of an operand: the rule of a null operand, then the kernel, whose
  /// result is never a non-finite number.
  #[inline(never)]
  pub(crate) fn negation(lhv: Value) -> Value {
    if let Some(null) = Arithmetic::Negation.null_operand(&[&lhv]) {
      return null;
    }
    finite(negate(lhv))
  }

  fn name(self) -> &'static str {
    match self {
      Arithmetic::Addition => "addition",
      Arithmetic::Subtraction => "subtraction",
      Arithmetic::Multiplication => "multiplication",
      Arithmetic::Division => "division",
      Arithmetic::Exponentiation => "exponentiation",
      Arithmetic::Negation => "negation",
    }
  }

  fn symbol(self) -> &'static str {
    match self {
      Arithmetic::Addition => "+",
      Arithmetic::Subtraction | Arithmetic::Negation => "-",
      Arithmetic::Multiplication => "*",
      Arithmetic::Division => "/",
      Arithmetic::Exponentiation => "**",
    }
  }

  /// The rule of a null operand, for every arithmetic operator and on either side: DMN 1.6, before
  /// Table 57 and before Table 59, "if input values are not of the listed types, the result is
  /// null", and the official TCK expects an error result for every arithmetic operation with a null
  /// operand (compliance-level-3/0100-arithmetic). With a null among `operands`, the result is
  /// [Arithmetic::incompatible], an error of the operator's own; a null that stands for an error
  /// of its own has counted it where it was created (§7.3.8). `None` when no operand is null.
  fn null_operand(self, operands: &[&Value]) -> Option<Value> {
    operands.iter().any(|operand| operand.is_null()).then(|| self.incompatible(operands))
  }

  /// The null of this operator on `operands` it does not take, with the error
  /// `[<name>] incompatible types: <operation>`, the operation written with its operands as
  /// evaluated (`1 - null`, `-null`).
  fn incompatible(self, operands: &[&Value]) -> Value {
    value_null!("[{}] incompatible types: {}", self.name(), Written(self, operands))
  }
}

/// An arithmetic operation as its error writes it: the operands infix, or after the symbol for a
/// single one.
struct Written<'a>(Arithmetic, &'a [&'a Value]);

impl std::fmt::Display for Written<'_> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let Written(operator, operands) = self;
    let symbol = operator.symbol();
    if let [operand] = operands {
      return write!(f, "{symbol}{}", arg(*operand));
    }
    for (index, operand) in operands.iter().enumerate() {
      if index > 0 {
        write!(f, " {symbol} ")?;
      }
      write!(f, "{}", arg(*operand))?;
    }
    Ok(())
  }
}

/// FEEL addition of two operands that are not null (DMN 1.6 Table 58).
fn add(lhv: Value, rhv: Value) -> Value {
  match lhv {
    Value::Number(lh) => match rhv {
      Value::Number(rh) => Value::Number(lh + rh),
      _ => value_null!(
        "incompatible types in addition: {}({}) + {}({})",
        arg(&lhv),
        arg(&TypeOf(&lhv)),
        arg(&rhv),
        arg(&TypeOf(&rhv))
      ),
    },
    Value::String(lh) => {
      if let Value::String(rh) = rhv {
        Value::String(lh.concat(&rh))
      } else {
        value_null!("expected string as a second argument in addition")
      }
    }
    Value::Date(lh) => match rhv {
      Value::DaysAndTimeDuration(rh) => {
        if let Some(result) = lh + rh {
          Value::Date(result)
        } else {
          value_null!("invalid result while adding days and time duration to date")
        }
      }
      Value::YearsAndMonthsDuration(rh) => {
        if let Some(a) = lh + rh {
          Value::Date(a)
        } else {
          value_null!("invalid result while adding years and months duration to date")
        }
      }
      other => invalid_argument_type!(module "builders", "add", "years and months duration", other),
    },
    Value::DateTime(lh) => match rhv {
      Value::DaysAndTimeDuration(rh) => {
        if let Some(a) = lh + rh {
          Value::DateTime(a)
        } else {
          value_null!("invalid result while adding days and time duration to date and time")
        }
      }
      Value::YearsAndMonthsDuration(rh) => {
        if let Some(a) = lh + rh {
          Value::DateTime(a)
        } else {
          value_null!("invalid result while adding years and months duration to date and time")
        }
      }
      other => invalid_argument_type!(module "builders", "add", "days and time duration, years and months duration", other),
    },
    Value::Time(lh) => match rhv {
      Value::DaysAndTimeDuration(rh) => Value::Time(lh + rh),
      other => invalid_argument_type!(module "builders", "add", "days and time duration", other),
    },
    Value::DaysAndTimeDuration(lh) => match rhv {
      Value::DaysAndTimeDuration(rh) => lh
        .checked_add(&rh)
        .map_or_else(|| value_null!("days and time duration addition overflow"), Value::DaysAndTimeDuration),
      Value::Date(rh) => {
        if let Some(result) = rh + lh {
          Value::Date(result)
        } else {
          value_null!("invalid result while adding date to days and time duration")
        }
      }
      Value::DateTime(rh) => {
        if let Some(a) = rh + lh {
          Value::DateTime(a)
        } else {
          value_null!("invalid result while adding date and time to days and time duration")
        }
      }
      Value::Time(rh) => Value::Time(rh + lh),
      other => invalid_argument_type!(module "builders", "add", "days and time duration, date and time", other),
    },
    Value::YearsAndMonthsDuration(lh) => match rhv {
      Value::Date(rh) => {
        if let Some(a) = rh + lh {
          Value::Date(a)
        } else {
          value_null!("invalid result while adding date to years and months duration")
        }
      }
      Value::DateTime(rh) => {
        if let Some(a) = rh + lh {
          Value::DateTime(a)
        } else {
          value_null!("invalid result while adding date and time to years and months duration")
        }
      }
      Value::YearsAndMonthsDuration(rh) => lh
        .checked_add(&rh)
        .map_or_else(|| value_null!("years and months duration addition overflow"), Value::YearsAndMonthsDuration),
      other => invalid_argument_type!(module "builders", "add", "years and months duration, date and time", other),
    },
    other => invalid_argument_type!(module "builders", "add", "number, string, date and time, days and time duration, years and months duration, null", other),
  }
}

/// FEEL division of two operands that are not null (DMN 1.6 Table 60).
fn divide(lhv: Value, rhv: Value) -> Value {
  match lhv {
    Value::Number(lh) => match rhv {
      Value::Number(rh) => {
        if rh.abs() == FeelNumber::zero() {
          value_null!("[division] division by zero")
        } else {
          Value::Number(lh / rh)
        }
      }
      _ => Arithmetic::Division.incompatible(&[&lhv, &rhv]),
    },
    Value::DaysAndTimeDuration(ref lh) => match rhv {
      Value::Number(rh) => {
        if rh.is_zero() {
          value_null!("[division] division by zero")
        } else {
          let lv = FeelNumber::from(lh.as_nanos()) / rh;
          if let Ok(v) = FeelNumber::try_into(lv)
            && let Some(duration) = FeelDaysAndTimeDuration::from_n(v)
          {
            Value::DaysAndTimeDuration(duration)
          } else {
            value_null!("[division] error: {} / {}", arg(&lhv), arg(&rhv))
          }
        }
      }
      Value::DaysAndTimeDuration(rh) => {
        if rh.as_nanos() == 0 {
          value_null!("[division] division by zero")
        } else {
          let lvl = FeelNumber::from(lh.as_nanos());
          let rvl = FeelNumber::from(rh.as_nanos());
          Value::Number(lvl / rvl)
        }
      }
      _ => Arithmetic::Division.incompatible(&[&lhv, &rhv]),
    },
    Value::YearsAndMonthsDuration(ref lh) => match rhv {
      Value::Number(rh) => {
        if rh.is_zero() {
          value_null!("[division] division by zero")
        } else {
          match temporal_value::ym_duration(FeelNumber::from(lh.as_months()) / rh, &lhv, '/', &rhv) {
            Some(duration) => Value::YearsAndMonthsDuration(duration),
            None => value_null!("[division] error: {} / {}", arg(&lhv), arg(&rhv)),
          }
        }
      }
      Value::YearsAndMonthsDuration(rh) => {
        if rh.as_months() == 0 {
          value_null!("[division] division by zero")
        } else {
          let lvl = FeelNumber::from(lh.as_months());
          let rvl = FeelNumber::from(rh.as_months());
          Value::Number(lvl / rvl)
        }
      }
      _ => Arithmetic::Division.incompatible(&[&lhv, &rhv]),
    },
    _ => Arithmetic::Division.incompatible(&[&lhv, &rhv]),
  }
}

/// FEEL exponentiation of two operands that are not null (DMN 1.6 Table 61).
fn exponentiate(lhv: Value, rhv: Value) -> Value {
  if let Value::Number(lh) = lhv {
    if let Value::Number(rh) = rhv {
      // DMN 1.6 Table 61: "value(e2) is a number in the range [-999,999,999..999,999,999]",
      // checked before any power, also where the power would not depend on the exponent
      if rh.abs() > FeelNumber::from(999_999_999) {
        value_null!("exponentiation exponent is out of range -999999999..999999999")
      } else if let Some(result) = lh.pow(&rh) {
        Value::Number(result)
      } else {
        value_null!("exponentiation result is not a finite number")
      }
    } else {
      value_null!("exponentiation exponent is not a number")
    }
  } else {
    value_null!("exponentiation base is not a number")
  }
}

/// FEEL multiplication of two operands that are not null (DMN 1.6 Table 60).
fn multiply(lhv: Value, rhv: Value) -> Value {
  match lhv {
    Value::Number(lh) => match rhv {
      Value::Number(rh) => Value::Number(lh * rh),
      Value::DaysAndTimeDuration(ref rh) => {
        let val = lh * FeelNumber::from(rh.as_nanos());
        if let Ok(v) = FeelNumber::try_into(val)
          && let Some(duration) = FeelDaysAndTimeDuration::from_n(v)
        {
          Value::DaysAndTimeDuration(duration)
        } else {
          value_null!("multiplication result is out of range of days and time duration")
        }
      }
      Value::YearsAndMonthsDuration(ref rh) => ym_product(temporal_value::ym_duration(lh * FeelNumber::from(rh.as_months()), &lhv, '*', &rhv)),
      _ => Arithmetic::Multiplication.incompatible(&[&lhv, &rhv]),
    },
    Value::DaysAndTimeDuration(ref lh) => match rhv {
      Value::Number(rh) => {
        let val = FeelNumber::from(lh.as_nanos()) * rh;
        if let Ok(v) = FeelNumber::try_into(val)
          && let Some(duration) = FeelDaysAndTimeDuration::from_n(v)
        {
          Value::DaysAndTimeDuration(duration)
        } else {
          value_null!("multiplication result is out of range of days and time duration")
        }
      }
      _ => Arithmetic::Multiplication.incompatible(&[&lhv, &rhv]),
    },
    Value::YearsAndMonthsDuration(ref lh) => match rhv {
      Value::Number(rh) => ym_product(temporal_value::ym_duration(FeelNumber::from(lh.as_months()) * rh, &lhv, '*', &rhv)),
      _ => Arithmetic::Multiplication.incompatible(&[&lhv, &rhv]),
    },
    other => value_null!("unexpected value type in multiplication: {}", arg(&TypeOf(&other))),
  }
}

/// FEEL subtraction of two operands that are not null (DMN 1.6 Table 58).
fn subtract(lhv: Value, rhv: Value) -> Value {
  // the operands stay available for the message below: temporal values are copied only in
  // the arm that subtracts them
  match &lhv {
    Value::Number(lh) => {
      if let Value::Number(rh) = &rhv {
        return Value::Number(*lh - *rh);
      }
    }
    Value::Date(lh) => match &rhv {
      Value::Date(rh) => {
        let l = FeelDateTime::new(lh.clone(), FeelTime::midnight(FeelZone::Utc));
        let r = FeelDateTime::new(rh.clone(), FeelTime::midnight(FeelZone::Utc));
        if let Some(result) = l - r {
          return Value::DaysAndTimeDuration(result);
        }
      }
      Value::DateTime(rh) => {
        let l = FeelDateTime::new(lh.clone(), FeelTime::midnight(FeelZone::Utc));
        if let Some(result) = l - rh.clone() {
          return Value::DaysAndTimeDuration(result);
        }
      }
      Value::DaysAndTimeDuration(rh) => {
        if let Some(date) = lh.clone() - rh.clone() {
          return Value::Date(date);
        }
      }
      Value::YearsAndMonthsDuration(rh) => {
        if let Some(date) = lh.clone() - rh.clone() {
          return Value::Date(date);
        }
      }
      _ => {}
    },
    Value::Time(lh) => match &rhv {
      Value::Time(rh) => {
        if let Some(result) = lh.clone() - rh.clone() {
          return Value::DaysAndTimeDuration(result);
        }
      }
      Value::DaysAndTimeDuration(rh) => {
        return Value::Time(lh.clone() - rh.clone());
      }
      _ => {}
    },
    Value::DateTime(lh) => match &rhv {
      Value::Date(rh) => {
        let r = FeelDateTime::new(rh.clone(), FeelTime::midnight(FeelZone::Utc));
        if let Some(result) = lh.clone() - r {
          return Value::DaysAndTimeDuration(result);
        }
      }
      Value::DateTime(rh) => {
        if let Some(result) = lh.clone() - rh.clone() {
          return Value::DaysAndTimeDuration(result);
        }
      }
      Value::DaysAndTimeDuration(rh) => {
        if let Some(result) = lh.clone() - rh.clone() {
          return Value::DateTime(result);
        }
      }
      Value::YearsAndMonthsDuration(rh) => {
        if let Some(result) = lh.clone() - rh.clone() {
          return Value::DateTime(result);
        }
      }
      _ => {}
    },
    Value::DaysAndTimeDuration(lh) => {
      if let Value::DaysAndTimeDuration(rh) = &rhv {
        return lh
          .checked_sub(rh)
          .map_or_else(|| value_null!("days and time duration subtraction overflow"), Value::DaysAndTimeDuration);
      }
    }
    Value::YearsAndMonthsDuration(lh) => {
      if let Value::YearsAndMonthsDuration(rh) = &rhv {
        return lh
          .checked_sub(rh)
          .map_or_else(|| value_null!("years and months duration subtraction overflow"), Value::YearsAndMonthsDuration);
      }
    }
    _ => {}
  }
  // the operands as evaluated above: evaluating them again would repeat their diagnostics
  // and take exponential time in a chain of failing subtractions
  Arithmetic::Subtraction.incompatible(&[&lhv, &rhv])
}

/// FEEL negation of an operand that is not null (DMN 1.6 Table 63).
fn negate(lhv: Value) -> Value {
  match lhv {
    Value::Number(lh) => Value::Number(-lh),
    Value::DaysAndTimeDuration(lh) => Value::DaysAndTimeDuration(-lh),
    Value::YearsAndMonthsDuration(lh) => Value::YearsAndMonthsDuration(-lh),
    other => value_null!("unexpected type in arithmetic negation: {}", arg(&TypeOf(&other))),
  }
}

/// The value of a years and months duration times a number: the duration [ym_duration](super::temporal_value::ym_duration)
/// gives, or null beyond the range.
fn ym_product(duration: Option<FeelYearsAndMonthsDuration>) -> Value {
  match duration {
    Some(duration) => Value::YearsAndMonthsDuration(duration),
    None => value_null!("multiplication result is out of range of years and months duration"),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use rheon_feel::diagnostics;
  use rheon_feel_temporal::FeelDate;

  /// The error of an operand that addition does not take is named `[builders::add]`.
  #[test]
  fn an_invalid_operand_of_addition_is_named_builders_add() {
    let date = Value::Date(FeelDate::new(2026, 11, 16).unwrap());
    let (value, collected) = diagnostics::collect(|| add(date, Value::Boolean(true)));
    let expected = "[builders::add] invalid argument type, expected years and months duration, actual type is boolean";
    assert_eq!(Value::Null(Some(expected.to_string())), value);
    assert_eq!(vec![expected.to_string()], collected.messages);
  }
}
