use super::super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_number(false, &scope!(), r#"number("1",",",".")"#, 1, 0);
}

#[test]
fn _0002() {
  te_number(false, &scope!(), r#"number(from: "1", grouping separator: ",", decimal separator: ".")"#, 1, 0);
}

#[test]
fn _0003() {
  te_number(false, &scope!(), r#"number("1,000.21",",",".")"#, 100021, 2);
}

#[test]
fn _0004() {
  te_number(false, &scope!(), r#"number("1 000.21"," ",".")"#, 100021, 2);
}

#[test]
fn _0005() {
  te_number(false, &scope!(), r#"number("1.000,21",".",",")"#, 100021, 2);
}

#[test]
fn _0006() {
  te_number(false, &scope!(), r#"number("12345",null,null)"#, 12345, 0);
}

#[test]
fn _0007() {
  te_number(false, &scope!(), r#"number("12,345",",",null)"#, 12345, 0);
}

#[test]
fn _0008() {
  te_number(false, &scope!(), r#"number("123,45",null,",")"#, 12345, 2);
}

#[test]
fn _0009() {
  te_null(
    false,
    &scope!(),
    r#"number("1,000.21",".",".")"#,
    r#"[core::number] decimal separator must be different from grouping separator"#,
  );
}

#[test]
fn _0010() {
  te_null(
    false,
    &scope!(),
    r#"number("1$000.21","$",".")"#,
    r#"[core::number] grouping separator must be space, period, comma or null"#,
  );
}

#[test]
fn _0011() {
  te_null(
    false,
    &scope!(),
    r#"number("1,000$21",",","$")"#,
    r#"[core::number] decimal separator must be period, comma or null"#,
  );
}

#[test]
fn _0012() {
  te_null(
    false,
    &scope!(),
    r#"number("123a56",null,null)"#,
    r#"[core::number] not a FEEL numeric literal: 123a56"#,
  );
}

#[test]
fn _0013() {
  te_null(
    false,
    &scope!(),
    r#"number("1,000.21",2,".")"#,
    r#"[core::number] grouping separator must be space, period, comma or null"#,
  );
}

#[test]
fn _0014() {
  te_null(
    false,
    &scope!(),
    r#"number("1,000.21",",",true)"#,
    r#"[core::number] decimal separator must be period, comma or null"#,
  );
}

#[test]
fn _0015() {
  te_null(
    false,
    &scope!(),
    r#"number(1000,null,null)"#,
    r#"[core::number] invalid argument type, expected string, actual type is number"#,
  );
}

#[test]
fn _0016() {
  te_null(false, &scope!(), r#"number()"#, r#"expected 1,3 parameters, actual number of parameters is 0"#);
}

#[test]
fn _0017() {
  // DMN 1.6 (DMN16-105): number(from) accepts a number (was an arity error before 1.6)
  te_number(false, &scope!(), r#"number(1000)"#, 1000, 0);
}

#[test]
fn _0018() {
  te_null(false, &scope!(), r#"number(1000,",")"#, r#"expected 1,3 parameters, actual number of parameters is 2"#);
}

#[test]
fn _0019() {
  te_null(
    false,
    &scope!(),
    r#"number(1000,",",".",",")"#,
    r#"expected 1,3 parameters, actual number of parameters is 4"#,
  );
}

#[test]
fn _0020() {
  te_null(
    false,
    &scope!(),
    r#"number(f: "1", grouping separator: ",", decimal separator: ".")"#,
    r#"parameter 'from' not found"#,
  );
}

#[test]
fn _0021() {
  te_null(
    false,
    &scope!(),
    r#"number(from: "1", grouping sep: ",", decimal separator: ".")"#,
    r#"parameter 'grouping separator' not found"#,
  );
}

#[test]
fn _0022() {
  te_null(
    false,
    &scope!(),
    r#"number(from: "1", grouping separator: ",", decimal sep: ".")"#,
    r#"parameter 'decimal separator' not found"#,
  );
}

#[test]
fn dmn_1_6_single_argument() {
  // DMN 1.6 (DMN16-208, DMN16-105): number(from) for a string, and for a number.
  te_number(false, &scope!(), r#"number("1.1")"#, 11, 1);
  te_number(false, &scope!(), r#"number(from: "1.1")"#, 11, 1);
  te_number(false, &scope!(), "number(5)", 5, 0);
  te_null(false, &scope!(), "number(null)", "");
}

#[test]
fn numeric_literal_grammar() {
  // the text must be a FEEL numeric literal (grammar rule 35) after separator handling
  te_null(false, &scope!(), r#"number("+5")"#, "[core::number] not a FEEL numeric literal: +5");
  te_null(false, &scope!(), r#"number("1.")"#, "[core::number] not a FEEL numeric literal: 1.");
  te_number(false, &scope!(), r#"number(".5")"#, 5, 1);
  te_number(false, &scope!(), r#"number("-1.2e3")"#, -1200, 0);
  te_number(false, &scope!(), r#"number("1.000.000,01", ".", ",")"#, 100000001, 2);
}
