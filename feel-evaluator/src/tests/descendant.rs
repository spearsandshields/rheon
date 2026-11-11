use super::*;
use rheon_feel::scope;

// DMN 1.6 grammar rule 68, Table 66 row 68 (DMN16-72): e...name

#[test]
fn _0001() {
  // the normative example
  te_be_value(false, &scope!(), "{ a: { b: { b: 1 } } }...b", "[{ b: 1 }, 1]");
}

#[test]
fn _0002() {
  te_be_value(false, &scope!(), "{ b: 1 } ... b", "[1]");
  te_be_value(false, &scope!(), "{ b: { b: { b: 1 } } }...b", "[{ b: { b: 1 } }, { b: 1 }, 1]");
  te_be_value(false, &scope!(), "{ a: 1, c: { d: 2 } }...b", "[]");
  te_be_value(false, &scope!(), "{}...b", "[]");
}

#[test]
fn _0003() {
  te_be_value(false, &scope!(), r#"{ a: { first name: "x" } }...first name"#, r#"["x"]"#);
  te_be_value(
    false,
    &scope!(),
    r#"({ deal: { party: { role: "Appraiser", name: "Ann" } } }...party)[role = "Appraiser"].name"#,
    r#"["Ann"]"#,
  );
}

#[test]
fn _0004() {
  te_null(false, &scope!(), r#""abc"...b"#, "descendant expression: expected context, actual type is string");
  te_null(false, &scope!(), "null...b", "descendant expression: expected context, actual type is Null");
}

#[test]
fn _0005() {
  // `..` still separates range endpoints
  te_be_value(false, &scope!(), "[1..5]", "[1..5]");
  te_be_value(false, &scope!(), "count(for i in 1..3 return i)", "3");
}
