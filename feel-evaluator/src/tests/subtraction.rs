use super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  let scope = &te_scope(r#"{}"#);
  te_number(false, scope, "1-1", 0, 0);
}

#[test]
fn _0002() {
  let scope = &te_scope(r#"{}"#);
  te_number(false, scope, " 1 - 2 ", -1, 0);
}

#[test]
fn _0003() {
  let scope = &te_scope(r#"{}"#);
  te_number(false, scope, " 5 -2 -1 ", 2, 0);
}

#[test]
fn _0004() {
  let scope = &te_scope(r#"{}"#);
  te_number(false, scope, "1000-200-2", 798, 0);
}

#[test]
fn _0005() {
  let scope = &te_scope(r#"{}"#);
  te_number(false, scope, "( 1 - 2 ) - ( 3 - 4 )", 0, 0);
}

#[test]
fn _0006() {
  let scope = &te_scope(r#"{}"#);
  te_number(false, scope, "( ( ( 4 - 3 ) ) )", 1, 0);
}

#[test]
fn _0007() {
  let scope = &te_scope("{ a: 11.2, b: 0.2}");
  te_number(false, scope, "a-b", 11, 0);
}

#[test]
fn _0008() {
  let scope = &te_scope("{ a: 11.2, b: 0.2, c: 5.351 }");
  te_number(false, scope, "a-b-c", 5649, 3);
}

#[test]
fn _0009() {
  let scope = &te_scope("{ a: 11.2, b: 0.2, c: 5.351 }");
  te_number(false, scope, " a  \n -  b \n  -  c", 5649, 3);
}

#[test]
fn _0010() {
  te_time(false, &scope!(), r#"time("12:34:56") - duration("P0D")"#, FeelTime::local_opt(12, 34, 56, 0).unwrap());
}

#[test]
fn _0011() {
  te_time(false, &scope!(), r#"time("12:34:56") - duration("P1D")"#, FeelTime::local_opt(12, 34, 56, 0).unwrap());
}

#[test]
fn _0012() {
  te_time(false, &scope!(), r#"time("12:34:56") - duration("P11D")"#, FeelTime::local_opt(12, 34, 56, 0).unwrap());
}

#[test]
fn _0013() {
  te_time(false, &scope!(), r#"time("12:34:56") - duration("PT10S")"#, FeelTime::local_opt(12, 34, 46, 0).unwrap());
}

#[test]
fn _0014() {
  te_time(false, &scope!(), r#"time("12:34:23") - duration("PT25S")"#, FeelTime::local_opt(12, 33, 58, 0).unwrap());
}

#[test]
fn _0015() {
  te_time(false, &scope!(), r#"time("12:34:23") - duration("PT40M")"#, FeelTime::local_opt(11, 54, 23, 0).unwrap());
}

#[test]
fn _0016() {
  te_date(false, &scope!(), r#" @"2020-03-01" - @"PT24H" "#, 2020, 2, 29);
}

#[test]
fn _0017() {
  te_date(false, &scope!(), r#" @"2021-01-02" - @"PT25H" "#, 2020, 12, 31);
}

#[test]
fn _0018() {
  te_days_and_time_duration_x(false, &scope!(), r#" @"2023-02-06" - @"2021-02-06" "#, "P730D");
}

#[test]
fn _0019() {
  te_days_and_time_duration_x(false, &scope!(), r#" @"2023-02-06" - @"2021-02-06T12:13:18Z" "#, "P729DT11H46M42S");
}

#[test]
fn _0020() {
  te_days_and_time_duration_x(false, &scope!(), r#" @"262142-12-31T00:00:00" - @"262142-12-31T00:00:00" "#, "PT0S");
}

#[test]
fn _0021() {
  te_date(false, &scope!(), r#" @"2023-02-06" - @"P1Y1M" "#, 2022, 1, 6);
}

#[test]
fn _0022() {
  te_be_value(false, &scope!(), r#" @"2023-02-06" - @"P999999Y" "#, r#"date("-997976-02-06")"#);
}

#[test]
fn _0023() {
  te_null(
    false,
    &scope!(),
    r#" @"2023-02-06" - @"P999999999999999Y" "#,
    "[subtraction] incompatible types: 2023-02-06 - P999999999999999Y",
  );
}

#[test]
fn _0024() {
  te_date(false, &scope!(), r#" @"2021-02-06" + (-@"P2Y") "#, 2019, 2, 6);
}

#[test]
fn _0025() {
  te_days_and_time_duration_x(false, &scope!(), r#" @"2021-02-06" - @"2023-02-18" "#, "-P742D");
}

#[test]
fn _0026() {
  te_be_value(false, &scope!(), r#" @"23746-01-01" + @"P99999999Y" "#, r#"date("100023745-01-01")"#);
}

#[test]
fn _0027() {
  te_days_and_time_duration_x(false, &scope!(), r#" @"2023-02-06T12:13:18Z" - @"2021-02-06" "#, "P730DT12H13M18S");
}

#[test]
fn _0028() {
  te_days_and_time_duration_x(false, &scope!(), r#" @"2023-02-06T12:13:18" - @"2021-02-06T12:13:18" "#, "P730D");
}

#[test]
fn _0029() {
  te_date_time_local(false, &scope!(), r#" @"2023-02-06T12:13:18" - @"P1DT2H" "#, (2023, 2, 5), (10, 13, 18, 0));
}

#[test]
fn _0030() {
  te_date_time_utc(false, &scope!(), r#" @"2023-02-06T12:13:18Z" - @"P1DT2H" "#, (2023, 2, 5), (10, 13, 18, 0));
}

#[test]
fn _0031() {
  te_days_and_time_duration_x(false, &scope!(), r#" @"12:34:56" - @"09:15:36" "#, "PT3H19M20S");
}

#[test]
fn _0032() {
  te_time(false, &scope!(), r#" @"12:34:56" - (-@"P1DT2H") "#, FeelTime::local_opt(14, 34, 56, 0).unwrap());
}

#[test]
fn _0033() {
  te_time(false, &scope!(), r#" @"12:34:56" - @"PT14H" "#, FeelTime::local_opt(22, 34, 56, 0).unwrap());
}

#[test]
fn _0034() {
  te_date_time_local(false, &scope!(), r#" @"2023-02-06T12:13:18" - @"P1Y" "#, (2022, 2, 6), (12, 13, 18, 0));
}

#[test]
fn _0035() {
  te_days_and_time_duration_x(false, &scope!(), r#" @"P2DT3H" - @"PT3H" "#, "P2D");
}

#[test]
fn _0036() {
  te_years_and_months_duration_x(false, &scope!(), r#" @"P2Y5M" - @"P5M" "#, "P2Y");
}

/// Date converted to date and time has the timezone equal to UTC 00:00:00. Subtracted date and time has local timezone.
/// Such subtraction is not allowed (both date and times should have offset defined), that's why null value should be returned.
#[test]
fn _0037() {
  te_null(
    false,
    &scope!(),
    r#" @"2021-01-02" - @"2021-01-01T10:10:10" "#,
    "[subtraction] incompatible types: 2021-01-02 - 2021-01-01T10:10:10",
  );
}

#[test]
fn _0038() {
  te_null(
    false,
    &scope!(),
    r#" @"2023-02-06T12:13:18" - 5 "#,
    "[subtraction] incompatible types: 2023-02-06T12:13:18 - 5",
  );
}

#[test]
fn _0039() {
  te_date_time_local(false, &scope!(), r#" @"262142-12-31T00:00:00" - @"P20D" "#, (262142, 12, 11), (0, 0, 0, 0));
}

#[test]
fn _0040() {
  te_date_time_local(false, &scope!(), r#" @"262142-12-31T00:00:00" - @"P20Y" "#, (262122, 12, 31), (0, 0, 0, 0));
}

#[test]
fn _0041() {
  te_date_time_local(false, &scope!(), r#" @"262142-12-31T00:00:00" - @"P2DT3H" "#, (262142, 12, 28), (21, 0, 0, 0));
}

#[test]
fn _0042() {
  // Time minus duration wraps around midnight, nanoseconds included.
  te_time(false, &scope!(), r#" @"00:00:00" - @"PT1S" "#, FeelTime::local_opt(23, 59, 59, 0).unwrap());
  te_be_value(false, &scope!(), r#" string(@"00:00:00" - @"PT0.000000001S") "#, r#""23:59:59.999999999""#);
  te_be_value(false, &scope!(), r#" string(@"10:00:01.25" - @"PT0.75S") "#, r#""10:00:00.5""#);
  te_time(false, &scope!(), r#" @"01:00:00Z" - @"PT2H" "#, FeelTime::utc_opt(23, 0, 0, 0).unwrap());
}

#[test]
fn _0043() {
  // `a - 1 - 1 - …` with `a` a string: the message of every failing subtraction shows the operands
  // as evaluated, it does not evaluate them again. Each evaluation of a failing subtraction adds
  // one diagnostic, so 16 subtractions give 16 diagnostics; evaluating the operands again for the
  // message would give 2^16 - 1.
  let scope = &te_scope(r#"{a: "x"}"#);
  let expression = format!("a{}", " - 1".repeat(16));
  let node = rheon_feel_parser::parse_expression(scope, &expression).unwrap();
  let evaluator = build_evaluator(&node);
  let (value, diagnostics) = rheon_feel::diagnostics::collect(|| evaluator(scope));
  assert!(value.is_null());
  assert_eq!(16, diagnostics.count);
  te_null(false, scope, r#""a" - 1"#, r#"[subtraction] incompatible types: "a" - 1"#);
  te_null(false, scope, r#"date("2020-01-01") - "x""#, r#"[subtraction] incompatible types: 2020-01-01 - "x""#);
}

#[test]
fn _0044() {
  // A difference of -2^63 months or nanoseconds is out of the range of a duration.
  let scope = &scope!();
  te_null(
    false,
    scope,
    r#"duration("P1M") * -9223372036854775807 - duration("P1M")"#,
    "years and months duration subtraction overflow",
  );
  te_null(
    false,
    scope,
    r#"duration("PT0.000000001S") * -9223372036854775807 - duration("PT0.000000001S")"#,
    "days and time duration subtraction overflow",
  );
  te_null(
    false,
    scope,
    r#"date and time("1677-09-21T00:12:43.145224192Z") - date and time("1970-01-01T00:00:00Z")"#,
    "[subtraction] incompatible types: 1677-09-21T00:12:43.145224192Z - 1970-01-01T00:00:00Z",
  );
  te_days_and_time_duration_x(
    false,
    scope,
    r#"date and time("1677-09-21T00:12:43.145224193Z") - date and time("1970-01-01T00:00:00Z")"#,
    "-P106751DT23H47M16.854775807S",
  );
}

#[test]
fn _0045() {
  // A date and time minus a duration before the first supported instant is null.
  let scope = &scope!();
  te_null(
    false,
    scope,
    r#"date and time("-262143-01-01T00:00:00Z") - duration("P1D")"#,
    "[subtraction] incompatible types: -262143-01-01T00:00:00Z - P1D",
  );
  te_null(
    false,
    scope,
    r#"date and time("-262143-01-01T00:00:00Z") - duration("PT0.000000001S")"#,
    "[subtraction] incompatible types: -262143-01-01T00:00:00Z - PT0.000000001S",
  );
  te_null(
    false,
    scope,
    r#"date and time("262142-12-31T23:59:59Z") - duration("-PT1S")"#,
    "[subtraction] incompatible types: 262142-12-31T23:59:59Z - -PT1S",
  );
  te_value(
    false,
    scope,
    r#"date and time("-262143-01-01T00:00:01Z") - duration("PT1S")"#,
    r#"date and time("-262143-01-01T00:00:00Z")"#,
  );
  te_value(false, scope, r#"date("2020-03-31") - duration("P1M")"#, r#"date("2020-02-29")"#);
  te_value(false, scope, r#"date("2020-03-31") - duration("-P11M")"#, r#"date("2021-02-28")"#);
  te_value(
    false,
    scope,
    r#"date and time("2020-03-31T10:00:00Z") - duration("P1M")"#,
    r#"date and time("2020-02-29T10:00:00Z")"#,
  );
}

#[test]
fn _0046() {
  // A date is the date and time at UTC midnight, so a date minus a days and time duration is
  // date(valuedt⁻¹(valuedt(e1) - valuedtd(e2))): the date of the instant, whatever its time of day.
  let scope = &scope!();
  te_date(false, scope, r#"date("2020-01-02") - duration("PT0.5S")"#, 2020, 1, 1);
  te_date(false, scope, r#"date("2020-01-02") - duration("PT0.000000001S")"#, 2020, 1, 1);
  te_date(false, scope, r#"date("2020-01-02") - duration("P1DT0.5S")"#, 2019, 12, 31);
  te_date(false, scope, r#"date("2020-01-02") - duration("PT1S")"#, 2020, 1, 1);
  te_date(false, scope, r#"date("2020-01-02") - duration("P1D")"#, 2020, 1, 1);
  te_date(false, scope, r#"date("2020-01-02") - duration("PT24H")"#, 2020, 1, 1);
  te_date(false, scope, r#"date("2020-01-02") - duration("-PT23H59M59.999999999S")"#, 2020, 1, 2);
  te_date(false, scope, r#"date("2020-01-02") - duration("-P1DT0.5S")"#, 2020, 1, 3);
}

#[test]
fn a_null_operand_of_a_subtraction_is_an_error() {
  // DMN 1.6, before Table 57: "if input values are not of the listed types, the result is null";
  // the official TCK expects an error result for a null operand on either side
  // (compliance-level-3/0100-arithmetic, error_when_subtract_lhs_null_minus_rhs_number and the others)
  te_null_operands("subtraction", "-");
}
