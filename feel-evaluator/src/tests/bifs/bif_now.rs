use super::super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_date_time_local_after(false, &scope!(), "now()", FeelDateTime::now(), 1);
}

#[test]
fn _0002() {
  te_null(
    false,
    &scope!(),
    r#"now(when: 1)"#,
    r#"expected the parameters of now(): date and time, found the parameter name 'when'"#,
  );
}
