use super::super::*;
use rheon_feel::scope;
use rheon_feel::values::Value;

#[test]
fn _0001() {
  te_bool(false, &scope!(), r#"matches("foobar","^fo*b")"#, true);
}

#[test]
fn _0002() {
  te_bool(false, &scope!(), r#"matches(input: "foobar", pattern: "^fo*b")"#, true);
}

#[test]
fn _0003() {
  te_bool(false, &scope!(), r#"matches("abracadabra","bra")"#, true);
}

#[test]
fn _0004() {
  te_bool(false, &scope!(), r#"matches("abracadabra","^a.*a$")"#, true);
}

#[test]
fn _0005() {
  te_bool(false, &scope!(), r#"matches("abracadabra","^bra")"#, false);
}

#[test]
fn _0006() {
  te_bool(false, &scope!(), r#"matches("hello\nworld","hello.*world")"#, false);
}

#[test]
fn _0007() {
  te_bool(false, &scope!(), r#"matches("hello", "hello", null)"#, true);
}

#[test]
fn _0008() {
  te_bool(false, &scope!(), r#"matches(input: "hello", pattern: "hello", flags: null)"#, true);
}

#[test]
fn _0009() {
  te_bool(false, &scope!(), r#"matches("hello\nworld","hello.*world","s")"#, true);
}

#[test]
fn _0010() {
  te_bool(false, &scope!(), r#"matches(input: "hello\nworld", pattern: "hello.*world", flags: "s")"#, true);
}

#[test]
fn _0011() {
  let scope = &te_scope(
    r#"{poem:"<poem author=\"Wilhelm Busch\">\nKaum hat dies der Hahn gesehen,\nFängt er auch schon an zu krähen:\nKikeriki! Kikikerikih!!\nTak, tak, tak! - da kommen sie.\n</poem>"}"#,
  );
  te_bool(false, scope, r#"matches(poem, "Kaum.*krähen")"#, false);
}

#[test]
fn _0012() {
  te_null(false, &scope!(), r#"matches()"#, "expected 2,3 parameters, actual number of parameters is 0");
}

#[test]
fn _0013() {
  te_null(false, &scope!(), r#"matches("abc")"#, "expected 2,3 parameters, actual number of parameters is 1");
}

#[test]
fn _0014() {
  te_null(
    false,
    &scope!(),
    r#"matches("abc","a","b","c")"#,
    "expected 2,3 parameters, actual number of parameters is 4",
  );
}

#[test]
fn _0015() {
  te_null(false, &scope!(), r#"matches(i: "foobar", pattern: "^fo*b")"#, "parameter 'input' not found");
}

#[test]
fn _0016() {
  te_null(false, &scope!(), r#"matches(input: "foobar", p: "^fo*b")"#, "parameter 'pattern' not found");
}

#[test]
fn _0017() {
  te_null(false, &scope!(), r#"matches(i: "foobar", pattern: "^fo*b", flags: "s")"#, "parameter 'input' not found");
}

#[test]
fn _0018() {
  te_null(false, &scope!(), r#"matches(input: "foobar", p: "^fo*b", flags: "s")"#, "parameter 'pattern' not found");
}

#[test]
fn _0019() {
  te_null(false, &scope!(), r#"matches(input: "foobar", pattern: "^fo*b", f: "s")"#, "parameter 'flags' not found");
}

#[test]
fn _0020() {
  te_null(
    false,
    &scope!(),
    r#"matches(input: "abc", pattern: "a", flags: "x", other: "c")"#,
    "expected 2,3 parameters, actual number of parameters is 4",
  );
}

#[test]
fn _0021() {
  te_null(
    false,
    &scope!(),
    r#"matches(input: 10, pattern: "^fo*b")"#,
    r#"[core::matches] invalid argument type, expected string, actual type is number"#,
  );
}

#[test]
fn _0022() {
  te_null(
    false,
    &scope!(),
    r#"matches(input: "foobar", pattern: true)"#,
    r#"[core::matches] invalid argument type, expected string, actual type is boolean"#,
  );
}

#[test]
fn _0023() {
  te_null(
    false,
    &scope!(),
    r#"matches(input: 10, pattern: "^fo*b", flags: "s")"#,
    r#"[core::matches] invalid argument type, expected string, actual type is number"#,
  );
}

#[test]
fn _0024() {
  te_null(
    false,
    &scope!(),
    r#"matches(input: "foobar", pattern: true, flags: "s")"#,
    r#"[core::matches] invalid argument type, expected string, actual type is boolean"#,
  );
}

#[test]
fn _0025() {
  te_bool(false, &scope!(), r#"matches("Mary\rJones", "Mary.Jones")"#, false);
}

#[test]
fn _0026() {
  te_bool(false, &scope!(), r#"matches("Mary\rJones", "Mary.Jones", "s")"#, true);
}

#[test]
fn _0027() {
  te_bool(false, &scope!(), r#"matches("Mary\nJones", "Mary.Jones")"#, false);
}

#[test]
fn _0028() {
  te_bool(false, &scope!(), r#"matches("Mary\nJones", "Mary.Jones", "s")"#, true);
}

#[test]
fn _0029() {
  te_bool(false, &scope!(), r#"matches("Mary\r\nJones", "Mary.Jones")"#, false);
}

fn _0030() {
  te_bool(false, &scope!(), r#"matches("Mary\r\nJones", "Mary.Jones", "s")"#, true);
}

#[test]
fn _0031() {
  te_bool(false, &scope!(), r#"matches("Mary\u000AJones", "Mary.Jones")"#, false);
}

#[test]
fn _0032() {
  te_bool(false, &scope!(), r#"matches("Mary\u000AJones", "Mary.Jones", "s")"#, true);
}

#[test]
fn _0033() {
  te_bool(false, &scope!(), r#"matches("Mary\u000DJones", "Mary.Jones")"#, false);
}

#[test]
fn _0034() {
  te_bool(false, &scope!(), r#"matches("Mary\u000DJones", "Mary.Jones", "s")"#, true);
}

#[test]
fn _0035() {
  te_bool(false, &scope!(), r#"matches("x", "[A-Z-[OI]]", "i")"#, true);
}

#[test]
fn _0036() {
  te_bool(false, &scope!(), r#"matches("O", "[A-Z-[OI]]", "i")"#, false);
}

#[test]
fn _0037() {
  te_bool(false, &scope!(), r#"matches("O", "[A-Z--O]", "i")"#, false);
}

#[test]
fn _0038() {
  te_null(
    false,
    &scope!(),
    r#"matches("input", "pattern", [])"#,
    "[core::matches] invalid argument type, expected string, actual type is list<Null>",
  );
}

#[test]
fn _0039() {
  te_bool(false, &scope!(), r#"matches("input", "pattern", "")"#, false);
}

#[test]
fn _0040() {
  te_bool(false, &scope!(), r#"matches("hello!world", " hello[ ! ]world", "x")"#, true);
}

#[test]
fn _0041() {
  te_bool(false, &scope!(), r#"matches("hello world", " hello[ ]world", "x")"#, true);
}

#[test]
fn _0042() {
  te_bool(false, &scope!(), r#"matches("hello world", "\p{ IsLatin}+", "x")"#, true);
}

#[test]
fn _0043() {
  te_bool(false, &scope!(), r#"matches("hello world", "\p{IsLatin}+")"#, true);
}

#[test]
fn _0044() {
  te_bool(false, &scope!(), r#"matches("hello world", "\p{ I s L a t i n }+", "x")"#, true);
}

#[test]
fn _0045() {
  te_bool(false, &scope!(), r#"matches("abracadabra", "bra", null)"#, true);
}

#[test]
#[ignore]
fn _0046() {
  te_bool(false, &scope!(), r#"matches("hello world", "\p{IsBasicLatin}+")"#, true);
}

#[test]
#[ignore]
fn _0047() {
  te_bool(false, &scope!(), r#"matches("hello world", "\p{ I s B a s i c L a t i n }+", "x")"#, true);
}

#[test]
#[ignore]
fn _0048() {
  te_bool(false, &scope!(), r#"matches("aA", "(a)\1", "i")"#, true);
}

#[test]
fn back_references() {
  te_bool(false, &scope!(), r#"matches("aa", "(a)\\1")"#, true);
  te_bool(false, &scope!(), r#"matches("aA", "(a)\\1", "i")"#, true);
  te_bool(false, &scope!(), r#"matches("ab", "(a)\\1")"#, false);
}

#[test]
fn the_reason_of_a_long_invalid_pattern_holds_at_most_1000_characters() {
  let expression = format!(r#"matches("a", "({}")"#, "a".repeat(5000));
  let node = rheon_feel_parser::parse_expression(&scope!(), &expression).unwrap();
  let Value::Null(Some(reason)) = crate::evaluate(&scope!(), &node).into_result().unwrap() else {
    panic!("a null with a reason")
  };
  assert_eq!(1000, reason.chars().count(), "{} characters", reason.chars().count());
  assert!(reason.starts_with("<FeelRegexError> invalid regex pattern: '(aaa"), "{reason}");
  assert!(reason.ends_with('…'), "{reason}");
}

#[test]
fn an_engine_limit_stops_the_evaluation() {
  let input = format!("{}!", "a".repeat(40));
  let backtracking = r"matches: the pattern '^(a|a)*\1$' exceeds the regex engine's backtracking limit; evaluation stopped";
  crate::tests::te_aborted(&format!(r#"matches("{input}", "^(a|a)*\\1$")"#), crate::Dialect::Feel, backtracking);
  crate::tests::te_aborted(&format!(r#"matches("{input}", "^(a|a)*\\1$", "i")"#), crate::Dialect::Feel, backtracking);
  crate::tests::te_aborted(
    r#"matches("a", "\\w{10000}")"#,
    crate::Dialect::Feel,
    r"matches: the pattern '\w{10000}' exceeds the regex engine's compiled size limit; evaluation stopped",
  );
  let nested = crate::tests::nested_groups(100);
  crate::tests::te_aborted(
    &format!(r#"matches("a", "{nested}")"#),
    crate::Dialect::Feel,
    &format!(
      "matches: the pattern '{}… (201 characters)' exceeds the regex engine's nesting limit; evaluation stopped",
      &nested[..120]
    ),
  );
  // a pattern within the limits matches, and an invalid pattern is null with its reason
  te_bool(false, &scope!(), &format!(r#"matches("a", "{}")"#, crate::tests::nested_groups(60)), true);
  te_null(false, &scope!(), r#"matches("a", "[a-z")"#, "<FeelRegexError> invalid regex pattern: '[a-z'");
}

#[test]
fn every_character_of_the_flags_is_a_flag() {
  // XQuery 1.0 and XPath 2.0 F&O §7.6.1.1: a character that is not a flag is an error, and the
  // flags are not trimmed
  te_null(false, &scope!(), r#"matches("abc", "b", " i ")"#, "<FeelRegexError> invalid regex flag: ' '");
  te_null(false, &scope!(), r#"matches("abc", "b", "iI")"#, "<FeelRegexError> invalid regex flag: 'I'");
  // the empty flags switch every option off
  te_bool(false, &scope!(), r#"matches("abc", "B", "")"#, false);
  te_bool(false, &scope!(), r#"matches("abc", "B", "ii")"#, true);
}

#[test]
fn x_removes_the_whitespace_outside_character_classes() {
  // the examples of F&O §7.6.1.1
  te_bool(false, &scope!(), r#"matches("helloworld", "hello world", "x")"#, true);
  te_bool(false, &scope!(), r#"matches("helloworld", "hello[ ]world", "x")"#, false);
  te_bool(false, &scope!(), r#"matches("hello world", "hello\\\nsworld", "x")"#, true);
  te_bool(false, &scope!(), r#"matches("hello world", "hello world", "x")"#, false);
  // `#` starts no comment
  te_bool(false, &scope!(), r#"matches("ab", "a#b", "x")"#, false);
  te_bool(false, &scope!(), r#"matches("a#b", "a#b", "x")"#, true);
  // a bracket after an escape opens no class
  te_bool(false, &scope!(), r#"matches("a[b", "a\\ [ b", "x")"#, true);
}

#[test]
fn q_takes_every_character_of_the_pattern_as_itself() {
  te_bool(false, &scope!(), r#"matches("a-[b]", "a-[b]", "q")"#, true);
  te_bool(false, &scope!(), r#"matches("a\\p{IsBasicLatin}", "\\p{IsBasicLatin}", "q")"#, true);
  // with `m`, `s` or `x` beside it, `q` still applies and they have no effect
  te_bool(false, &scope!(), r#"matches("a b", "a b", "qx")"#, true);
  te_bool(false, &scope!(), r#"matches("a.b", ".", "qs")"#, true);
  te_bool(false, &scope!(), r#"matches("ab", ".", "qs")"#, false);
}

#[test]
fn an_invalid_pattern_is_quoted_as_written() {
  te_null(false, &scope!(), r#"matches("a", "[a-z", "i")"#, "<FeelRegexError> invalid regex pattern: '[a-z'");
}
