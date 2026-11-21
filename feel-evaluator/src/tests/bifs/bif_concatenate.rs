use super::super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_be_value(false, &scope!(), r#"concatenate([1],[2],[3])"#, r#"[1,2,3]"#);
}

#[test]
fn _0002() {
  te_be_value(false, &scope!(), r#"concatenate([1,2],[3])"#, r#"[1,2,3]"#);
}

#[test]
fn _0003() {
  te_be_value(false, &scope!(), r#"concatenate([1,2],["A","B","C"])"#, r#"[1,2,"A","B","C"]"#);
}

#[test]
fn _0004() {
  // a value that is no list is the list of it where a list is taken (10.3.2.9.4)
  te_be_value(false, &scope!(), r#"concatenate(1,[2])"#, r#"[1, 2]"#);
}

#[test]
fn _0005() {
  te_null(false, &scope!(), r#"concatenate()"#, r#"expected 1+ parameters, actual number of parameters is 0"#);
}

#[test]
fn _0006() {
  te_null(
    false,
    &scope!(),
    r#"concatenate(list: [1,2,3])"#,
    r#"[named::concatenate] this function has no version with named parameters"#,
  );
}
