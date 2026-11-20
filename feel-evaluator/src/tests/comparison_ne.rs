use super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_bool(false, &scope!(), r#"1!=1"#, false);
}

#[test]
fn _0002() {
  te_bool(false, &scope!(), r#"1.25!=2.11"#, true);
}

#[test]
fn _0003() {
  te_bool(false, &scope!(), r#"1!=2"#, true);
}

#[test]
fn _0004() {
  te_bool(false, &scope!(), r#"(1+1)!=2"#, false);
}

#[test]
fn _0005() {
  te_bool(false, &scope!(), r#"(1+2)!=2"#, true);
}

#[test]
fn _0006() {
  te_bool(false, &scope!(), r#" ( 1 + 2 ) != 2"#, true);
}

#[test]
fn _0007() {
  te_bool(false, &scope!(), r#" ( 1 + 3 ) != 4"#, false);
}

#[test]
fn _0008() {
  te_bool(false, &scope!(), r#"(1+0.3)!=(5-3)"#, true);
}

#[test]
fn _0009() {
  te_bool(false, &scope!(), r#"(1*2)!=(10/5.1)"#, true);
}

#[test]
fn _0010() {
  te_bool(false, &scope!(), r#"true != false"#, true);
}

#[test]
fn _0011() {
  te_bool(false, &scope!(), r#" "alfa" != "beta" "#, true);
}

#[test]
fn _0012() {
  te_bool(false, &scope!(), r#" "alfa" != "alfa" "#, false);
}

#[test]
fn _0013() {
  te_bool(false, &scope!(), r#" @"2023-02-06" != @"2023-02-07" "#, true);
}

#[test]
fn _0014() {
  te_bool(false, &scope!(), r#" @"2023-02-06" != @"2023-02-06" "#, false);
}

#[test]
fn _0015() {
  te_bool(false, &scope!(), r#" @"2023-02-06T10:11:12" != @"2023-02-06T10:11:13" "#, true);
}

#[test]
fn _0016() {
  te_bool(false, &scope!(), r#" @"2023-02-06T10:11:12" != @"2023-02-06T10:11:12" "#, false);
}

#[test]
fn _0017() {
  te_bool(false, &scope!(), r#" @"10:11:12" != @"10:11:13" "#, true);
}

#[test]
fn _0018() {
  te_bool(false, &scope!(), r#" @"10:11:12" != @"10:11:12" "#, false);
}

#[test]
fn _0019() {
  te_bool(false, &scope!(), r#" @"P1D" != @"P2D" "#, true);
}

#[test]
fn _0020() {
  te_bool(false, &scope!(), r#" @"P1D" != @"P1D" "#, false);
}

#[test]
fn _0021() {
  te_bool(false, &scope!(), r#" @"P1Y" != @"P2Y" "#, true);
}

#[test]
fn _0022() {
  te_bool(false, &scope!(), r#" @"P1Y" != @"P1Y" "#, false);
}

#[test]
fn _0023() {
  for value in ["100", r#""a""#, "true", "[1,2,3]", "{a: 1}", r#"@"2018-12-08""#, r#"@"P1D""#, "(1..2]"] {
    te_bool(false, &scope!(), &format!("null != {value}"), true);
    te_bool(false, &scope!(), &format!("{value} != null"), true);
  }
  te_bool(false, &scope!(), "null != null", false);
}

#[test]
fn _0024() {
  let scope = &te_scope("{x: null, y: null, n: 100}");
  te_bool(false, scope, "x != 100", true);
  te_bool(false, scope, "100 != x", true);
  te_bool(false, scope, "x != n", true);
  te_bool(false, scope, "n != x", true);
  te_bool(false, scope, "x != null", false);
  te_bool(false, scope, "null != x", false);
  te_bool(false, scope, "x != y", false);
}

#[test]
fn not_equal_on_kinds_that_equal_cannot_compare_reports_like_equal() {
  // DMN 1.6 §10.3.2.15: "By definition, FEEL(e1 != e2) is FEEL(not(e 1= e2))"
  for (lhs, rhs, lhs_text, rhs_text) in [
    (r#""a""#, "1", r#""a""#, "1"),
    ("10", r#"{c1: {a: {c: "bar", b: "foo"}}}"#, "10", r#"{c1: {a: {c: "bar", b: "foo"}}}"#),
    (r#"{a: {b: "bar", c: "car"}}"#, "15", r#"{a: {b: "bar", c: "car"}}"#, "15"),
    ("[1,2,3]", r#""a""#, "[1, 2, 3]", r#""a""#),
    (r#"@"2018-12-08""#, "2018", "2018-12-08", "2018"),
    (r#"@"2018-12-08T10:11:12""#, "2018", "2018-12-08T10:11:12", "2018"),
    (r#"@"10:11:12""#, "10", "10:11:12", "10"),
    (r#"@"P1D""#, r#""P1D""#, "P1D", r#""P1D""#),
    (r#"@"P1Y""#, r#""P1Y""#, "P1Y", r#""P1Y""#),
    ("(1..2]", "10", "(1..2]", "10"),
  ] {
    let reason = format!("equal err '{lhs_text}' =?= '{rhs_text}'");
    te_errors(&format!("{lhs} = {rhs}"), &format!("null({reason})"), &[&reason]);
    te_errors(&format!("{lhs} != {rhs}"), &format!("null({reason})"), &[&reason]);
  }
}
