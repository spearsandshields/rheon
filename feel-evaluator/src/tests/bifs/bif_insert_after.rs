use super::super::*;
use rheon_feel::scope;

// DMN 1.7 beta 1 (DMN17-100), adopted early.

#[test]
fn _0001() {
  te_be_value(false, &scope!(), "insert after([1,3],1,2)", "[1,2,3]");
}

#[test]
fn _0002() {
  te_be_value(false, &scope!(), "insert after(list: [1,3], position: 1, newItem: 2)", "[1,2,3]");
}

#[test]
fn _0003() {
  te_be_value(false, &scope!(), "insert after([1,3],2,4)", "[1,3,4]");
  te_be_value(false, &scope!(), "insert after([1,3],-1,4)", "[1,3,4]");
  te_be_value(false, &scope!(), "insert after([1,3],-2,2)", "[1,2,3]");
}

#[test]
fn _0004() {
  te_be_value(false, &scope!(), "insert after([1,3],1,null)", "[1,null,3]");
  te_be_value(false, &scope!(), r#"insert after([1,3],1,"2")"#, r#"[1,"2",3]"#);
}

#[test]
fn _0005() {
  te_null(false, &scope!(), "insert after([1,3],null,2)", "index is out of range");
  te_null(false, &scope!(), "insert after([1,3],0,2)", "index is out of range");
  te_null(false, &scope!(), "insert after([1,3],3,2)", "index is out of range");
  te_null(false, &scope!(), "insert after([1,3],-3,2)", "index is out of range");
}
