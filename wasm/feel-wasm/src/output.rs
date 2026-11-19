//! The response of a successful evaluation, written as JSON text straight from the result value:
//! no tree of JSON values and no copy of the value in another form between the value and the
//! text. The text is, byte for byte, what `serde_json` writes for the tree that the response
//! was before ([result_tree] in the tests): the keys of every object in alphabetical order, no
//! white space, and its escapes in strings.

use crate::display::write_display;
use crate::json::string_stop;
use rheon_feel::dto::{XSD_BOOLEAN, XSD_DATE, XSD_DATE_TIME, XSD_DECIMAL, XSD_DURATION, XSD_STRING, XSD_TIME};
use rheon_feel::values::Value;
use std::fmt::{self, Write};

/// Writes `text` as a JSON string: with the escapes of `serde_json`, the two-character forms
/// where JSON has one, `\u00XX` for the other control characters, and nothing else escaped.
pub(crate) fn write_string(text: &str, out: &mut String) {
  out.push('"');
  write_escaped(text, out);
  out.push('"');
}

/// For each byte, the letter of its escape in a JSON string (`u` for `\u00XX`), or zero when
/// the byte is written as it is.
const ESCAPES: [u8; 256] = {
  let mut escapes = [0; 256];
  let mut control = 0;
  while control < 0x20 {
    escapes[control] = b'u';
    control += 1;
  }
  escapes[0x08] = b'b';
  escapes[0x09] = b't';
  escapes[0x0A] = b'n';
  escapes[0x0C] = b'f';
  escapes[0x0D] = b'r';
  escapes[b'"' as usize] = b'"';
  escapes[b'\\' as usize] = b'\\';
  escapes
};

/// Writes `text` as the inside of a JSON string: the pieces between the bytes that are escaped
/// as they are. A long piece is passed eight bytes at a time ([string_stop]).
pub(crate) fn write_escaped(text: &str, out: &mut String) {
  let bytes = text.as_bytes();
  // the start of the piece that is not written yet, and the position
  let (mut piece, mut at) = (0, 0);
  while let Some(byte) = bytes.get(at) {
    let escape = ESCAPES[*byte as usize];
    if escape == 0 {
      at += 1;
      if at - piece == 32 {
        at = string_stop(bytes, at);
      }
      continue;
    }
    // an escaped byte is an ASCII byte, so both sides of it are whole characters
    out.push_str(&text[piece..at]);
    out.push('\\');
    out.push(escape as char);
    if escape == b'u' {
      const HEX: &[u8; 16] = b"0123456789abcdef";
      out.push_str("00");
      out.push(HEX[(byte >> 4) as usize] as char);
      out.push(HEX[(byte & 0xF) as usize] as char);
    }
    at += 1;
    piece = at;
  }
  out.push_str(&text[piece..]);
}

/// A writer into the inside of a JSON string: what is written to it is escaped.
struct Escaped<'a>(&'a mut String);

impl Write for Escaped<'_> {
  fn write_str(&mut self, text: &str) -> fmt::Result {
    write_escaped(text, self.0);
    Ok(())
  }

  /// A character that needs no escape is written as it is: some values display their text one
  /// character at a time.
  fn write_char(&mut self, c: char) -> fmt::Result {
    if matches!(c, '"' | '\\' | '\0'..='\x1F') {
      write_escaped(c.encode_utf8(&mut [0; 4]), self.0);
    } else {
      self.0.push(c);
    }
    Ok(())
  }
}

/// Writes what `shown` displays as a JSON string, without a text of it in between.
fn write_shown(shown: impl fmt::Display, out: &mut String) {
  out.push('"');
  // writing into a string never fails
  let _ = write!(Escaped(out), "{shown}");
  out.push('"');
}

/// Writes the `result` object of a successful evaluation (CONTRACT §3).
pub(crate) fn write_result(value: &Value, out: &mut String) {
  out.push_str("{\"display\":\"");
  write_display(value, out);
  out.push('"');
  out.push_str(",\"feel\":");
  write_shown(value, out);
  out.push_str(if value.is_null() { ",\"isNull\":true" } else { ",\"isNull\":false" });
  if let Value::Null(Some(reason)) = value {
    out.push_str(",\"nullReason\":");
    write_string(reason, out);
  }
  out.push_str(",\"type\":");
  write_shown(value.type_of(), out);
  out.push_str(",\"typed\":");
  write_typed(value, out);
  out.push('}');
}

/// Writes the typed form of a value (CONTRACT §3, `typed`): a simple value as its text and the
/// name of its XSD type, a context as its components, a list as its items, and a value of
/// another kind (a range, a function) as the form without any of the three.
fn write_typed(value: &Value, out: &mut String) {
  let simple = |xsd_type: &str, out: &mut String| {
    out.push_str("{\"components\":null,\"list\":null,\"simple\":{\"isNil\":false,\"text\":");
    match value {
      Value::String(text) => write_string(text.as_str(), out),
      // the text of a number or a boolean has no character that JSON escapes
      Value::Number(number) => {
        out.push('"');
        // writing into a string never fails
        let _ = write!(out, "{number}");
        out.push('"');
      }
      Value::Boolean(boolean) => out.push_str(if *boolean { "\"true\"" } else { "\"false\"" }),
      other => write_shown(other, out),
    }
    // nor has the name of an XSD type
    out.push_str(",\"type\":\"");
    out.push_str(xsd_type);
    out.push_str("\"}}");
  };
  match value {
    Value::String(_) => simple(XSD_STRING, out),
    Value::Number(_) => simple(XSD_DECIMAL, out),
    Value::Boolean(_) => simple(XSD_BOOLEAN, out),
    Value::Date(_) => simple(XSD_DATE, out),
    Value::DateTime(_) => simple(XSD_DATE_TIME, out),
    Value::Time(_) => simple(XSD_TIME, out),
    Value::YearsAndMonthsDuration(_) | Value::DaysAndTimeDuration(_) => simple(XSD_DURATION, out),
    Value::Null(_) => out.push_str("{\"components\":null,\"list\":null,\"simple\":{\"isNil\":true,\"text\":null,\"type\":null}}"),
    Value::Context(context) => {
      out.push_str("{\"components\":[");
      for (index, (name, entry)) in context.iter().enumerate() {
        if index > 0 {
          out.push(',');
        }
        out.push_str("{\"isNil\":false,\"name\":");
        write_string(name.as_str(), out);
        out.push_str(",\"value\":");
        write_typed(entry, out);
        out.push('}');
      }
      out.push_str("],\"list\":null,\"simple\":null}");
    }
    Value::List(items) => {
      out.push_str("{\"components\":null,\"list\":{\"isNil\":false,\"items\":[");
      for (index, item) in items.iter().enumerate() {
        if index > 0 {
          out.push(',');
        }
        write_typed(item, out);
      }
      out.push_str("]},\"simple\":null}");
    }
    _ => out.push_str("{\"components\":null,\"list\":null,\"simple\":null}"),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::display::display;
  use rheon_feel::dto::ValueDto;
  use rheon_feel::{FeelScope, diagnostics};
  use serde_json::{Value as Json, json};

  /// The `result` object as the tree of JSON values that it was written from before: the
  /// reference of [write_result].
  fn result_tree(value: &Value) -> Json {
    let typed = ValueDto::try_from(value).ok().and_then(|dto| serde_json::to_value(dto).ok()).unwrap_or(Json::Null);
    let mut result = json!({
      "display": display(value),
      "feel": value.to_string(),
      "type": value.type_of().to_string(),
      "typed": typed,
      "isNull": value.is_null(),
    });
    if let Value::Null(Some(reason)) = value {
      result["nullReason"] = Json::String(reason.clone());
    }
    result
  }

  fn evaluated(expression: &str) -> Value {
    let scope = FeelScope::default();
    let node = rheon_feel_parser::parse_expression(&scope, expression).unwrap_or_else(|e| panic!("{expression}: {e}"));
    let (outcome, _) = diagnostics::collect(|| rheon_feel_evaluator::evaluate(&scope, &node));
    outcome.into_result().unwrap_or_else(|abort| panic!("{expression}: {abort}"))
  }

  /// The result is written byte for byte as `serde_json` writes the tree of it, for values of
  /// every kind, nested, with every character that a string escapes.
  #[test]
  fn result_text_equals_the_text_of_its_tree() {
    let every_control: String = (0u8..0x20).map(|byte| format!("\\u00{byte:02X}")).collect();
    let expressions = [
      "1".to_string(),
      "-0.50".to_string(),
      "1e40 * 1e-80".to_string(),
      "true".to_string(),
      "null".to_string(),
      "1 + \"a\"".to_string(),
      "\"\"".to_string(),
      "\"plain text\"".to_string(),
      "\"quote \\\" backslash \\\\ slash / tab \\t newline \\n return \\r\"".to_string(),
      format!("\"{every_control}\""),
      "\"\\u007F \\u0080 \\u00E9 \\u4E2D \\U01F600 \\u2028 \\u2029\"".to_string(),
      "\"12345678\\\"0123456\\\\789\"".to_string(),
      "date(\"2026-01-31\")".to_string(),
      "time(\"10:11:12.5+02:00\")".to_string(),
      "date and time(\"2026-01-31T10:11:12@Europe/Paris\")".to_string(),
      "duration(\"P1Y2M\")".to_string(),
      "duration(\"-P1DT2H3.5S\")".to_string(),
      "[]".to_string(),
      "{}".to_string(),
      "[1, \"a\\nb\", null, true, [2, [3, []]], {}]".to_string(),
      "{a: 1, \"b \\\"q\\\"\": [1, {c: null, \"d\\te\": \"x\\\\y\"}], \"é\": date(\"2026-01-01\")}".to_string(),
      "[1..5]".to_string(),
      "(1..\"a\\\"b\"]".to_string(),
      "function(a, b) a + b".to_string(),
      "abs".to_string(),
      "[function(a) a, [1..2], {f: abs}]".to_string(),
      "{\"with \\\"quotes\\\" and \\\\\": {\"\\n\": 1}}".to_string(),
      "for i in 1..40 return {n: i, s: string(i) + \"\\t\\\"\", l: [i, null, \"é\"]}".to_string(),
      "for i in 1..300 return i / 7".to_string(),
      "[{long: \"a long text that does not fit on one line of the display, with a \\\" quote and more words to fill it\", other: [1, 2, 3]}]".to_string(),
    ];
    for expression in expressions {
      let value = evaluated(&expression);
      let mut text = String::from("before");
      write_result(&value, &mut text);
      assert_eq!(format!("before{}", serde_json::to_string(&result_tree(&value)).unwrap()), text, "{expression}");
    }
  }

  /// A string is written as `serde_json` writes it, whatever characters it has and wherever
  /// they stand in the eight bytes that are looked at together.
  #[test]
  fn strings_are_escaped_as_serde_json_escapes_them() {
    let pieces = [
      "",
      "a",
      "abcdefg",
      "abcdefgh",
      "abcdefghi",
      "\"",
      "\\",
      "/",
      "\n",
      "\r",
      "\t",
      "\u{8}",
      "\u{c}",
      "\u{0}",
      "\u{1}",
      "\u{1f}",
      "\u{7f}",
      "é",
      "中",
      "😀",
      "\u{2028}",
      " ",
    ];
    for a in pieces {
      for b in pieces {
        for c in pieces {
          let text = format!("{a}{b}{c}{a}");
          let mut written = String::new();
          write_string(&text, &mut written);
          assert_eq!(serde_json::to_string(&text).unwrap(), written, "{text:?}");
          let mut shown = String::new();
          write_shown(&text, &mut shown);
          assert_eq!(written, shown, "{text:?}");
        }
      }
    }
  }
}
