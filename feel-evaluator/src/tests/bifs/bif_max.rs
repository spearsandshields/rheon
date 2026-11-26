use super::super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_null(false, &scope!(), "max()", r#"expected 1+ parameters, actual number of parameters is 0"#);
}

#[test]
fn _0002() {
  te_null(false, &scope!(), "max([])", r#""#);
}

#[test]
fn _0003() {
  te_null(false, &scope!(), "max(l:[])", r#"expected the parameters of max(list): Any, found the parameter name 'l'"#);
}

#[test]
fn _0004() {
  te_null(
    false,
    &scope!(),
    "max(l:[1,2,3])",
    r#"expected the parameters of max(list): Any, found the parameter name 'l'"#,
  );
}

#[test]
fn _0005() {
  te_number(false, &scope!(), "max([2021])", 2021, 0);
}

#[test]
fn _0006() {
  te_null(false, &scope!(), "max([])", r#""#);
}

#[test]
fn _0007() {
  te_number(false, &scope!(), "max(2021)", 2021, 0);
}

#[test]
fn _0008() {
  te_number(false, &scope!(), "max([1,2,3])", 3, 0);
}

#[test]
fn _0009() {
  te_number(false, &scope!(), "max(list: [2,3,1])", 3, 0);
}

#[test]
fn _0010() {
  te_number(false, &scope!(), "max(1,2,3)", 3, 0);
}

#[test]
fn _0011() {
  te_number(false, &scope!(), "max([8,4,2.89,3,8.0001,2.89,5])", 80001, 4);
}

#[test]
fn _0012() {
  te_number(false, &scope!(), "max(8,4,2.89,3,8.0001,2.89,5)", 80001, 4);
}

#[test]
fn _0013() {
  te_number(false, &scope!(), "max([2837465.9584,-39408573.456749])", 28374659584, 4);
}

#[test]
fn _0014() {
  te_number(false, &scope!(), "max(2837465.9584,-39408573.456749)", 28374659584, 4);
}

#[test]
fn _0015() {
  te_string(false, &scope!(), r#"max(["a","b","c"])"#, r#"c"#);
}

#[test]
fn _0016() {
  te_string(false, &scope!(), r#"max("a","b","c")"#, r#"c"#);
}

#[test]
fn _0017() {
  te_string(false, &scope!(), r#"max(["John","Johnny"])"#, r#"Johnny"#);
}

#[test]
fn _0018() {
  te_string(false, &scope!(), r#"max("John","Johnny")"#, r#"Johnny"#);
}

#[test]
fn _0019() {
  te_null(
    false,
    &scope!(),
    r#"max(1,2,3,"a")"#,
    r#"[core::max] invalid argument type, expected number, actual type is string"#,
  );
}

#[test]
fn _0020() {
  te_null(
    false,
    &scope!(),
    r#"max("a","b","c",true)"#,
    r#"[core::max] invalid argument type, expected string, actual type is boolean"#,
  );
}

#[test]
fn _0021() {
  te_null(
    false,
    &scope!(),
    r#"max(true, "a","b","c",1,2)"#,
    r#"[core::max] invalid argument type, expected number, string, date, time, date and time, or duration, actual type is boolean"#,
  );
}

#[test]
fn _0022() {
  // over FEEL's order: dates, times, date and times and durations too (10.3.4.4)
  let scope = &scope!();
  te_date(false, scope, r#"max([date("2020-01-01"), date("2021-01-01")])"#, 2021, 1, 1);
  te_be_value(false, scope, r#"max([duration("P1D"), duration("PT1H")])"#, r#"duration("P1D")"#);
  te_be_value(false, scope, r#"max(["a", "b"])"#, r#""b""#);
  // a null item is outside the domain wherever it is
  te_null(false, scope, r#"max([1, null])"#, "[core::max] invalid argument type, expected number, actual type is Null");
  te_null(
    false,
    scope,
    r#"max([null, 1])"#,
    "[core::max] invalid argument type, expected number, string, date, time, date and time, or duration, actual type is Null",
  );
  // items of two types
  te_null(
    false,
    scope,
    r#"max([1, "a"])"#,
    "[core::max] invalid argument type, expected number, actual type is string",
  );
  // a time without an offset takes the offset of the other (Table 55)
  te_string(false, scope, r#"string(max([time("10:00:00"), time("11:00:00+01:00")]))"#, "11:00:00+01:00");
}
