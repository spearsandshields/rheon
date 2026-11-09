//! Conversion of request input (JSON or FEEL context literal) into FEEL values.

use rheon_feel::context::FeelContext;
use rheon_feel::values::Value;
use rheon_feel::{FeelNumber, FeelScope, Name};
use serde_json::Value as Json;

/// Maximum number of significant digits of a decimal128 (BID128) number.
const MAX_DIGITS: usize = 34;

/// Converts a JSON value into a FEEL value.
///
/// Numbers are parsed from their exact JSON source text (serde_json `arbitrary_precision`),
/// never through `f64`. Numbers with more than 34 significant digits are rounded by the decimal
/// library; a warning is pushed in that case. `path` is used in error/warning messages.
pub fn json_to_value(json: &Json, path: &str, warnings: &mut Vec<String>) -> Result<Value, String> {
  Ok(match json {
    Json::Null => Value::Null(None),
    Json::Bool(b) => Value::Boolean(*b),
    Json::String(s) => Value::String(s.clone()),
    Json::Number(n) => {
      let text = n.to_string();
      let (number, rounded) = FeelNumber::from_str_reporting_rounding(&text).map_err(|e| format!("{path}: invalid number '{text}': {e}"))?;
      if rounded {
        warnings.push(if number.is_zero() {
          format!("{path}: number {text} is too small for a 34-digit decimal (exponent below -6176) and became 0")
        } else if significant_digits(&text) > MAX_DIGITS {
          format!("{path}: number {text} has more than {MAX_DIGITS} significant digits and was rounded to {number}")
        } else {
          format!("{path}: number {text} is too small for a 34-digit decimal and was rounded to {number}")
        });
      }
      Value::Number(number)
    }
    Json::Array(items) => {
      let mut values = Vec::with_capacity(items.len());
      for (i, item) in items.iter().enumerate() {
        values.push(json_to_value(item, &format!("{path}[{i}]"), warnings)?);
      }
      Value::List(values)
    }
    Json::Object(map) => Value::Context(json_object_to_context(map, path, warnings)?),
  })
}

/// Converts a JSON object into a FEEL context.
///
/// Keys become FEEL names, which trims leading/trailing whitespace (`" x "` is `x`); inner
/// whitespace is kept as is (`"a  b"` is not the same name as `a b`).
pub fn json_object_to_context(map: &serde_json::Map<String, Json>, path: &str, warnings: &mut Vec<String>) -> Result<FeelContext, String> {
  let mut ctx = FeelContext::default();
  for (key, value) in map {
    let child_path = if path.is_empty() { key.clone() } else { format!("{path}.{key}") };
    let name = Name::from(key.as_str());
    if name.is_empty() {
      return Err(format!("{}: empty context key", if path.is_empty() { "context" } else { path }));
    }
    ctx.set_entry(&name, json_to_value(value, &child_path, warnings)?);
  }
  Ok(ctx)
}

/// Evaluates a FEEL context literal (e.g. `{a: 1, "Grace's age": 40}`) into a FEEL context.
/// An empty / blank string is an empty context.
pub fn feel_context(text: &str) -> Result<FeelContext, String> {
  if text.trim().is_empty() {
    return Ok(FeelContext::default());
  }
  let scope = FeelScope::default();
  let node = rheon_feel_parser::parse_context(&scope, text, false).map_err(|e| format!("invalid FEEL context: {e}"))?;
  crate::engine::check_ast_depth(&node).map_err(|e| format!("invalid FEEL context: {e}"))?;
  rheon_feel_evaluator::reset_call_depth();
  let value = rheon_feel_evaluator::evaluate(&scope, &node);
  if rheon_feel_evaluator::call_depth_exceeded() {
    return Err(crate::engine::call_depth_message());
  }
  match value {
    Value::Context(ctx) => Ok(ctx),
    Value::Null(Some(reason)) => Err(format!("FEEL context evaluated to null: {reason}")),
    other => Err(format!("FEEL context literal expected, it evaluated to {} ({})", other, other.type_of())),
  }
}

/// Counts significant digits in a JSON number literal (ignoring sign, exponent,
/// leading zeros and trailing zeros).
fn significant_digits(text: &str) -> usize {
  let mantissa = text.split(['e', 'E']).next().unwrap_or("");
  let digits: String = mantissa.chars().filter(|c| c.is_ascii_digit()).collect();
  digits.trim_start_matches('0').trim_end_matches('0').len()
}

#[cfg(test)]
mod tests {
  use super::significant_digits;

  #[test]
  fn counts_significant_digits() {
    assert_eq!(significant_digits("0.000120"), 2);
    assert_eq!(significant_digits("-1.5e10"), 2);
    assert_eq!(significant_digits("100"), 1);
    assert_eq!(significant_digits("0"), 0);
    assert_eq!(significant_digits("12345678901234567890123456789012345"), 35);
  }
}
