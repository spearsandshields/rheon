use super::super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_be_value(false, &scope!(), "exp(5)", r#"148.4131591025766034211155800405523"#);
}

#[test]
fn _0002() {
  te_be_value(false, &scope!(), "exp(4)", r#"54.59815003314423907811026120286088"#);
}

#[test]
fn _0003() {
  te_be_value(false, &scope!(), "exp(-1)", r#"0.3678794411714423215955237701614609"#);
}

#[test]
fn _0004() {
  te_be_value(false, &scope!(), "exp(0)", r#"1"#);
}

#[test]
fn _0005() {
  te_be_value(false, &scope!(), "exp(number:4)", r#"54.59815003314423907811026120286088"#);
}

#[test]
fn _0006() {
  te_null(false, &scope!(), "exp(n:4)", r#"parameter 'number' not found"#);
}

#[test]
fn _0007() {
  te_null(false, &scope!(), "exp()", "expected 1 parameters, actual number of parameters is 0");
}

#[test]
fn _0008() {
  te_null(false, &scope!(), "exp(4,4)", "expected 1 parameters, actual number of parameters is 2");
}

#[test]
fn _0009() {
  te_null(false, &scope!(), "exp(null)", "[core::exp] invalid argument type, expected number, actual type is Null");
}

#[test]
fn _0010() {
  te_null(false, &scope!(), r#"exp("4")"#, "[core::exp] invalid argument type, expected number, actual type is string");
}

#[test]
fn _0011() {
  te_null(false, &scope!(), "exp(true)", "[core::exp] invalid argument type, expected number, actual type is boolean");
}

#[test]
fn _0012() {
  te_null(
    false,
    &scope!(),
    r#"exp(duration("P4D"))"#,
    "[core::exp] invalid argument type, expected number, actual type is days and time duration",
  );
}

#[test]
fn _0013() {
  te_null(
    false,
    &scope!(),
    r#"exp(duration("P4Y"))"#,
    "[core::exp] invalid argument type, expected number, actual type is years and months duration",
  );
}

#[test]
fn _0014() {
  te_null(
    false,
    &scope!(),
    r#"exp(date("2018-12-06"))"#,
    "[core::exp] invalid argument type, expected number, actual type is date",
  );
}

#[test]
fn _0015() {
  te_null(
    false,
    &scope!(),
    r#"exp(time("00:00:00"))"#,
    "[core::exp] invalid argument type, expected number, actual type is time",
  );
}

#[test]
fn _0016() {
  te_null(
    false,
    &scope!(),
    r#"exp(date and time("2018-12-06T00:00:00"))"#,
    "[core::exp] invalid argument type, expected number, actual type is date and time",
  );
}

#[test]
fn _0017() {
  te_null(false, &scope!(), r#"exp(10000000000000000000000)"#, "exp: result is not a finite number (decimal overflow)");
}
