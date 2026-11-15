use super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_number(false, &scope!(), r#" 1/1 "#, 1, 0);
}

#[test]
fn _0002() {
  te_number(false, &scope!(), r#" 1 / 2 "#, 5, 1);
}

#[test]
fn _0003() {
  te_number(false, &scope!(), r#" 5 / 2 / 4 "#, 625, 3);
}

#[test]
fn _0004() {
  te_number(false, &scope!(), r#" 10 / 2 / 5"#, 1, 0);
}

#[test]
fn _0005() {
  te_number(false, &scope!(), r#"( 1 / 2 ) / ( 12 / 6 )"#, 25, 2);
}

#[test]
fn _0006() {
  te_number(false, &scope!(), r#"( ( ( 6 / 3 ) ) )"#, 2, 0);
}

#[test]
fn _0007() {
  te_number_x(false, &scope!(), r#"1/3"#, r#"0.3333333333333333333333333333333333"#);
}

#[test]
fn _0008() {
  te_number(false, &scope!(), r#"1.01/2"#, 505, 3);
}

#[test]
fn _0009() {
  te_null(false, &scope!(), r#"0.0 / 0.0"#, r#"[division] division by zero"#);
}

#[test]
fn _0010() {
  te_null(false, &scope!(), r#" 0.0 / "a" "#, r#"[division] incompatible types: 0 / "a""#);
}

#[test]
fn _0011() {
  te_days_and_time_duration_x(false, &scope!(), r#" @"P10D" / 2 "#, r#"P5D"#);
}

#[test]
fn _0012() {
  te_null(
    false,
    &scope!(),
    r#" @"P10D" / 0.00000000000000000000000000000000000000000000000001 "#,
    // a number longer than 40 characters is quoted in scientific notation
    r#"[division] error: P10D / 1E-50"#,
  );
}

#[test]
fn _0013() {
  te_null(false, &scope!(), r#" @"P10D" / 0 "#, r#"[division] division by zero"#);
}

#[test]
fn _0014() {
  te_number(false, &scope!(), r#" @"P10D" / @"P5D" "#, 2, 0);
}

#[test]
fn _0015() {
  te_null(false, &scope!(), r#" @"P10D" / @"P0D" "#, r#"[division] division by zero"#);
}

#[test]
fn _0016() {
  te_years_and_months_duration_x(false, &scope!(), r#" @"P10Y" / 2 "#, r#"P5Y"#);
}

#[test]
fn _0017() {
  te_null(
    false,
    &scope!(),
    r#" @"P10Y" / 0.00000000000000000000000000000000000000000000000001 "#,
    // a number longer than 40 characters is quoted in scientific notation
    r#"[division] error: P10Y / 1E-50"#,
  );
}

#[test]
fn _0018() {
  te_number(false, &scope!(), r#" @"P10Y" / @"P5Y" "#, 2, 0);
}

#[test]
fn _0019() {
  te_null(false, &scope!(), r#" @"P10Y" / 0 "#, r#"[division] division by zero"#);
}

#[test]
fn _0020() {
  te_null(false, &scope!(), r#" @"P10Y" / @"P0Y" "#, r#"[division] division by zero"#);
}

#[test]
fn _0021() {
  te_null(false, &scope!(), r#" @"P10D" / "a" "#, r#"[division] incompatible types: P10D / "a""#);
}

#[test]
fn _0022() {
  te_null(false, &scope!(), r#" @"P10Y" / "a" "#, r#"[division] incompatible types: P10Y / "a""#);
}

#[test]
fn _0023() {
  te_null(false, &scope!(), r#" "a" / 2.21 "#, r#"[division] incompatible types: "a" / 2.21"#);
}

#[test]
fn _0024() {
  // A quotient of -2^63 months or nanoseconds is out of the range of a duration.
  let scope = &scope!();
  te_null(
    false,
    scope,
    r#"duration("-P4611686018427387904M") / 0.5"#,
    "[division] error: -P384307168202282325Y4M / 0.5",
  );
  te_null(
    false,
    scope,
    r#"duration("-PT4611686018.427387904S") / 0.5"#,
    "[division] error: -P53375DT23H53M38.427387904S / 0.5",
  );
  te_years_and_months_duration_x(false, scope, r#"duration("-P4611686018427387903M") / 0.5"#, "-P768614336404564650Y6M");
}

/// DMN 1.6 Table 60: a years and months duration divided by a non-zero number is
/// `valueymd⁻¹(valueymd(e1) / value(e2))`. A quotient with a fraction of a month is truncated
/// toward zero, as the DMN TCK expects (`@"P10Y11M" / 2.5` is `P4Y4M`), and the evaluation warns
/// that the fraction was dropped; a whole number of months does not warn.
#[test]
fn a_years_and_months_quotient_warns_when_it_drops_a_fraction_of_a_month() {
  use super::multiplication::months_warning;
  te_warnings(r#"@"P10Y11M" / 2.5"#, "P4Y4M", &[&months_warning("P10Y11M / 2.5", "0.4")]);
  te_warnings(r#"@"P10Y11M" / -2.5"#, "-P4Y4M", &[&months_warning("P10Y11M / -2.5", "-0.4")]);
  te_warnings(r#"duration("P1Y") / 5"#, "P2M", &[&months_warning("P1Y / 5", "0.4")]);
  // whole numbers of months
  te_warnings(r#"duration("P1Y") / 4"#, "P3M", &[]);
  te_warnings(r#"duration("P1Y") / 0.5"#, "P2Y", &[]);
  te_warnings(r#"duration("-P1Y") / 3"#, "-P4M", &[]);
}

#[test]
fn a_null_operand_of_a_division_is_an_error() {
  // DMN 1.6, before Table 59: "if input values are not of the listed types, the result is null";
  // the official TCK expects an error result for a null operand on either side
  // (compliance-level-3/0100-arithmetic, error_when_divide_lhs_null_by_rhs_number and the others)
  te_null_operands("division", "/");
}
