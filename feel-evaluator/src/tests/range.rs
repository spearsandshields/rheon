use super::*;
use rheon_feel::{IntervalType, scope};

#[test]
fn _0001() {
  te_bool(false, &scope!(), r#"2 in [1..5]"#, true);
}

#[test]
fn _0002() {
  te_null(false, &scope!(), r#"2 in ["a".."z"]"#, "eval_in_range");
}

#[test]
fn _0003() {
  te_null(false, &scope!(), r#"2 in [1.."z"]"#, "eval_in_range");
}

#[test]
fn _0004() {
  te_bool(false, &scope!(), r#"0 in [1..5]"#, false);
}

#[test]
fn _0005() {
  te_bool(false, &scope!(), r#"6 in [1..5]"#, false);
}

#[test]
fn _0006() {
  te_bool(false, &scope!(), r#"1 in [1..5]"#, true);
}

#[test]
fn _0007() {
  te_bool(false, &scope!(), r#"5 in [1..5]"#, true);
}

#[test]
fn _0008() {
  te_bool(false, &scope!(), r#"1 in (1..5]"#, false);
}

#[test]
fn _0009() {
  te_bool(false, &scope!(), r#"5 in [1..5)"#, false);
}

#[test]
fn _0010() {
  te_bool(false, &scope!(), r#"1 in (1..5)"#, false);
}

#[test]
fn _0011() {
  te_bool(false, &scope!(), r#"1.01 in (1..5)"#, true);
}

#[test]
fn _0012() {
  te_bool(false, &scope!(), r#"5 in (1..5)"#, false);
}

#[test]
fn _0013() {
  te_bool(false, &scope!(), r#"4.99 in (1..5)"#, true);
}

#[test]
fn _0014() {
  te_bool(false, &scope!(), r#"1 in ]1..5]"#, false);
}

#[test]
fn _0015() {
  te_bool(false, &scope!(), r#"5 in [1..5["#, false);
}

#[test]
fn _0016() {
  te_bool(false, &scope!(), r#"1 in ]1..5["#, false);
}

#[test]
fn _0017() {
  te_bool(false, &scope!(), r#"5 in ]1..5["#, false);
}

#[test]
fn _0018() {
  te_bool(false, &scope!(), r#"1 in [1..5] and 5 in [1..5]"#, true);
}

#[test]
fn _0019() {
  te_bool(false, &scope!(), r#"1 in [1..5] or 6 in [1..5]"#, true);
}

#[test]
fn _0020() {
  te_bool(false, &scope!(), r#" @"P3D" in [@"P1D"..@"P5D"]"#, true);
}

#[test]
fn _0021() {
  te_bool(false, &scope!(), r#" @"P5D" in [@"P1D"..@"P5D")"#, false);
}

#[test]
fn _0022() {
  te_null(false, &scope!(), r#" "k" in [1.."z"]"#, "eval_in_range");
}

#[test]
fn _0023() {
  te_null(false, &scope!(), r#" "k" in ["a"..34]"#, "eval_in_range");
}

#[test]
fn _0024() {
  te_null(false, &scope!(), r#" @"2023-02-06" in ["2023-02-01"..@"2023-02-28"]"#, "eval_in_range");
}

#[test]
fn _0025() {
  te_null(false, &scope!(), r#" @"2023-02-06" in [@"2023-02-01".."2023-02-28"]"#, "eval_in_range");
}

#[test]
fn _0026() {
  te_null(
    false,
    &scope!(),
    r#" @"2023-02-06T10:11:12" in ["2023-02-01T00:00:00"..@"2023-02-28T23:59:59"]"#,
    "eval_in_range",
  );
}

#[test]
fn _0027() {
  te_null(
    false,
    &scope!(),
    r#" @"2023-02-06T10:11:12" in [@"2023-02-01T00:00:00".."2023-02-28T23:59:59"]"#,
    "eval_in_range",
  );
}

#[test]
fn _0028() {
  te_null(false, &scope!(), r#" @"10:11:12" in ["00:00:00"..@"23:59:59"]"#, "eval_in_range");
}

#[test]
fn _0029() {
  te_null(false, &scope!(), r#" @"10:11:12" in [@"00:00:00".."23:59:59"]"#, "eval_in_range");
}

#[test]
fn _0030() {
  te_null(false, &scope!(), r#" @"P5D" in [1..@"P5D"]"#, "eval_in_range");
}

#[test]
fn _0031() {
  te_null(false, &scope!(), r#" @"P5D" in [@"P1D"..5]"#, "eval_in_range");
}

#[test]
fn _0032() {
  te_null(false, &scope!(), r#" @"P5Y" in [1..@"P5Y"]"#, "eval_in_range");
}

#[test]
fn _0033() {
  te_null(false, &scope!(), r#" @"P5Y" in [@"P1Y"..5]"#, "eval_in_range");
}

#[test]
fn _0034() {
  te_null(false, &scope!(), r#" true in [false..true]"#, "eval_in_range");
}

#[test]
fn _0035() {
  let node = AstNode::Range(
    Box::new(AstNode::IntervalStart(
      Box::new(AstNode::Numeric("1".into(), "0".into(), '+', "".into())),
      IntervalType::Opened,
    )),
    Box::new(AstNode::IntervalEnd(
      Box::new(AstNode::Numeric("5".into(), "0".into(), '+', "".into())),
      IntervalType::Opened,
    )),
  );
  assert_eq!(r#"(1..5)"#, crate::tests::eval(&scope!(), &node).to_string());
}

#[test]
fn _0036() {
  let node = AstNode::Range(
    Box::new(AstNode::IntervalEnd(
      Box::new(AstNode::Numeric("5".into(), "0".into(), '+', "".into())),
      IntervalType::Opened,
    )),
    Box::new(AstNode::IntervalStart(
      Box::new(AstNode::Numeric("1".into(), "0".into(), '+', "".into())),
      IntervalType::Opened,
    )),
  );
  assert_eq!(r#"null(expected interval start)"#, crate::tests::eval(&scope!(), &node).to_string());
}

#[test]
fn _0037() {
  let node = AstNode::Range(
    Box::new(AstNode::IntervalStart(
      Box::new(AstNode::Numeric("1".into(), "0".into(), '+', "".into())),
      IntervalType::Opened,
    )),
    Box::new(AstNode::IntervalStart(
      Box::new(AstNode::Numeric("1".into(), "0".into(), '+', "".into())),
      IntervalType::Opened,
    )),
  );
  assert_eq!(r#"null(expected interval end)"#, crate::tests::eval(&scope!(), &node).to_string());
}

/// A comparison with one endpoint is a range (DMN 1.6 clause 10.3.2.7): it has the type of a
/// range and is shown as it is written.
#[test]
fn a_comparison_with_one_endpoint_is_a_range() {
  let scope = scope!();
  for (text, shown) in [("< 10", "< 10"), ("<= 10", "<= 10"), ("> 1", "> 1"), (">= 1", ">= 1"), ("= 5", "= 5"), ("!= 5", "!= 5")] {
    te_value(false, &scope, &format!("string({text})"), &format!("\"{shown}\""));
    te_bool(false, &scope, &format!("({text}) instance of range<number>"), true);
    te_bool(false, &scope, &format!("({text}) instance of range<string>"), false);
    te_bool(false, &scope, &format!("({text}) instance of boolean"), false);
  }
  te_bool(false, &scope, r#"(>= "a") instance of range<string>"#, true);
  te_value(false, &scope, "string([< 10, >= 2])", r#""[< 10, >= 2]""#);
  te_value(false, &scope, "(function(r: range<number>) r.end)(< 10)", "10");
  te_null(
    false,
    &scope,
    "(function(r: boolean) r)(< 10)",
    "argument for parameter 'r' does not conform to type boolean",
  );
}

/// DMN 1.6 clause 10.3.2.7 and Table 42: a comparison with one endpoint is the range whose other
/// endpoint is undefined and not included; `range("(..2)")` is `< 2`.
#[test]
fn a_comparison_with_one_endpoint_equals_the_range_with_the_other_endpoint_undefined() {
  let scope = &scope!();
  for (range, comparison) in [
    (r#"range("(..10)")"#, "(< 10)"),
    (r#"range("(..10]")"#, "(<= 10)"),
    (r#"range("(10..)")"#, "(> 10)"),
    (r#"range("[10..)")"#, "(>= 10)"),
  ] {
    te_bool(false, scope, &format!("{range} = {comparison}"), true);
    te_bool(false, scope, &format!("{comparison} = {range}"), true);
    te_bool(false, scope, &format!("{comparison} != {range}"), false);
    te_bool(false, scope, &format!("{range} instance of range<number>"), true);
    te_bool(false, scope, &format!("{range} instance of range<string>"), false);
  }
  // another endpoint, another flag, or two endpoints: another range
  te_bool(false, scope, r#"(< 10) = range("(..11)")"#, false);
  te_bool(false, scope, r#"(< 10) = range("(..10]")"#, false);
  te_bool(false, scope, "(< 10) = (<= 10)", false);
  te_bool(false, scope, "(< 10) = (> 10)", false);
  te_bool(false, scope, "(< 10) = (1..10)", false);
  te_bool(false, scope, "(1..10) = (< 10)", false);
  te_bool(false, scope, "(> 1) = (1..10)", false);
  // the undefined endpoint is null, without an error
  te_null(false, scope, "(< 10).start", "");
  te_null(false, scope, "(<= 10).start", "");
  te_null(false, scope, "(> 10).end", "");
  te_null(false, scope, "(>= 10).end", "");
  te_null(false, scope, r#"range("(..10)").start"#, "");
  te_number(false, scope, "(< 10).end", 10, 0);
  te_number(false, scope, "(>= 10).start", 10, 0);
  // an endpoint written as `null` is a value, not an undefined endpoint (TCK 0068, range_006 to 009)
  te_bool(false, scope, "(< 10) = (null..10)", false);
  te_bool(false, scope, "(null..10) = (< 10)", false);
  te_bool(false, scope, r#"range("(..10)") = (null..10)"#, false);
  // no range: not compared
  te_null(false, scope, "(< 10) = 10", "equal err '< 10' =?= '10'");
}
