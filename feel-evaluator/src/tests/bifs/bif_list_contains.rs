use super::super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_bool(false, &scope!(), r#"list contains([1,2,3,4],2)"#, true);
}

#[test]
fn _0002() {
  te_bool(false, &scope!(), r#"list contains([1,2,null,4],null)"#, true);
}

#[test]
fn _0003() {
  te_bool(false, &scope!(), r#"list contains([1,2,3,4],8)"#, false);
}

#[test]
fn _0004() {
  te_bool(false, &scope!(), r#"list contains(list: [1,2,3,4],element: 2)"#, true);
}

#[test]
fn _0005() {
  te_null(false, &scope!(), r#"list contains()"#, r#"expected 2 parameters, actual number of parameters is 0"#);
}

#[test]
fn _0006() {
  te_null(false, &scope!(), r#"list contains([1,2,3])"#, r#"expected 2 parameters, actual number of parameters is 1"#);
}

#[test]
fn _0007() {
  te_null(
    false,
    &scope!(),
    r#"list contains([1,2,3],2,6)"#,
    r#"expected 2 parameters, actual number of parameters is 3"#,
  );
}

#[test]
fn _0008() {
  // a value that is no list is the list of it where a list is taken (10.3.2.9.4)
  te_be_value(false, &scope!(), r#"list contains(10,2)"#, r#"false"#);
}

#[test]
fn _0009() {
  te_null(
    false,
    &scope!(),
    r#"list contains(l:[10,11,12],match:11)"#,
    r#"expected the parameters of list contains(list, element): boolean, found the parameter name 'l'"#,
  );
}

#[test]
fn _0010() {
  te_null(
    false,
    &scope!(),
    r#"list contains(list:[10,11,12],m:11)"#,
    r#"expected the parameters of list contains(list, element): boolean, found the parameter name 'm'"#,
  );
}

#[test]
fn _0011() {
  // the second parameter is named element (DMN 1.7 Beta 1, 10.3.4.4), not match
  te_null(
    false,
    &scope!(),
    r#"list contains(list: [1, 2], match: 2)"#,
    "expected the parameters of list contains(list, element): boolean, found the parameter name 'match'",
  );
}
