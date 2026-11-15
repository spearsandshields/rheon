use super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_number(false, &scope!(), "1*1", 1, 0);
}

#[test]
fn _0002() {
  te_number(false, &scope!(), " 1 * 2 ", 2, 0);
}

#[test]
fn _0003() {
  te_number(false, &scope!(), " 5 *2 *3", 30, 0);
}

#[test]
fn _0004() {
  te_number(false, &scope!(), "10*2*5", 100, 0);
}

#[test]
fn _0005() {
  te_number(false, &scope!(), "( 1 * 2 ) * ( 3 * 4 )", 24, 0);
}

#[test]
fn _0006() {
  te_number(false, &scope!(), "( ( ( 4 * 3 ) ) )", 12, 0);
}

#[test]
fn _0007() {
  te_number(false, &scope!(), "(3*2)+(4*5)", 26, 0);
}

#[test]
fn _0008() {
  te_number(false, &scope!(), "3*2+4*5", 26, 0);
}

#[test]
fn _0009() {
  te_number(false, &scope!(), "3*(2+4)*5", 90, 0);
}

#[test]
fn _0010() {
  te_number(false, &scope!(), ".10 * 30.00", 3, 0);
}

#[test]
fn _0011() {
  te_number(false, &scope!(), "1.0*10**3", 1000, 0);
}

#[test]
fn _0012() {
  let scope = &te_scope("{Monthly Salary:10000}");
  te_number(false, scope, "12 * Monthly Salary", 120000, 0);
}

#[test]
fn _0013() {
  te_days_and_time_duration_x(false, &scope!(), r#" 1.5 * @"P4DT1H" "#, "P6DT1H30M");
}

#[test]
fn _0014() {
  te_days_and_time_duration_x(false, &scope!(), r#" @"P4DT1H" * 1.5 "#, "P6DT1H30M");
}

#[test]
fn _0015() {
  te_years_and_months_duration_x(false, &scope!(), r#" 10 * @"P1Y" "#, "P10Y");
}

#[test]
fn _0016() {
  te_years_and_months_duration_x(false, &scope!(), r#" @"P1Y" * 10 "#, "P10Y");
}

#[test]
fn _0017() {
  te_null(false, &scope!(), r#" 1 * "a" "#, r#"[multiplication] incompatible types: 1 * "a""#);
}

#[test]
fn _0018() {
  te_null(false, &scope!(), r#" @"P4DT1H" * @"P2Y" "#, r#"[multiplication] incompatible types: P4DT1H * P2Y"#);
}

#[test]
fn _0019() {
  te_null(false, &scope!(), r#" @"P2Y" * @"P4DT1H" "#, r#"[multiplication] incompatible types: P2Y * P4DT1H"#);
}

#[test]
fn _0020() {
  te_null(
    false,
    &scope!(),
    r#" 999999999999999999999 * @"P1D" "#,
    r#"multiplication result is out of range of days and time duration"#,
  );
}

#[test]
fn _0021() {
  te_null(
    false,
    &scope!(),
    r#" 999999999999999999999 * @"P1Y" "#,
    r#"multiplication result is out of range of years and months duration"#,
  );
}

#[test]
fn _0022() {
  te_null(
    false,
    &scope!(),
    r#" @"P1D" * 999999999999999999999 "#,
    r#"multiplication result is out of range of days and time duration"#,
  );
}

#[test]
fn _0023() {
  te_null(
    false,
    &scope!(),
    r#" @"P1Y" * 999999999999999999999 "#,
    r#"multiplication result is out of range of years and months duration"#,
  );
}

#[test]
fn _0024() {
  te_null(false, &scope!(), r#" null * @"P1Y" "#, r#""#);
}

#[test]
fn _0025() {
  te_null(false, &scope!(), r#" "a" * 2 "#, r#"unexpected value type in multiplication: string"#);
}

#[test]
fn _0026() {
  // A duration lies in -(2^63 - 1)..=2^63 - 1 months or nanoseconds, so a product of -2^63 is out of range.
  let scope = &scope!();
  te_null(
    false,
    scope,
    r#"duration("P1M") * -9223372036854775808"#,
    "multiplication result is out of range of years and months duration",
  );
  te_null(
    false,
    scope,
    r#"-9223372036854775808 * duration("P1M")"#,
    "multiplication result is out of range of years and months duration",
  );
  te_null(
    false,
    scope,
    r#"duration("PT0.000000001S") * -9223372036854775808"#,
    "multiplication result is out of range of days and time duration",
  );
  te_null(
    false,
    scope,
    r#"-9223372036854775808 * duration("PT0.000000001S")"#,
    "multiplication result is out of range of days and time duration",
  );
  te_years_and_months_duration_x(false, scope, r#"duration("P1M") * -9223372036854775807"#, "-P768614336404564650Y7M");
  te_days_and_time_duration_x(false, scope, r#"duration("PT0.000000001S") * -9223372036854775807"#, "-P106751DT23H47M16.854775807S");
}

/// The warning that the product or quotient `operation` of a years and months duration lost the
/// fraction of a month `dropped`.
pub(super) fn months_warning(operation: &str, dropped: &str) -> String {
  format!("fractional months of '{operation}' truncated to whole months; dropped fraction: '{dropped}'")
}

/// DMN 1.6 Table 60: a years and months duration times a number is
/// `valueymd⁻¹(valueymd(e1) * value(e2))`, and `valueymd⁻¹` (§10.3.2.3.8) maps a number of
/// months to a duration. A product with a fraction of a month is truncated toward zero, as the
/// DMN TCK expects (`-2.5 * @"P1Y11M"` is `-P4Y9M`), and the evaluation warns that the fraction
/// was dropped; a whole number of months does not warn.
#[test]
fn a_years_and_months_product_warns_when_it_drops_a_fraction_of_a_month() {
  te_warnings(r#"-2.5 * @"P1Y11M""#, "-P4Y9M", &[&months_warning("-2.5 * P1Y11M", "-0.5")]);
  te_warnings(r#"@"P1Y11M" * -2.5"#, "-P4Y9M", &[&months_warning("P1Y11M * -2.5", "-0.5")]);
  te_warnings(r#"duration("P1M") * 1.5"#, "P1M", &[&months_warning("P1M * 1.5", "0.5")]);
  te_warnings(r#"1.5 * duration("P1M")"#, "P1M", &[&months_warning("1.5 * P1M", "0.5")]);
  te_warnings(r#"duration("P1M") * 0.1"#, "P0M", &[&months_warning("P1M * 0.1", "0.1")]);
  // whole numbers of months
  te_warnings(r#"duration("P1Y") * 0.5"#, "P6M", &[]);
  te_warnings(r#"0.25 * duration("P1Y")"#, "P3M", &[]);
  te_warnings(r#"duration("P2M") * 1.5"#, "P3M", &[]);
  te_warnings(r#"duration("-P2M") * 1.5"#, "-P3M", &[]);
  te_warnings(r#"duration("P1M") * 0"#, "P0M", &[]);
  // a days and time duration keeps its truncation of a fraction of a nanosecond
  te_warnings(r#"duration("PT0.000000001S") * 1.5"#, "PT0.000000001S", &[]);
}
