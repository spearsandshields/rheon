use crate::Dialect;
use crate::builders::{build_evaluator, build_evaluator_with_dialect};
use rheon_feel::{FeelScope, scope};

// B-FEEL (DMN 1.6 chapter 11): the spec's own examples, compared with FEEL where they differ.

/// Asserts the B-FEEL result of `expr` (`expected` is a FEEL expression evaluated in FEEL) and,
/// when `feel` is given, the plain FEEL result.
fn bf(expr: &str, expected: &str, feel: Option<&str>) {
  let scope: FeelScope = scope!();
  let node = rheon_feel_parser::parse_expression(&scope, expr, false).unwrap();
  let actual = build_evaluator_with_dialect(&node, Dialect::BFeel)(&scope);
  let expected_value = build_evaluator(&rheon_feel_parser::parse_expression(&scope, expected, false).unwrap())(&scope);
  assert_eq!(actual.to_string(), expected_value.to_string(), "B-FEEL {expr}");
  if let Some(feel) = feel {
    let feel_actual = build_evaluator(&node)(&scope);
    if feel == "null" {
      assert!(feel_actual.is_null(), "FEEL {expr}: expected null, actual {feel_actual}");
    } else {
      assert_eq!(feel_actual.to_string(), feel, "FEEL {expr}");
    }
  }
}

#[test]
fn _11_2_booleans() {
  for (e, x) in [
    (r#""a" = 1"#, "false"),
    (r#""a" < 1"#, "false"),
    (r#""a" <= null"#, "false"),
    (r#""a" > 1"#, "false"),
    ("null >= 1", "false"),
    (r#"not("a")"#, "false"),
    (r#"true and "x""#, "false"),
    (r#"false or "x""#, "false"),
    (r#""a" in [1..100]"#, "false"),
    ("null between 1 and 100", "false"),
    (r#""a" != 1"#, "true"),
    (r#"matches("bad pattern","[0-9")"#, "false"),
    (r#"before(date("2021-01-01"), null)"#, "false"),
    (r#"all(true,"x",true)"#, "false"),
    ("any(null)", "false"),
  ] {
    bf(e, x, Some("null"));
  }
}

#[test]
fn _11_3_numbers() {
  bf(r#"decimal("a", 0)"#, "0", Some("null"));
  bf(r#"round up("5.5", 0)"#, "0", Some("null"));
  bf("string length(22)", "0", Some("null"));
  bf(r#"day of year("a")"#, "0", Some("null"));
  bf("count([1,null,3])", "3", Some("3"));
  bf("sum([1, null, 3])", "4", None);
  bf(r#"sum([1, "1" ,3])"#, "4", Some("null"));
  bf("sum([])", "0", None);
  bf(r#"mean(["a"])"#, "0", Some("null"));
  bf(r#"mean([1, "a", 3])"#, "2", Some("null"));
}

#[test]
fn _11_4_to_11_8_defaults() {
  bf("lower case(12)", r#""""#, Some("null"));
  bf("string(null)", r#""""#, Some("null"));
  bf(r#"day of week("a")"#, r#""""#, Some("null"));
  bf(r#"substring("a", "z")"#, r#""""#, Some("null"));
  bf(r#"time("a") = time("00:00:00+00:00")"#, "true", None);
  bf("date(null)", r#"date("1970-01-01")"#, Some("null"));
  bf(r#"duration("a")"#, r#"duration("P0M")"#, Some("null"));
  bf("years and months duration(null, null)", r#"duration("P0M")"#, Some("null"));
  bf(r#"split("abc", 22)"#, "[]", Some("null"));
  bf("mode([null,null,null, 1, 1,2])", "[1]", None);
  bf(r#"range("x") = range("(0..0)")"#, "true", None);
}

#[test]
fn _11_9_to_11_11_arithmetic() {
  bf(r#""Today is " + date("2020-01-01")"#, r#""Today is 2020-01-01""#, Some("null"));
  bf(r#""The result is: " + 1"#, r#""The result is: 1""#, Some("null"));
  bf(r#"5 + " minutes""#, r#""5 minutes""#, Some("null"));
  bf(r#""This is " + null"#, r#""This is ""#, Some("null"));
  bf("1 + null", "1", Some("null"));
  bf("null - 6", "-6", Some("null"));
  bf(r#"date("2021-01-01") + 7"#, "7", Some("null"));
  bf(r#""abc" - 2"#, r#""""#, Some("null"));
  bf(r#"22 * "a""#, "0", Some("null"));
  bf("null / 22", "0", Some("null"));
  bf(r#"duration("P1Y") * null"#, r#"duration("P0M")"#, Some("null"));
  bf(r#"2 ** "a""#, "1", Some("null"));
  // a non-null FEEL result is kept
  bf("1 + 2", "3", Some("3"));
  bf(r#""a" + "b""#, r#""ab""#, Some(r#""ab""#));
}
