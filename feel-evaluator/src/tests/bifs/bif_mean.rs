use super::super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_number(false, &scope!(), r#"mean([1,2,3])"#, 2, 0);
}

#[test]
fn _0002() {
  te_number(false, &scope!(), r#"mean(1,2,3,4,5,6)"#, 35, 1);
}

#[test]
fn _0003() {
  te_number(false, &scope!(), r#"mean(list: [1,2,3,4])"#, 25, 1);
}

#[test]
fn _0004() {
  te_null(false, &scope!(), r#"mean([])"#, r#""#);
}

#[test]
fn _0005() {
  te_null(
    false,
    &scope!(),
    r#"mean(true)"#,
    r#"[core::mean] invalid argument type, expected number, actual type is boolean"#,
  );
}

#[test]
fn _0006() {
  te_null(
    false,
    &scope!(),
    r#"mean(l: [1,2,3])"#,
    r#"expected the parameters of mean(list): number, found the parameter name 'l'"#,
  );
}

#[test]
fn _0007() {
  te_null(false, &scope!(), r#"mean()"#, r#"expected 1+ parameters, actual number of parameters is 0"#);
}

/// DMN 1.6 §10.3.4.4: `mean([]) = null`, a null the specification defines, so no error.
#[test]
fn the_mean_of_an_empty_list_is_a_null_without_an_error() {
  te_errors("mean([])", "null", &[]);
}
