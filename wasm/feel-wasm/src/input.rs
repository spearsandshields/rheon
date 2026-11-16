//! Conversion of request input (JSON or FEEL context literal) into FEEL values.

use rheon_feel::context::FeelContext;
use rheon_feel::diagnostics;
use rheon_feel::message::{MessageArg, Size, arg};
use rheon_feel::values::{TypeOf, Value};
use rheon_feel::{FeelNumber, FeelScope, Name};
use serde_json::{Map, Value as Json};
use std::fmt;
use std::sync::Arc;

/// Maximum number of significant digits of a decimal128 (BID128) number.
const MAX_DIGITS: usize = 34;

/// Where a JSON value is in the request, for the messages about it (`items[3].price`).
///
/// The text is built only when a message needs it.
#[derive(Clone, Copy)]
pub enum Path<'a> {
  /// The top: `""` for the context, `"input"` for the input of unary tests.
  Root(&'a str),
  /// An entry of an object, under its key as written in the JSON text.
  Key(&'a Path<'a>, &'a str),
  /// An item of an array.
  Index(&'a Path<'a>, usize),
}

impl fmt::Display for Path<'_> {
  /// `key` under the root context, `parent.key` elsewhere, `parent[index]` for array items.
  ///
  /// The steps from this path up to its root are collected first and then written root first,
  /// so the depth of the path costs no recursion and no text per level.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut steps = vec![self];
    let mut path = self;
    while let Path::Key(parent, _) | Path::Index(parent, _) = path {
      path = parent;
      steps.push(path);
    }
    // whether nothing but empty text has been written so far: a key then needs no dot before it
    let mut empty = true;
    for step in steps.into_iter().rev() {
      match step {
        Path::Root(root) => {
          f.write_str(root)?;
          empty = root.is_empty();
        }
        Path::Key(_, key) => {
          if !empty {
            f.write_str(".")?;
          }
          f.write_str(key)?;
          empty &= key.is_empty();
        }
        Path::Index(_, index) => {
          write!(f, "[{index}]")?;
          empty = false;
        }
      }
    }
    Ok(())
  }
}

impl Path<'_> {
  /// The number of characters of the path's text, counted as it is written, without building it.
  fn chars(&self) -> usize {
    struct Count(usize);
    impl fmt::Write for Count {
      fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0 += s.chars().count();
        Ok(())
      }
    }
    let mut count = Count(0);
    // counting never refuses a write
    let _ = fmt::write(&mut count, format_args!("{self}"));
    count.0
  }
}

impl MessageArg for Path<'_> {
  fn write_message(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Display::fmt(self, f)
  }

  fn size(&self) -> Option<Size> {
    Some(Size::Characters(self.chars()))
  }
}

/// Converts a JSON value into a FEEL value, taking it (strings are moved, not copied).
///
/// Numbers are parsed from their exact JSON source text (serde_json `arbitrary_precision`),
/// never through `f64`. Numbers with more than 34 significant digits are rounded by the decimal
/// library; a warning is pushed in that case. `path` is used in error/warning messages.
pub fn json_into_value(json: Json, path: &Path, warnings: &mut Vec<String>) -> Result<Value, String> {
  Ok(match json {
    Json::Null => Value::Null(None),
    Json::Bool(b) => Value::Boolean(b),
    Json::String(s) => Value::String(s.into()),
    Json::Number(n) => {
      let text = n.as_str();
      let (number, rounded) = FeelNumber::from_str_reporting_rounding(text).map_err(|e| format!("{}: invalid number '{}': {e}", arg(path), arg(text)))?;
      if rounded {
        let (place, quoted) = (arg(path), arg(text));
        warnings.push(if number.is_zero() {
          format!("{place}: number {quoted} is too small for a 34-digit decimal (exponent below -6176) and became 0")
        } else if significant_digits(text) > MAX_DIGITS {
          format!("{place}: number {quoted} has more than {MAX_DIGITS} significant digits and was rounded to {}", arg(&number))
        } else {
          format!("{place}: number {quoted} is too small for a 34-digit decimal and was rounded to {}", arg(&number))
        });
      }
      Value::Number(number)
    }
    Json::Array(items) => {
      let mut values = Vec::with_capacity(items.len());
      for (i, item) in items.into_iter().enumerate() {
        values.push(json_into_value(item, &Path::Index(path, i), warnings)?);
      }
      Value::List(values.into())
    }
    Json::Object(map) => json_object_into_context(map, path, warnings)?.into(),
  })
}

/// The largest number of entries of a JSON object that are inserted into their context one by
/// one: up to here the entries are one node of the context's map, and an insert is cheaper than
/// collecting the entries first (measured: equal at 8 entries, 3% apart at 12).
const DIRECT_ENTRIES: usize = 11;

/// Converts a JSON object into a FEEL context, taking it.
///
/// Keys become FEEL names, which trims leading/trailing whitespace (`" x "` is `x`); inner
/// whitespace is kept as is (`"a  b"` is not the same name as `a b`). A key that trimming leaves
/// unchanged becomes the name itself, without a copy. Of two keys that are one name after
/// trimming, the later one in the order of the keys gives the entry.
///
/// The entries come in the order of the keys, which is the order of the names unless a key was
/// trimmed, so a context of more than [DIRECT_ENTRIES] entries is built from them in one pass
/// ([FeelContext]'s `FromIterator`), without a search per entry.
pub fn json_object_into_context(map: Map<String, Json>, path: &Path, warnings: &mut Vec<String>) -> Result<FeelContext, String> {
  let direct = map.len() <= DIRECT_ENTRIES;
  let mut ctx = FeelContext::default();
  let mut entries = Vec::with_capacity(if direct { 0 } else { map.len() });
  for (key, value) in map {
    let (name, untrimmed_key) = if key.trim().len() == key.len() {
      (Name::from(key), None)
    } else {
      (Name::from(key.as_str()), Some(key))
    };
    if name.is_empty() {
      let context = Path::Root("context");
      return Err(format!("{}: empty context key", arg(if path.chars() == 0 { &context } else { path })));
    }
    let key = untrimmed_key.as_deref().unwrap_or(name.as_str());
    let value = json_into_value(value, &Path::Key(path, key), warnings)?;
    if direct {
      ctx.insert(name, value);
    } else {
      entries.push((name, value));
    }
  }
  Ok(if direct { ctx } else { entries.into_iter().collect() })
}

/// Evaluates a FEEL context literal (e.g. `{a: 1, "Grace's age": 40}`) into a FEEL context,
/// adding the warnings of its evaluation to `warnings`. An empty / blank string is an empty
/// context.
pub fn feel_context(text: &str, warnings: &mut Vec<String>) -> Result<FeelContext, String> {
  if text.trim().is_empty() {
    return Ok(FeelContext::default());
  }
  let scope = FeelScope::default();
  let node = rheon_feel_parser::parse_context(&scope, text, false).map_err(|e| format!("invalid FEEL context: {e}"))?;
  crate::engine::check_ast_depth(&node).map_err(|e| format!("invalid FEEL context: {e}"))?;
  // the context's errors are not counted in the response, only its warnings are reported
  let (outcome, diagnostics) = diagnostics::collect(|| rheon_feel_evaluator::evaluate(&scope, &node));
  let value = outcome.into_result().map_err(|abort| abort.to_string())?;
  warnings.extend_from_slice(diagnostics.warnings());
  match value {
    Value::Context(ctx) => Ok(Arc::unwrap_or_clone(ctx)),
    Value::Null(Some(reason)) => Err(format!("FEEL context evaluated to null: {reason}")),
    other => Err(format!("FEEL context literal expected, it evaluated to {} ({})", arg(&other), arg(&TypeOf(&other)))),
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
  use super::{Path, significant_digits};

  #[test]
  fn paths_are_displayed_as_dotted_keys_and_indexes() {
    let context = Path::Root("");
    let input = Path::Root("input");
    let items = Path::Key(&context, "items");
    let third = Path::Index(&items, 3);
    let price = Path::Key(&third, " price ");
    assert_eq!("", context.to_string());
    assert_eq!("items", items.to_string());
    assert_eq!("items[3]", third.to_string());
    assert_eq!("items[3]. price ", price.to_string());
    assert_eq!("input[0]", Path::Index(&input, 0).to_string());
    assert_eq!("input.a", Path::Key(&input, "a").to_string());
    assert_eq!("[0]", Path::Index(&context, 0).to_string());
    assert_eq!("[0][1].a", Path::Key(&Path::Index(&Path::Index(&context, 0), 1), "a").to_string());
    // a key after nothing but empty text has no dot before it
    let blank = Path::Key(&context, "");
    assert_eq!("", blank.to_string());
    assert_eq!("a", Path::Key(&blank, "a").to_string());
    assert_eq!("input.", Path::Key(&input, "").to_string());
  }

  #[test]
  fn counts_significant_digits() {
    assert_eq!(significant_digits("0.000120"), 2);
    assert_eq!(significant_digits("-1.5e10"), 2);
    assert_eq!(significant_digits("100"), 1);
    assert_eq!(significant_digits("0"), 0);
    assert_eq!(significant_digits("12345678901234567890123456789012345"), 35);
  }
}
