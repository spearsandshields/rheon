//! Literals: numbers, `@` literals, lists, ranges and the entries of context literals.

use rheon_feel::message::arg;
use rheon_feel::values::{Value, truncated_fraction_warning};
use rheon_feel::{FeelNumber, Name, diagnostics, value_null};
use rheon_feel_temporal::{FeelDate, FeelDateTime, FeelDaysAndTimeDuration, FeelTime, FeelYearsAndMonthsDuration};
use std::fmt::Write;
use std::str::FromStr;

/// The number that a numeric literal writes with the digits `before` and `after` its point and the
/// exponent `sign` and digits, with the warning that its evaluation records when a number does
/// not hold it exactly (more than 34 significant digits, or too small) and it was rounded; or
/// the error text of a literal that is no number.
pub(crate) fn number_literal(before: &str, after: &str, sign: char, exponent: &str) -> Result<(FeelNumber, Option<String>), String> {
  let mut text = String::new();
  let _ = write!(&mut text, "{before}");
  if !after.is_empty() {
    let _ = write!(&mut text, ".{after}");
  }
  if !exponent.is_empty() {
    let _ = write!(&mut text, "e{sign}{exponent}");
  }
  let (number, rounded) = FeelNumber::from_str_reporting_rounding(&text).map_err(|_| format!("failed to convert text '{}' into number", arg(&text)))?;
  let warning = rounded.then(|| {
    // the digits of the literal without the zeros at its ends
    let digits: String = before.chars().chain(after.chars()).filter(char::is_ascii_digit).collect();
    let significant = digits.trim_start_matches('0').trim_end_matches('0').len();
    if number.is_zero() {
      format!("number literal '{}' is too small for a 34-digit decimal (exponent below -6176) and became 0", arg(&text))
    } else if significant > 34 {
      format!("number literal '{}' has more than 34 significant digits and was rounded to {}", arg(&text), arg(&number))
    } else {
      format!("number literal '{}' is too small for a 34-digit decimal and was rounded to {}", arg(&text), arg(&number))
    }
  });
  Ok((number, warning))
}

/// The value of the `@` literal `text` and the warning that its evaluation records when fraction
/// digits beyond nanoseconds were truncated, or the error text of a literal that has no value.
pub(crate) fn at_literal(text: &str) -> Result<(Value, Option<String>), String> {
  let truncated = |value: Value, dropped: Option<&str>| (value, dropped.map(|dropped| truncated_fraction_warning(text, dropped)));
  if let Ok(date) = FeelDate::from_str(text) {
    Ok((Value::Date(date), None))
  } else if let Ok((date_time, dropped)) = FeelDateTime::parse_reporting_truncation(text) {
    Ok(truncated(Value::DateTime(date_time), dropped))
  } else if let Ok((time, dropped)) = FeelTime::parse_reporting_truncation(text) {
    Ok(truncated(Value::Time(time), dropped))
  } else if let Ok(ym_duration) = FeelYearsAndMonthsDuration::try_from(text) {
    Ok((Value::YearsAndMonthsDuration(ym_duration), None))
  } else if let Ok((dt_duration, dropped)) = FeelDaysAndTimeDuration::parse_reporting_truncation(text) {
    Ok(truncated(Value::DaysAndTimeDuration(dt_duration), dropped))
  } else {
    Err(format!("invalid @ literal: {}", arg(text)))
  }
}

/// The list of the evaluated items `values`, nested no deeper than values may nest.
pub(crate) fn list_of(values: Vec<Value>) -> Value {
  diagnostics::bound_depth(Value::List(values.into()))
}

/// The range of a range literal from its evaluated start and end.
pub(crate) fn range_of(lhv: Value, rhv: Value) -> Value {
  if let Value::IntervalStart(lhv, l_closed) = lhv {
    if let Value::IntervalEnd(rhv, r_closed) = rhv {
      diagnostics::bound_depth(Value::Range(lhv, l_closed, rhv, r_closed))
    } else {
      value_null!("expected interval end")
    }
  } else {
    value_null!("expected interval start")
  }
}

/// The error of an entry of a context literal whose key an earlier entry has.
pub(crate) fn err_duplicated_context_entry_key(name: &Name) -> String {
  format!("duplicated context entry key: {}", arg(name))
}

/// The error of an entry of a context literal that needs its own value through other entries.
pub(crate) fn err_circular_context_entry(name: &Name) -> String {
  format!(
    "the context entry {} needs its own value: the entries of a context must not depend on each other in a circle",
    arg(name)
  )
}
