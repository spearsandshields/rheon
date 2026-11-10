use super::super::*;
use rheon_feel::scope;

// DMN 1.7 beta 1 (DMN17-98), adopted early. The result order is unspecified; rheon keeps the
// order of the first list.

#[test]
fn _0001() {
  te_be_value(false, &scope!(), "intersection([1,2],[2,3],[2,3,4])", "[2]");
  te_be_value(false, &scope!(), r#"intersection(["a","b","c"],["a","b"])"#, r#"["a","b"]"#);
  te_be_value(false, &scope!(), "intersection([1],[2])", "[]");
}

#[test]
fn _0002() {
  te_be_value(false, &scope!(), "intersection([1,1,2],[1,2,2])", "[1,2]");
  te_be_value(false, &scope!(), "intersection([1,2,3])", "[1,2,3]");
}

#[test]
fn _0003() {
  te_null(false, &scope!(), "intersection([1,2,3], null)", "");
}
