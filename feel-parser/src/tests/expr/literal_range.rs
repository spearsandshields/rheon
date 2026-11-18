use super::super::*;
use crate::token::TokenType::StartRangeLiteral;

#[test]
fn _0001() {
  let input = "[1..2]";
  let expected = r#"
       Range
       ├─ IntervalStart (closed)
       │  └─ Numeric
       │     └─ `1`
       └─ IntervalEnd (closed)
          └─ Numeric
             └─ `2`
    "#;
  accept(&scope!(), StartRangeLiteral, input, expected, false);
}

#[test]
fn _0002() {
  let input = "(1..2]";
  let expected = r#"
       Range
       ├─ IntervalStart (opened)
       │  └─ Numeric
       │     └─ `1`
       └─ IntervalEnd (closed)
          └─ Numeric
             └─ `2`
    "#;
  accept(&scope!(), StartRangeLiteral, input, expected, false);
}

#[test]
fn _0003() {
  let input = "]1..2]";
  let expected = r#"
       Range
       ├─ IntervalStart (opened)
       │  └─ Numeric
       │     └─ `1`
       └─ IntervalEnd (closed)
          └─ Numeric
             └─ `2`
    "#;
  accept(&scope!(), StartRangeLiteral, input, expected, false);
}

#[test]
fn _0004() {
  let input = "[1..2)";
  let expected = r#"
       Range
       ├─ IntervalStart (closed)
       │  └─ Numeric
       │     └─ `1`
       └─ IntervalEnd (opened)
          └─ Numeric
             └─ `2`
    "#;
  accept(&scope!(), StartRangeLiteral, input, expected, false);
}

#[test]
fn _0005() {
  let input = "[1..2[";
  let expected = r#"
       Range
       ├─ IntervalStart (closed)
       │  └─ Numeric
       │     └─ `1`
       └─ IntervalEnd (opened)
          └─ Numeric
             └─ `2`
    "#;
  accept(&scope!(), StartRangeLiteral, input, expected, false);
}

#[test]
fn _0006() {
  let input = "(1..2)";
  let expected = r#"
       Range
       ├─ IntervalStart (opened)
       │  └─ Numeric
       │     └─ `1`
       └─ IntervalEnd (opened)
          └─ Numeric
             └─ `2`
    "#;
  accept(&scope!(), StartRangeLiteral, input, expected, false);
}

#[test]
fn _0007() {
  let input = "]1..2[";
  let expected = r#"
       Range
       ├─ IntervalStart (opened)
       │  └─ Numeric
       │     └─ `1`
       └─ IntervalEnd (opened)
          └─ Numeric
             └─ `2`
    "#;
  accept(&scope!(), StartRangeLiteral, input, expected, false);
}

#[test]
fn _0008() {
  let input = r#"["a".."z"]"#;
  let expected = r#"
       Range
       ├─ IntervalStart (closed)
       │  └─ String
       │     └─ `a`
       └─ IntervalEnd (closed)
          └─ String
             └─ `z`
    "#;
  accept(&scope!(), StartRangeLiteral, input, expected, false);
}

#[test]
fn _0009() {
  let input = r#"[date("2012-01-01")..date("2021-12-31")]"#;
  let expected = r#"
       Range
       ├─ IntervalStart (closed)
       │  └─ FunctionInvocation
       │     ├─ Name
       │     │  └─ `date`
       │     └─ PositionalParameters
       │        └─ String
       │           └─ `2012-01-01`
       └─ IntervalEnd (closed)
          └─ FunctionInvocation
             ├─ Name
             │  └─ `date`
             └─ PositionalParameters
                └─ String
                   └─ `2021-12-31`
    "#;
  accept(&scope!(), StartRangeLiteral, input, expected, true);
}

#[test]
fn _0010() {
  let input = r#"[@"2012-01-01"..@"2021-12-31"]"#;
  let expected = r#"
       Range
       ├─ IntervalStart (closed)
       │  └─ At
       │     └─ `2012-01-01`
       └─ IntervalEnd (closed)
          └─ At
             └─ `2021-12-31`
    "#;
  accept(&scope!(), StartRangeLiteral, input, expected, false);
}

#[test]
fn _0011() {
  let input = r#"[false..true]"#;
  Parser::new(&scope!(), StartRangeLiteral, input).parse().unwrap_err();
}

#[test]
fn _0012() {
  // Opened start may not have a left bracket as opening character.
  let input = "[..10]";
  Parser::new(&scope!(), StartRangeLiteral, input).parse().unwrap_err();
}

#[test]
fn _0013() {
  let input = "(..10)";
  let expected = r#"
       Range
       ├─ IntervalStart (opened,undefined)
       │  └─ Null
       └─ IntervalEnd (opened)
          └─ Numeric
             └─ `10`
    "#;
  accept(&scope!(), StartRangeLiteral, input, expected, false);
}

#[test]
fn _0014() {
  let input = "]..10]";
  let expected = r#"
       Range
       ├─ IntervalStart (opened,undefined)
       │  └─ Null
       └─ IntervalEnd (closed)
          └─ Numeric
             └─ `10`
    "#;
  accept(&scope!(), StartRangeLiteral, input, expected, false);
}

#[test]
fn _0015() {
  // Opened start may not have a left bracket as opening character.
  let input = "[..10)";
  Parser::new(&scope!(), StartRangeLiteral, input).parse().unwrap_err();
}

#[test]
fn _0016() {
  // Opened end may not have a right bracket as closing character.
  let input = "[10..]";
  Parser::new(&scope!(), StartRangeLiteral, input).parse().unwrap_err();
}

#[test]
fn _0017() {
  let input = "(10..)";
  let expected = r#"
       Range
       ├─ IntervalStart (opened)
       │  └─ Numeric
       │     └─ `10`
       └─ IntervalEnd (opened,undefined)
          └─ Null
    "#;
  accept(&scope!(), StartRangeLiteral, input, expected, false);
}

#[test]
fn _0018() {
  // Opened end may not have a right bracket as closing character.
  let input = "]10..]";
  Parser::new(&scope!(), StartRangeLiteral, input).parse().unwrap_err();
}

#[test]
fn _0019() {
  let input = "[10..)";
  let expected = r#"
       Range
       ├─ IntervalStart (closed)
       │  └─ Numeric
       │     └─ `10`
       └─ IntervalEnd (opened,undefined)
          └─ Null
    "#;
  accept(&scope!(), StartRangeLiteral, input, expected, false);
}

#[test]
fn _0020() {
  // The argument to `date` function must be a string literal.
  let input = r#"[date(string("1970-01-01"))..date("1970-01-02")]"#;
  Parser::new(&scope!(), StartRangeLiteral, input).parse().unwrap_err();
}

#[test]
fn _0021() {
  let input = r#"["a".."z"]"#;
  let expected = r#"
       Range
       ├─ IntervalStart (closed)
       │  └─ String
       │     └─ `a`
       └─ IntervalEnd (closed)
          └─ String
             └─ `z`
    "#;
  accept(&scope!(), StartRangeLiteral, input, expected, false);
}

#[test]
fn _0022() {
  // The `string` function is not allowed as range endpoint definition,
  // only temporal functions: `date`, `date and time`, `time` and `duration` are allowed.
  let input = r#"[string("a").."z"]"#;
  Parser::new(&scope!(), StartRangeLiteral, input).parse().unwrap_err();
}

#[test]
fn _0023() {
  // a named parameter of a temporal function (DMN 1.6 grammar rules 60 and 67)
  let input = r#"[date(from: "2012-01-01")..date("2021-12-31")]"#;
  let expected = r#"
       Range
       ├─ IntervalStart (closed)
       │  └─ FunctionInvocation
       │     ├─ Name
       │     │  └─ `date`
       │     └─ NamedParameters
       │        └─ NamedParameter
       │           ├─ ParameterName
       │           │  └─ `from`
       │           └─ String
       │              └─ `2012-01-01`
       └─ IntervalEnd (closed)
          └─ FunctionInvocation
             ├─ Name
             │  └─ `date`
             └─ PositionalParameters
                └─ String
                   └─ `2021-12-31`
    "#;
  accept(&scope!(), StartRangeLiteral, input, expected, false);
}

#[test]
fn _0024() {
  let input = r#"(time("10:00:00")..duration(x: "P1D")]"#;
  let expected = r#"
       Range
       ├─ IntervalStart (opened)
       │  └─ FunctionInvocation
       │     ├─ Name
       │     │  └─ `time`
       │     └─ PositionalParameters
       │        └─ String
       │           └─ `10:00:00`
       └─ IntervalEnd (closed)
          └─ FunctionInvocation
             ├─ Name
             │  └─ `duration`
             └─ NamedParameters
                └─ NamedParameter
                   ├─ ParameterName
                   │  └─ `x`
                   └─ String
                      └─ `P1D`
    "#;
  accept(&scope!(), StartRangeLiteral, input, expected, false);
}

/// A range literal takes a negative number as an endpoint (grammar rule 67 with the sign of
/// rule 35), and it has at least one endpoint (rule 66).
#[test]
fn signed_endpoints_and_no_range_without_an_endpoint() {
  let scope = scope!();
  let parse = |input: &str| Parser::new(&scope, StartRangeLiteral, input).parse().map(|node| node.trace());
  for input in ["[-1..1]", "(-2.5..-1)", "]-1e3..0[", "[0..-1]", "(..-5]", "[-5..)", "]-1..["] {
    let tree = parse(input).unwrap_or_else(|reason| panic!("{input}: {reason}"));
    assert!(tree.contains("Neg") && tree.contains("Range"), "{input}: {tree}");
  }
  // as before
  for input in [
    "[1..2]",
    "(1..2)",
    "]1..2[",
    "(..2]",
    "[1..)",
    "]..2)",
    "[\"a\"..\"z\"]",
    "[@\"2026-01-01\"..@\"2026-12-31\"]",
  ] {
    assert!(parse(input).is_ok(), "{input}");
  }
  for input in ["(..)", "]..[", "(..[", "]..)", "[--1..1]", "[- ..1]", "[-\"a\"..\"b\"]", "[1..-]", "[..]"] {
    assert!(parse(input).is_err(), "{input}: {:?}", parse(input));
  }
}
