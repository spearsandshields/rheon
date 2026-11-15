use super::super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_years_and_months_duration(false, &scope!(), r#"duration("P1Y")"#, 1, 0);
}

#[test]
fn _0002() {
  te_years_and_months_duration(false, &scope!(), r#"duration("P4Y")"#, 4, 0);
}

#[test]
fn _0003() {
  te_years_and_months_duration(false, &scope!(), r#"duration("-P999999999Y")"#, -999_999_999, 0);
}

#[test]
fn _0004() {
  te_years_and_months_duration(false, &scope!(), r#"duration("P999999999Y")"#, 999_999_999, 0);
}

#[test]
fn _0005() {
  te_years_and_months_duration(false, &scope!(), r#"duration(from:"P26M")"#, 2, 2);
}

#[test]
fn _0006() {
  te_days_and_time_duration(false, &scope!(), r#"duration("P1D")"#, false, SECONDS_IN_DAY, 0);
}

#[test]
fn _0007() {
  te_days_and_time_duration(false, &scope!(), r#"duration("P4D")"#, false, 4 * SECONDS_IN_DAY, 0);
}

#[test]
fn _0008() {
  te_days_and_time_duration(false, &scope!(), r#"duration("PT2H")"#, false, 2 * SECONDS_IN_HOUR, 0);
}

#[test]
fn _0009() {
  te_days_and_time_duration(false, &scope!(), r#"duration("PT3M")"#, false, 3 * SECONDS_IN_MINUTE, 0);
}

#[test]
fn _0010() {
  te_days_and_time_duration(false, &scope!(), r#"duration("PT4S")"#, false, 4, 0);
}

#[test]
fn _0011() {
  te_days_and_time_duration(false, &scope!(), r#"duration(from:"PT24H")"#, false, SECONDS_IN_DAY, 0);
}

#[test]
fn _0012() {
  te_null(
    false,
    &scope!(),
    "duration(null)",
    "[core::duration] invalid argument type, expected string, actual type is Null",
  );
}

#[test]
fn _0013() {
  te_null(false, &scope!(), "duration()", "expected 1 parameters, actual number of parameters is 0");
}

#[test]
fn _0014() {
  te_null(
    false,
    &scope!(),
    "duration([])",
    "[core::duration] invalid argument type, expected string, actual type is list<Null>",
  );
}

#[test]
fn _0015() {
  te_null(false, &scope!(), r#"duration("")"#, "duration: invalid duration literal: ''");
}

#[test]
fn _0016() {
  te_null(
    false,
    &scope!(),
    "duration(2017)",
    "[core::duration] invalid argument type, expected string, actual type is number",
  );
}

#[test]
fn _0017() {
  te_null(
    false,
    &scope!(),
    r#"duration("2012T-12-2511:00:00Z")"#,
    "duration: invalid duration literal: '2012T-12-2511:00:00Z'",
  );
}

#[test]
fn _0018() {
  te_null(false, &scope!(), r#"duration("P")"#, "duration: invalid duration literal: 'P'");
}

#[test]
fn _0019() {
  te_null(false, &scope!(), r#"duration("P0")"#, "duration: invalid duration literal: 'P0'");
}

#[test]
fn _0020() {
  te_null(false, &scope!(), r#"duration("1Y")"#, "duration: invalid duration literal: '1Y'");
}

#[test]
fn _0021() {
  te_null(false, &scope!(), r#"duration("1D")"#, "duration: invalid duration literal: '1D'");
}

#[test]
fn _0022() {
  te_null(false, &scope!(), r#"duration("P1H")"#, "duration: invalid duration literal: 'P1H'");
}

#[test]
fn _0023() {
  te_null(false, &scope!(), r#"duration("P1S")"#, "duration: invalid duration literal: 'P1S'");
}

#[test]
fn _0024() {
  te_null(false, &scope!(), r#"duration(f: "PT24H")"#, r#"parameter 'from' not found"#);
}

#[test]
fn _0025() {
  // A years and months literal is valid only when every component it contains fits.
  let scope = &scope!();
  te_null(
    false,
    scope,
    r#"duration("P768614336404564650Y8M")"#,
    "duration: invalid duration literal: 'P768614336404564650Y8M'",
  );
  te_null(
    false,
    scope,
    r#"duration("-P768614336404564650Y8M")"#,
    "duration: invalid duration literal: '-P768614336404564650Y8M'",
  );
  te_null(
    false,
    scope,
    r#"duration("P99999999999999999999Y3M")"#,
    "duration: invalid duration literal: 'P99999999999999999999Y3M'",
  );
  te_null(
    false,
    scope,
    r#"duration("P1Y99999999999999999999M")"#,
    "duration: invalid duration literal: 'P1Y99999999999999999999M'",
  );
  te_years_and_months_duration_x(false, scope, r#"duration("P768614336404564650Y7M")"#, "P768614336404564650Y7M");
  te_years_and_months_duration_x(false, scope, r#"duration("-P768614336404564650Y7M")"#, "-P768614336404564650Y7M");
}

#[test]
fn fractional_seconds_are_exact() {
  te_string(false, &scope!(), r#"string(duration("PT0.000015839S"))"#, "PT0.000015839S");
  te_string(false, &scope!(), r#"string(duration("PT1.99999999999999999S"))"#, "PT1.999999999S");
  te_days_and_time_duration(false, &scope!(), r#"duration("PT999.999999999999S")"#, false, 999, 999_999_999);
}

#[test]
fn fraction_digits_beyond_nanoseconds_warn() {
  te_warnings(
    r#"duration("PT999.999999999999S")"#,
    "PT16M39.999999999S",
    &[&truncation_warning("PT999.999999999999S", "999")],
  );
  te_warnings(r#"duration("-P1DT0.0000000001S")"#, "-P1D", &[&truncation_warning("-P1DT0.0000000001S", "1")]);
  te_warnings(r#"duration("PT1.0000000000S")"#, "PT1S", &[]);
}
