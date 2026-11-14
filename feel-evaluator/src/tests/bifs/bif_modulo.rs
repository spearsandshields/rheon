use super::super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_null(false, &scope!(), r#"modulo()"#, r#"expected 2 parameters, actual number of parameters is 0"#);
}

#[test]
fn _0002() {
  te_null(false, &scope!(), r#"modulo(1)"#, r#"expected 2 parameters, actual number of parameters is 1"#);
}

#[test]
fn _0003() {
  te_null(false, &scope!(), r#"modulo(1,2,3)"#, r#"expected 2 parameters, actual number of parameters is 3"#);
}

#[test]
fn _0004() {
  te_null(
    false,
    &scope!(),
    r#"modulo(1, true)"#,
    r#"[core::modulo] invalid argument type, expected number, actual type is boolean"#,
  );
}

#[test]
fn _0005() {
  te_null(false, &scope!(), r#"modulo(a:1,b:2)"#, r#"parameter 'dividend' not found"#);
}

#[test]
fn _0006() {
  te_number(false, &scope!(), r#"modulo(1,1)"#, 0, 0);
}

#[test]
fn _0007() {
  te_number(false, &scope!(), r#"modulo(1,2)"#, 1, 0);
}

#[test]
fn _0008() {
  te_number(false, &scope!(), r#"modulo(dividend:1,divisor:2)"#, 1, 0);
}

#[test]
fn _0009() {
  te_number(false, &scope!(), r#"modulo(modulo(10,6),3)"#, 1, 0);
}

#[test]
fn _0010() {
  te_number(false, &scope!(), r#"modulo(-12,5)"#, 3, 0);
}

#[test]
fn _0011() {
  te_number(false, &scope!(), r#"modulo(12,5)"#, 2, 0);
}

#[test]
fn _0012() {
  te_number(false, &scope!(), r#"modulo(12,-5)"#, -3, 0);
}

#[test]
fn _0013() {
  te_number(false, &scope!(), r#"modulo(-12,-5)"#, -2, 0);
}

#[test]
fn _0014() {
  te_number(false, &scope!(), r#"decimal(modulo(10.1,4.5),1)"#, 11, 1);
}

#[test]
fn _0015() {
  te_number(false, &scope!(), r#"decimal(modulo(-10.1,4.5),1)"#, 34, 1);
}

#[test]
fn _0016() {
  te_number(false, &scope!(), r#"decimal(modulo(10.1,-4.5),1)"#, -34, 1);
}

#[test]
fn _0017() {
  te_number(false, &scope!(), r#"decimal(modulo(-10.1,-4.5),1)"#, -11, 1);
}

#[test]
fn _0018() {
  te_null(false, &scope!(), r#"modulo(12,0)"#, r#"[core::modulo] division by zero"#);
}

#[test]
fn _0019() {
  te_null(
    false,
    &scope!(),
    r#"modulo(true,2)"#,
    r#"[core::modulo] invalid argument type, expected number, actual type is boolean"#,
  );
}

#[test]
fn _0020() {
  te_null(
    false,
    &scope!(),
    r#"modulo(12,"2")"#,
    r#"[core::modulo] invalid argument type, expected number, actual type is string"#,
  );
}

#[test]
fn _0021() {
  te_null(false, &scope!(), r#"modulo(dividend: 12, d: 6)"#, r#"parameter 'divisor' not found"#);
}

#[test]
fn _0022() {
  te_null(false, &scope!(), r#"modulo(d: 12, divisor: 6)"#, r#"parameter 'dividend' not found"#);
}

#[test]
fn table_77_examples() {
  te_number_x(false, &scope!(), r#"modulo(10.1, 4.5)"#, "1.1");
  te_number_x(false, &scope!(), r#"modulo(-10.1, 4.5)"#, "3.4");
  te_number_x(false, &scope!(), r#"modulo(10.1, -4.5)"#, "-3.4");
  te_number_x(false, &scope!(), r#"modulo(-10.1, -4.5)"#, "-1.1");
}

#[test]
fn the_remainder_is_exact() {
  // DMN 1.6 Table 77: modulo(dividend, divisor) = dividend - divisor*floor(dividend/divisor),
  // over the exact values, rounded once to 34 digits
  te_number_x(false, &scope!(), r#"modulo(1e40, 3)"#, "1");
  te_number_x(false, &scope!(), r#"modulo(1e40, 7)"#, "4");
  te_number_x(false, &scope!(), r#"modulo(1e34, 7)"#, "4");
  te_number_x(false, &scope!(), r#"modulo(-1e40, 7)"#, "3");
  te_number_x(false, &scope!(), r#"modulo(5e33, 0.7)"#, "0.6");
  te_number_x(false, &scope!(), r#"modulo(1, 3e-35)"#, "1E-35");
  te_number_x(false, &scope!(), r#"modulo(1 - 2e-34, 1/3)"#, "0.3333333333333333333333333333333332");
  te_number_x(false, &scope!(), r#"modulo(1e6000, 1e-6000)"#, "0");
  te_number_x(false, &scope!(), r#"modulo(1e6000, -7)"#, "-6");
  te_number_x(false, &scope!(), r#"modulo(-1e-6176, 1e6000)"#, "1E+6000");
  te_number_x(false, &scope!(), r#"modulo(1e-6176, -1e-6175)"#, "-9E-6176");
}
