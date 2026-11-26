use super::super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_be_value(false, &scope!(), r#"reverse([1,2,3,4,5])"#, r#"[5,4,3,2,1]"#);
}

#[test]
fn _0002() {
  te_be_value(false, &scope!(), r#"reverse([1,true,3,"z",5])"#, r#"[5,"z",3,true,1]"#);
}

#[test]
fn _0003() {
  te_be_value(false, &scope!(), r#"reverse([])"#, r#"[]"#);
}

#[test]
fn _0004() {
  te_be_value(false, &scope!(), r#"reverse(list: [1,2,3,4,5])"#, r#"[5,4,3,2,1]"#);
}

#[test]
fn _0005() {
  // a value that is no list is the list of it where a list is taken (10.3.2.9.4)
  te_be_value(false, &scope!(), r#"reverse(10)"#, r#"[10]"#);
}

#[test]
fn _0006() {
  // a value that is no list is the list of it where a list is taken (10.3.2.9.4)
  te_be_value(false, &scope!(), r#"reverse(list:true)"#, r#"[true]"#);
}

#[test]
fn _0007() {
  te_null(false, &scope!(), r#"reverse()"#, r#"expected 1 parameters, actual number of parameters is 0"#);
}

#[test]
fn _0008() {
  te_null(
    false,
    &scope!(),
    r#"reverse(l:[10,11,12])"#,
    r#"expected the parameters of reverse(list): list, found the parameter name 'l'"#,
  );
}
