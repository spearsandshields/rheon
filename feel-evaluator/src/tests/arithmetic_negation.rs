use super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_number(false, &scope!(), "-0", 0, 0);
}

#[test]
fn _0002() {
  te_number(false, &scope!(), "-0", -0, 0);
}

#[test]
fn _0003() {
  te_number(false, &scope!(), "0", 0, 0);
}

#[test]
fn _0004() {
  te_number(false, &scope!(), "--0", 0, 0);
}

#[test]
fn _0005() {
  te_number(false, &scope!(), "-1", -1, 0);
}

#[test]
fn _0006() {
  te_number(false, &scope!(), "--1", 1, 0);
}

#[test]
fn _0007() {
  te_number(false, &scope!(), " -10.2 ", -102, 1);
}

#[test]
fn _0008() {
  te_number(false, &scope!(), " - 10 ", -10, 0);
}

#[test]
fn _0009() {
  te_number(false, &scope!(), "(-20)", -20, 0);
}

#[test]
fn _0010() {
  te_number(false, &scope!(), " ( -21 ) ", -21, 0);
}

#[test]
fn _0011() {
  te_number(false, &scope!(), " ( - 22 ) ", -22, 0);
}

#[test]
fn _0012() {
  te_number(false, &scope!(), " ((( - 23 ))) ", -23, 0);
}

#[test]
fn _0013() {
  te_number(false, &scope!(), " - - 24 ", 24, 0);
}

#[test]
fn _0014() {
  te_number(false, &scope!(), " - ( - 25 ) ", 25, 0);
}

#[test]
fn _0015() {
  te_days_and_time_duration_x(false, &scope!(), r#"duration("-PT1H")"#, "-PT1H")
}

#[test]
fn _0016() {
  te_days_and_time_duration_x(false, &scope!(), r#" - @"PT1H" "#, "-PT1H")
}

#[test]
fn _0017() {
  te_years_and_months_duration_x(false, &scope!(), r#" - @"P1Y" "#, "-P1Y")
}

#[test]
fn _0018() {
  te_null(false, &scope!(), r#" -"a" "#, "unexpected type in arithmetic negation: string")
}

#[test]
fn _0019() {
  // Negation is total: a duration lies in -(2^63 - 1)..=2^63 - 1 months or nanoseconds.
  let scope = &scope!();
  te_years_and_months_duration_x(false, scope, r#"-duration("-P9223372036854775807M")"#, "P768614336404564650Y7M");
  te_years_and_months_duration_x(false, scope, r#"-duration("P9223372036854775807M")"#, "-P768614336404564650Y7M");
  te_days_and_time_duration_x(false, scope, r#"-duration("-PT9223372036.854775807S")"#, "P106751DT23H47M16.854775807S");
  te_null(
    false,
    scope,
    r#"-(duration("P1M") * -9223372036854775808)"#,
    "[negation] incompatible types: -null(multiplication result is out of range of years and months duration)",
  );
}

#[test]
fn a_null_operand_of_a_negation_is_an_error() {
  // DMN 1.6 Table 63: -e is "e*-1", so its null operand is an error as the multiplication's is
  let reason = "[negation] incompatible types: -null";
  te_errors("-null", &format!("null({reason})"), &[reason]);
  let division_by_zero = "[division] division by zero";
  let reason = format!("[negation] incompatible types: -null({division_by_zero})");
  te_errors("-(1/0)", &format!("null({reason})"), &[division_by_zero, &reason]);
}
