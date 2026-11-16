//! The product of `edge-operands.json`: every `unary` template applied to every operand and every
//! `binary` template to every ordered pair of operands.

use serde_json::Value as Json;

/// The texts of a JSON array of strings.
fn texts<'a>(product: &'a Json, key: &str) -> Vec<&'a str> {
  product[key]
    .as_array()
    .unwrap_or_else(|| panic!("edge-operands.json has no array `{key}`"))
    .iter()
    .map(|text| text.as_str().expect("a template or an operand is a string"))
    .collect()
}

/// `template` with every `{a}` replaced by `a` and every `{b}` by `b`, in one pass, so an operand
/// is never read as a placeholder.
fn instantiate(template: &str, a: &str, b: &str) -> String {
  let mut expression = String::new();
  let mut rest = template;
  while let Some(start) = rest.find('{') {
    expression.push_str(&rest[..start]);
    rest = &rest[start..];
    if let Some(after) = rest.strip_prefix("{a}") {
      expression.push_str(a);
      rest = after;
    } else if let Some(after) = rest.strip_prefix("{b}") {
      expression.push_str(b);
      rest = after;
    } else {
      expression.push('{');
      rest = &rest[1..];
    }
  }
  expression.push_str(rest);
  expression
}

/// `edge-operands.json` is a deterministic product over operand classes at the engine's edges:
/// numbers at the limits of the 64-bit integers, of 32-bit integers and of decimal128, both
/// duration kinds at the ends of their range and at one unit, dates and date-times at the ends of
/// the supported range and in daylight saving gaps and overlaps, times at the ends of the day,
/// and null, the empty string, list and context. Every `unary` template is applied to every
/// operand and every `binary` template to every ordered pair; the `binary_full` templates are
/// added when the full size runs.
pub fn expressions(full: bool) -> Vec<String> {
  let product: Json = serde_json::from_str(include_str!("edge-operands.json")).expect("edge-operands.json is a JSON object");
  let operands = texts(&product, "operands");
  let mut binary = texts(&product, "binary");
  if full {
    binary.extend(texts(&product, "binary_full"));
  }
  let mut expressions = vec![];
  for template in texts(&product, "unary") {
    for a in &operands {
      expressions.push(instantiate(template, a, ""));
    }
  }
  for template in &binary {
    for a in &operands {
      for b in &operands {
        expressions.push(instantiate(template, a, b));
      }
    }
  }
  expressions
}
