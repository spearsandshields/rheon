use super::super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  let (year, month, day) = FeelDate::today().as_tuple();
  te_date(false, &scope!(), "today()", year, month, day);
}

#[test]
fn _0002() {
  te_null(
    false,
    &scope!(),
    r#"today(when: 1)"#,
    r#"expected the parameters of today(): date, found the parameter name 'when'"#,
  );
}
