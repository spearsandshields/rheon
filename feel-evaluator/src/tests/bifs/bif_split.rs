use super::super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_be_value(false, &scope!(), r#"split("John Doe","\\s")"#, r#"["John","Doe"]"#);
}

#[test]
fn _0002() {
  te_be_value(false, &scope!(), r#"split(string: "John Doe", delimiter: "\\s")"#, r#"["John","Doe"]"#);
}

#[test]
fn _0003() {
  te_be_value(false, &scope!(), r#"split(delimiter: "\\s", string: "John Doe")"#, r#"["John","Doe"]"#);
}

#[test]
fn _0004() {
  te_be_value(false, &scope!(), r#"split("a;b;c;;",";")"#, r#"["a","b","c","",""]"#);
}

#[test]
fn _0005() {
  te_null(false, &scope!(), r#"split()"#, r#"expected 2 parameters, actual number of parameters is 0"#);
}

#[test]
fn _0006() {
  te_null(false, &scope!(), r#"split("abc")"#, r#"expected 2 parameters, actual number of parameters is 1"#);
}

#[test]
fn _0007() {
  te_null(false, &scope!(), r#"split("abc","a","b")"#, r#"expected 2 parameters, actual number of parameters is 3"#);
}

#[test]
fn _0008() {
  te_null(false, &scope!(), r#"split(s: "John Doe", delimiter: "\\s")"#, r#"parameter 'string' not found"#);
}

#[test]
fn _0009() {
  te_null(false, &scope!(), r#"split(string: "John Doe", d: "\\s")"#, r#"parameter 'delimiter' not found"#);
}

#[test]
fn _0010() {
  te_null(false, &scope!(), r#"split(10,"\\s")"#, r#"split: input must be a string"#);
}

#[test]
fn _0011() {
  te_null(false, &scope!(), r#"split("John Doe",10)"#, r#"split: delimiter must be a string"#);
}

#[test]
fn _0012() {
  te_null(false, &scope!(), r#"split("John Doe","[a-z")"#, r#"split: invalid delimiter"#);
}

#[test]
fn an_engine_limit_stops_the_evaluation() {
  crate::tests::te_aborted(
    r#"split("a", "\\w{10000}")"#,
    crate::Dialect::Feel,
    r"split: the pattern '\w{10000}' exceeds the regex engine's compiled size limit; evaluation stopped",
  );
  let nested = crate::tests::nested_groups(300);
  crate::tests::te_aborted(
    &format!(r#"split("a", "{nested}")"#),
    crate::Dialect::Feel,
    &format!(
      "split: the pattern '{}… (601 characters)' exceeds the regex engine's nesting limit; evaluation stopped",
      &nested[..120]
    ),
  );
}

#[test]
fn a_delimiter_that_matches_the_zero_length_string_is_null() {
  // XQuery 1.0 and XPath 2.0 F&O §7.6.4: FORX0003 "If the supplied $pattern matches a zero-length
  // string, that is, if fn:matches("", $pattern, $flags) returns true"
  let reason = "<FeelRegexError> the pattern matches the zero-length string";
  te_null(false, &scope!(), r#"split("abc","")"#, reason);
  te_null(false, &scope!(), r#"split("a,b",",*")"#, reason);
  te_null(false, &scope!(), r#"split("abba",".?")"#, reason);
  // the delimiter is checked also for an empty input
  te_null(false, &scope!(), r#"split("","x*")"#, reason);
  te_null(false, &scope!(), r#"split("","[a-z")"#, "split: invalid delimiter");
}

#[test]
fn split_follows_tokenize() {
  // F&O §7.6.4: "If $input is the empty sequence, or if $input is the zero-length string, the
  // result is the empty sequence."
  te_be_value(false, &scope!(), r#"split("",";")"#, "[]");
  // separators at the start, at the end and next to each other give zero-length strings
  te_be_value(false, &scope!(), r#"split("1,15,,24,50,",",")"#, r#"["1","15","","24","50",""]"#);
  te_be_value(false, &scope!(), r#"split(";",";")"#, r#"["",""]"#);
  te_be_value(false, &scope!(), r#"split("abracadabra","(ab)|(a)")"#, r#"["","r","c","d","r",""]"#);
  te_be_value(false, &scope!(), r#"split("The cat sat on the mat","\\s+")"#, r#"["The","cat","sat","on","the","mat"]"#);
  te_be_value(false, &scope!(), r#"split("abc",";")"#, r#"["abc"]"#);
}
