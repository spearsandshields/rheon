//! Testing parsing of invalid statements.

use super::super::*;
use dsntk_common::DsntkError;
use std::str::from_utf8;

/// Utility function to shorten repeatable code in tests.
fn te(input: &str, source: &str, message: &str) {
  assert_eq!(Err(DsntkError::new(source, message)), Parser::new(&scope!(), StartExpression, input).parse())
}

#[test]
fn _0001() {
  // Caret character is not valid at the beginning of any FEEL statement.
  te(r#"^123"#, "ParserError", r#"syntax error: ^123"#);
}

#[test]
fn _0002() {
  // Vertical space is not allowed inside FEEL string.
  let input = from_utf8(&[34, 49, 50, 10, 51, 34]).unwrap(); // "12\n3"
  te(input, "ParserError", "syntax error: \"12\n3\"");
}

#[test]
fn _0003() {
  // Unexpected end of input before the FEEL string is closed with quotation mark `"`.
  te(r#""123"#, "ParserError", "syntax error: \"123");
}

#[test]
fn _0004() {
  // Unexpected end of input when parsing hex digits.
  te(r#""a\u4B"#, "LexerError", "unexpected end of file");
}

#[test]
fn _0005() {
  // After decimal point must always be a digit.
  te(r#"1. + 2"#, "ParserError", "syntax error: 1. + 2");
}

#[test]
fn _0006() {
  // After decimal point must be a digit.
  te(r#"1.^ + 2"#, "ParserError", "syntax error: 1.^ + 2");
}

#[test]
fn _0007() {
  // End of input after decimal point.
  te(r#"1."#, "ParserError", "syntax error: 1.");
}

#[test]
fn _0008() {
  // Keyword `function` interpreted as a name. This is ok, when such name is in scope.
  let scope = scope!();
  accept(
    &scope,
    StartExpression,
    "function ",
    r#"
       Name
       └─ `function`
    "#,
    false,
  );
}

#[test]
fn _0009() {
  // Invalid UTF-16 surrogate.
  te(r#""\uD800\uE000""#, "LexerError", "surrogate value is out of allowed range 0xD800..0xDFFF : E000");
}

#[test]
fn _0010() {
  // Invalid UTF value.
  te(r#""\U110000""#, "LexerError", "value is out of allowed Unicode range 0x0000..0x10FFFF : 110000");
}

#[test]
fn _0012() {
  // The name of an iteration variable can not start with the keyword `in` (DMN 1.6 §10.3.1.4,
  // "A name start SHALL NOT be a literal terminal symbol"), whatever follows it.
  for input in [
    "for in+1 in [1] return 1",
    "for in+1 in [1] return in+1",
    "for in[1] return 1",
    "for in(1) return 1",
    "for in.x in [1] return 1",
    "for in/2 in [1] return 1",
    "for in'x in [1] return 1",
    "for in: 1 return 1",
    "for x in [1], in*2 in [2] return x",
    "some in.x in [1] satisfies true",
    r#"some in"a" satisfies true"#,
    "every in-x in [] satisfies false",
    "for in in [1] return 1",
  ] {
    te(input, "ParserError", &format!("syntax error: {input}"));
  }
}

#[test]
fn _0013() {
  // The word `in` is the keyword whatever follows it (DMN 1.6 clause 10.3.1.4), also after the
  // iteration variable `item`, which is a name like any other.
  for input in ["for x in[1] return x", "for item in[1] return item", "for item in(1..2) return item"] {
    assert!(Parser::new(&scope!(), StartExpression, input).parse().is_ok(), "{input}");
  }
  for input in ["for item in [1] return in+1", "for i in [1] return in+1", "for item in, x in [1] return 1"] {
    te(input, "ParserError", &format!("syntax error: {input}"));
  }
}

#[test]
fn _0011() {
  // There is no `+=` operator in FEEL, but this expression parses properly (will return null on execution).
  let input = "1 += 2";
  let expected = r#"
       Add
       ├─ Numeric
       │  └─ `1`
       └─ UnaryEq
          └─ Numeric
             └─ `2`
    "#;
  accept(&scope!(), StartExpression, input, expected, false);
}

#[test]
fn a_long_input_is_quoted_as_a_preview() {
  // a syntax error quotes an input of more than 120 characters as its first 120 and its length
  let input = format!("1{}+", "+1".repeat(20000));
  te(&input, "ParserError", &format!("syntax error: {}… (40002 characters)", &input[..120]));
  // 120 characters are quoted whole
  let input = format!("1{}+", "+1".repeat(59));
  assert_eq!(120, input.len());
  te(&input, "ParserError", &format!("syntax error: {input}"));
}

/// A string literal or a block comment that the input does not close is a syntax error, with
/// every start rule: the input does not end before it.
#[test]
fn unclosed_string_or_block_comment_is_a_syntax_error() {
  let scope = scope!();
  scope.set_entry_name("a".into());
  for input in [
    "1 + 2 \"never closed",
    "1 \"never closed",
    "\"never closed",
    "\"",
    "1 \"",
    "\"closed\" + \"never closed",
    "\"never \\\" closed",
    "\"never closed\\",
    "[1, 2] \"x",
    "a \"x",
    "1 /* never closed",
    "1 + 2 /* never closed",
    "/* never closed",
    "/*",
    "1 /*",
    "1 /* closed */ + 2 /* never closed",
    "1 /* never closed *",
    "1 /* never closed * /",
    "1 // line\n /* never closed",
    "a /* never closed",
    "1 /*/",
  ] {
    let starts = [StartExpression, StartTextualExpression, StartBoxedExpression, StartUnaryTests, StartRangeLiteral];
    for (rule, start) in starts.into_iter().enumerate() {
      let message = match Parser::new(&scope, start, input).parse() {
        Ok(node) => node.trace(),
        Err(reason) => reason.to_string(),
      };
      assert_eq!(format!("<ParserError> syntax error: {input}"), message, "start rule {rule}, {input:?}");
    }
  }
}

/// Closed strings and comments, and a line comment up to the end of the input, are read as before.
#[test]
fn closed_strings_and_comments_are_read() {
  let scope = scope!();
  for input in ["1 + 2 \"closed\"", "1 \"never closed"] {
    assert!(Parser::new(&scope, StartExpression, input).parse().is_err(), "{input}");
  }
  for input in [
    "1 /* closed */",
    "1 /**/",
    "1 /***/",
    "1 /* a /* b */",
    "1 // never ended",
    "1 //",
    "/* c */ 1 // d",
    "\"/* never closed\"",
    "\"a\\\"b\"",
    "1 /* \" */",
    "1 // \"",
  ] {
    assert!(Parser::new(&scope, StartExpression, input).parse().is_ok(), "{input}");
  }
}
