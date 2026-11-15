//! The `display` rendering: pretty, JSON-like text with exact FEEL digits.
//!
//! * numbers: dsntk's decimal text (exact, up to 34 digits; never via `f64`);
//! * strings: JSON-quoted and escaped;
//! * booleans / null: `true`, `false`, `null`;
//! * temporal values, ranges and other non-JSON values: their FEEL text, JSON-quoted
//!   (e.g. `"2026-10-16"`, `"P1DT2H"`, `"[1..10]"`), except that a date and time with an IANA
//!   time zone is written as ISO 8601 with its offset and the zone in brackets
//!   (`"2007-12-03T10:15:30+01:00[Europe/Paris]"`, DMN 1.6 Table 48, DMN16-68);
//! * functions: `"function<...>->..."` (their type);
//! * lists / contexts: `[...]` / `{"key": value}`; kept on one line when short, otherwise
//!   expanded with two-space indentation.
//!
//! A container is measured before it is written: its one-line text is counted, without being
//! built, until it passes the width left on its line. Each node is then written once, so the
//! memory of a display is its output, whatever the depth of the value.

use rheon_feel::values::Value;
use rheon_feel_temporal::{FeelDateTime, FeelOffset};
use std::io;

/// Lines longer than this are expanded.
const MAX_INLINE_WIDTH: usize = 80;

/// Renders a value for display.
pub fn display(value: &Value) -> String {
  let mut out = String::new();
  render(value, 0, &mut out);
  out
}

fn quote(s: &str) -> String {
  serde_json::to_string(s).unwrap_or_else(|_| format!("\"{s}\""))
}

/// A measure that passed its limit; it stops the one-line rendering.
struct Full;

/// Where the one-line text of a value goes: the output, or a [Measure].
trait Sink {
  /// Appends text as it is.
  fn text(&mut self, text: &str) -> Result<(), Full>;
  /// Appends a string JSON-quoted and escaped.
  fn quoted(&mut self, text: &str) -> Result<(), Full>;
}

impl Sink for String {
  fn text(&mut self, text: &str) -> Result<(), Full> {
    self.push_str(text);
    Ok(())
  }

  fn quoted(&mut self, text: &str) -> Result<(), Full> {
    self.push_str(&quote(text));
    Ok(())
  }
}

/// Counts the bytes of a one-line text and stops once they pass `limit`.
struct Measure {
  bytes: usize,
  limit: usize,
}

impl Measure {
  fn add(&mut self, bytes: usize) -> Result<(), Full> {
    self.bytes = self.bytes.saturating_add(bytes);
    if self.bytes > self.limit { Err(Full) } else { Ok(()) }
  }
}

impl Sink for Measure {
  fn text(&mut self, text: &str) -> Result<(), Full> {
    self.add(text.len())
  }

  fn quoted(&mut self, text: &str) -> Result<(), Full> {
    // the quotes and escapes only add bytes, so a string longer than what is left never fits
    if text.len().saturating_add(2) > self.limit - self.bytes {
      return Err(Full);
    }
    serde_json::to_writer(&mut *self, text).map_err(|_| Full)
  }
}

impl io::Write for Measure {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    self.add(buf.len()).map_err(|_| io::Error::from(io::ErrorKind::WriteZero))?;
    Ok(buf.len())
  }

  fn flush(&mut self) -> io::Result<()> {
    Ok(())
  }
}

/// Whether the one-line text of `value` has at most `limit` bytes.
fn fits(value: &Value, limit: usize) -> bool {
  write_inline(value, &mut Measure { bytes: 0, limit }).is_ok()
}

/// Writes a value on a single line.
fn write_inline(value: &Value, sink: &mut impl Sink) -> Result<(), Full> {
  match value {
    Value::List(items) => {
      sink.text("[")?;
      for (i, item) in items.iter().enumerate() {
        if i > 0 {
          sink.text(", ")?;
        }
        write_inline(item, sink)?;
      }
      sink.text("]")
    }
    Value::Context(ctx) => {
      sink.text("{")?;
      for (i, (name, v)) in ctx.iter().enumerate() {
        if i > 0 {
          sink.text(", ")?;
        }
        sink.quoted(name.as_str())?;
        sink.text(": ")?;
        write_inline(v, sink)?;
      }
      sink.text("}")
    }
    other => write_scalar(other, sink),
  }
}

fn write_scalar(value: &Value, sink: &mut impl Sink) -> Result<(), Full> {
  match value {
    Value::Null(_) => sink.text("null"),
    Value::Boolean(b) => sink.text(if *b { "true" } else { "false" }),
    Value::Number(n) => sink.text(&n.to_string()),
    Value::String(s) => sink.quoted(s),
    Value::FunctionDefinition(..) | Value::BuiltInFunction(_) | Value::ExternalJavaFunction(..) | Value::ExternalPmmlFunction(..) => sink.quoted(&value.type_of().to_string()),
    Value::DateTime(date_time) => sink.quoted(&zoned_date_time(date_time).unwrap_or_else(|| date_time.to_string())),
    other => sink.quoted(&other.to_string()),
  }
}

/// `<date>T<time><±hh:mm>[<zone>]` for a date and time with an IANA zone (DMN 1.6 Table 48), or
/// `None` for any other date and time (written in FEEL form, which already is ISO 8601). The
/// offset is the zone's at that date and time, written by the one offset printer, with `:ss` when
/// it has seconds (local mean time before the standard zones, `+00:09:21` in Paris before 1911).
fn zoned_date_time(date_time: &FeelDateTime) -> Option<String> {
  let zone = date_time.feel_time_zone()?;
  let offset = date_time.feel_time_offset()?;
  let text = date_time.to_string();
  let mut out = String::from(text.split('@').next()?);
  FeelOffset::write_seconds(&mut out, offset).ok()?;
  out.push('[');
  out.push_str(&zone);
  out.push(']');
  Some(out)
}

fn render(value: &Value, indent: usize, out: &mut String) {
  // `indent + width <= MAX_INLINE_WIDTH` exactly when `width <= MAX_INLINE_WIDTH - indent`; past
  // that indent no container fits, as its text has at least 2 bytes
  let is_container = matches!(value, Value::List(_) | Value::Context(_));
  if !is_container || fits(value, MAX_INLINE_WIDTH.saturating_sub(indent)) {
    let _ = write_inline(value, out);
    return;
  }
  let pad = " ".repeat(indent + 2);
  let close_pad = " ".repeat(indent);
  match value {
    Value::List(items) => {
      out.push_str("[\n");
      for (i, item) in items.iter().enumerate() {
        out.push_str(&pad);
        render(item, indent + 2, out);
        if i + 1 < items.len() {
          out.push(',');
        }
        out.push('\n');
      }
      out.push_str(&close_pad);
      out.push(']');
    }
    Value::Context(ctx) => {
      out.push_str("{\n");
      let len = ctx.len();
      for (i, (name, v)) in ctx.iter().enumerate() {
        out.push_str(&pad);
        out.push_str(&quote(name.as_str()));
        out.push_str(": ");
        render(v, indent + 2, out);
        if i + 1 < len {
          out.push(',');
        }
        out.push('\n');
      }
      out.push_str(&close_pad);
      out.push('}');
    }
    _ => {
      let _ = write_inline(value, out);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::display;
  use rheon_feel::context::FeelContext;
  use rheon_feel::values::Value;

  fn string(s: &str) -> Value {
    Value::String(s.to_string().into())
  }

  fn list(items: Vec<Value>) -> Value {
    Value::List(items.into())
  }

  fn context(entries: &[(&str, Value)]) -> Value {
    let mut ctx = FeelContext::default();
    for (name, value) in entries {
      ctx.set_entry(&(*name).into(), value.clone());
    }
    Value::Context(ctx)
  }

  /// `inner` at indent `2 * depth`: inside `depth` lists, each of which also holds an 80-byte
  /// string, so that every one of them is expanded.
  fn nested(depth: usize, inner: Value) -> Value {
    (0..depth).fold(inner, |value, _| list(vec![value, string(&"f".repeat(78))]))
  }

  /// The line of `inner` as `nested(depth, inner)` displays it, its indent included.
  fn line_of(depth: usize, inner: Value) -> String {
    let text = display(&nested(depth, inner));
    let start = text
      .find(&format!("\n{}[", " ".repeat(2 * depth)))
      .map(|i| i + 1)
      .unwrap_or_else(|| text.find(&format!("\n{}{{", " ".repeat(2 * depth))).unwrap() + 1);
    text[start..].lines().next().unwrap().to_string()
  }

  #[test]
  fn a_list_of_80_bytes_stays_inline_and_one_of_81_is_expanded() {
    let at = list(vec![string(&"a".repeat(76))]);
    assert_eq!(format!("[\"{}\"]", "a".repeat(76)), display(&at));
    let past = list(vec![string(&"a".repeat(77))]);
    assert_eq!(format!("[\n  \"{}\"\n]", "a".repeat(77)), display(&past));
  }

  #[test]
  fn the_width_left_shrinks_with_the_indent() {
    let width = |bytes: usize| list(vec![string(&"a".repeat(bytes - 4))]);
    // at indent 2, 78 bytes fit and 79 do not
    assert_eq!(format!("  [\"{}\"],", "a".repeat(74)), line_of(1, width(78)));
    assert_eq!("  [", line_of(1, width(79)));
    // at indent 78, 2 bytes fit and 3 do not
    assert_eq!(format!("{}[],", " ".repeat(78)), line_of(39, list(vec![])));
    assert_eq!(format!("{}[", " ".repeat(78)), line_of(39, list(vec![Value::Boolean(true)])));
    assert_eq!(format!("{}{{}},", " ".repeat(78)), line_of(39, context(&[])));
  }

  #[test]
  fn the_width_is_counted_in_bytes() {
    // 38 two-byte characters: 80 bytes inline; one more byte makes 81 bytes but 43 characters
    let at = list(vec![string(&"é".repeat(38))]);
    assert_eq!(format!("[\"{}\"]", "é".repeat(38)), display(&at));
    let past = list(vec![string(&("é".repeat(38) + "a"))]);
    assert_eq!(format!("[\n  \"{}a\"\n]", "é".repeat(38)), display(&past));
  }

  #[test]
  fn escapes_count_as_written() {
    // a control character is written as \u0001 (6 bytes), a quote, a backslash and a newline as 2
    for (escaped, written) in [("\u{1}", "\\u0001"), ("\"", "\\\""), ("\\", "\\\\"), ("\n", "\\n")] {
      let fill = 76 - written.len();
      let at = list(vec![string(&("a".repeat(fill) + escaped))]);
      assert_eq!(format!("[\"{}{written}\"]", "a".repeat(fill)), display(&at), "{written}");
      let past = list(vec![string(&("a".repeat(fill + 1) + escaped))]);
      assert_eq!(format!("[\n  \"{}{written}\"\n]", "a".repeat(fill + 1)), display(&past), "{written}");
      // the same in a key, {"<key>": true} being 10 bytes more than the key as written (a name
      // folds white space, so a newline does not stay in it)
      if escaped == "\n" {
        continue;
      }
      let fill = 70 - written.len();
      let at = context(&[(&("k".repeat(fill) + escaped), Value::Boolean(true))]);
      assert_eq!(format!("{{\"{}{written}\": true}}", "k".repeat(fill)), display(&at), "{written}");
      let past = context(&[(&("k".repeat(fill + 1) + escaped), Value::Boolean(true))]);
      assert_eq!(format!("{{\n  \"{}{written}\": true\n}}", "k".repeat(fill + 1)), display(&past), "{written}");
    }
  }

  #[test]
  fn empty_containers() {
    assert_eq!("[]", display(&list(vec![])));
    assert_eq!("{}", display(&context(&[])));
    assert_eq!("[[], {}]", display(&list(vec![list(vec![]), context(&[])])));
  }

  #[test]
  fn past_indent_80_every_container_is_expanded() {
    let mut value = list(vec![]);
    for _ in 0..45 {
      value = list(vec![value]);
    }
    let mut expected = format!("[\n{}]", " ".repeat(90));
    for depth in (0..45).rev() {
      expected = format!("[\n{}{expected}\n{}]", " ".repeat(2 * depth + 2), " ".repeat(2 * depth));
    }
    assert_eq!(expected, display(&value));
  }
}
