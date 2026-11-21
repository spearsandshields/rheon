use super::super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_null(false, &scope!(), "min()", r#"expected 1+ parameters, actual number of parameters is 0"#);
}

#[test]
fn _0002() {
  te_null(false, &scope!(), "min([])", r#""#);
}

#[test]
fn _0003() {
  te_null(false, &scope!(), "min(l:[])", r#"expected the parameters of min(list): Any, found the parameter name 'l'"#);
}

#[test]
fn _0004() {
  te_null(false, &scope!(), "min(list:[])", r#""#);
}

#[test]
fn _0005() {
  te_null(
    false,
    &scope!(),
    "min(l:[1,2,3])",
    r#"expected the parameters of min(list): Any, found the parameter name 'l'"#,
  );
}

#[test]
fn _0006() {
  te_number(false, &scope!(), "min(list:[1,2,3])", 1, 0);
}

#[test]
fn _0007() {
  te_number(false, &scope!(), "min([2021])", 2021, 0);
}

#[test]
fn _0008() {
  te_number(false, &scope!(), "min(2021)", 2021, 0);
}

#[test]
fn _0009() {
  te_number(false, &scope!(), "min([1,2,3])", 1, 0);
}

#[test]
fn _0010() {
  te_number(false, &scope!(), "min(1,2,3)", 1, 0);
}

#[test]
fn _0011() {
  te_number(false, &scope!(), "min([8,4,2.89,3,2.89,5])", 289, 2);
}

#[test]
fn _0012() {
  te_number(false, &scope!(), "min(8,4,2.89,3,2.89,5)", 289, 2);
}

#[test]
fn _0013() {
  te_number(false, &scope!(), "min([2837465.9584,-39408573.456749])", -39408573456749, 6);
}

#[test]
fn _0014() {
  te_number(false, &scope!(), "min(2837465.9584,-39408573.456749)", -39408573456749, 6);
}

#[test]
fn _0015() {
  te_string(false, &scope!(), r#"min(["a","b","c"])"#, "a");
}

#[test]
fn _0016() {
  te_string(false, &scope!(), r#"min("b","a","c")"#, "a");
}

#[test]
fn _0017() {
  te_string(false, &scope!(), r#"min(["Johnny","John"])"#, "John");
}

#[test]
fn _0018() {
  te_string(false, &scope!(), r#"min("Johnny","John")"#, "John");
}

#[test]
fn _0019() {
  te_null(
    false,
    &scope!(),
    r#"min(1,2,3,"a")"#,
    r#"[core::min] invalid argument type, expected number, actual type is string"#,
  );
}

#[test]
fn _0020() {
  te_null(
    false,
    &scope!(),
    r#"min("a","b","c",true)"#,
    r#"[core::min] invalid argument type, expected string, actual type is boolean"#,
  );
}

#[test]
fn _0021() {
  te_null(
    false,
    &scope!(),
    r#"min(true, "a","b","c",1,2)"#,
    r#"[core::min] invalid argument type, expected number, string, date, time, date and time, or duration, actual type is boolean"#,
  );
}

#[test]
fn _0022() {
  // over FEEL's order: dates, times, date and times and durations too (10.3.4.4)
  let scope = &scope!();
  te_date(false, scope, r#"min([date("2020-01-01"), date("2021-01-01")])"#, 2020, 1, 1);
  te_be_value(false, scope, r#"min([duration("P1D"), duration("PT1H")])"#, r#"duration("PT1H")"#);
  te_be_value(false, scope, r#"min(["a", "b"])"#, r#""a""#);
  // a null item is outside the domain wherever it is
  te_null(false, scope, r#"min([1, null])"#, "[core::min] invalid argument type, expected number, actual type is Null");
  te_null(
    false,
    scope,
    r#"min([null, 1])"#,
    "[core::min] invalid argument type, expected number, string, date, time, date and time, or duration, actual type is Null",
  );
  // items of two types
  te_null(
    false,
    scope,
    r#"min([1, "a"])"#,
    "[core::min] invalid argument type, expected number, actual type is string",
  );
  // a time without an offset takes the offset of the other (Table 55)
  te_string(false, scope, r#"string(min([time("10:00:00"), time("11:00:00+01:00")]))"#, "10:00:00");
}
