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

use rheon_feel::values::Value;

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

/// Renders a value on a single line.
fn inline(value: &Value) -> String {
  match value {
    Value::List(items) => {
      let parts: Vec<String> = items.iter().map(inline).collect();
      format!("[{}]", parts.join(", "))
    }
    Value::Context(ctx) => {
      if ctx.is_empty() {
        return "{}".to_string();
      }
      let parts: Vec<String> = ctx.iter().map(|(name, v)| format!("{}: {}", quote(name.as_str()), inline(v))).collect();
      format!("{{{}}}", parts.join(", "))
    }
    other => scalar(other),
  }
}

fn scalar(value: &Value) -> String {
  match value {
    Value::Null(_) => "null".to_string(),
    Value::Boolean(b) => b.to_string(),
    Value::Number(n) => n.to_string(),
    Value::String(s) => quote(s),
    Value::FunctionDefinition(..) | Value::BuiltInFunction(_) | Value::ExternalJavaFunction(..) | Value::ExternalPmmlFunction(..) => {
      quote(&value.type_of().to_string())
    }
    Value::DateTime(date_time) => quote(&zoned_date_time(date_time).unwrap_or_else(|| date_time.to_string())),
    other => quote(&other.to_string()),
  }
}

/// `<date>T<time><±hh:mm>[<zone>]` for a date and time with an IANA zone (DMN 1.6 Table 48), or
/// `None` for any other date and time (written in FEEL form, which already is ISO 8601).
fn zoned_date_time(date_time: &rheon_feel_temporal::FeelDateTime) -> Option<String> {
  let zone = date_time.feel_time_zone()?;
  let offset = date_time.feel_time_offset()?;
  let text = date_time.to_string();
  let local = text.split('@').next()?;
  let sign = if offset < 0 { '-' } else { '+' };
  let minutes = offset.unsigned_abs() / 60;
  Some(format!("{local}{sign}{:02}:{:02}[{zone}]", minutes / 60, minutes % 60))
}

fn render(value: &Value, indent: usize, out: &mut String) {
  let one_line = inline(value);
  let is_container = matches!(value, Value::List(_) | Value::Context(_));
  if !is_container || indent + one_line.len() <= MAX_INLINE_WIDTH {
    out.push_str(&one_line);
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
    _ => out.push_str(&one_line),
  }
}
