use super::*;

#[test]
fn test_0001() {
  let scope = &te_scope(r#"{}"#);
  te_value(false, scope, r#"@"2021-01-28""#, r#"date("2021-01-28")"#);
}

#[test]
fn test_0002() {
  let scope = &te_scope(r#"{}"#);
  te_value(false, scope, r#"@"2021-01-28T19:15:00""#, r#"date and time("2021-01-28T19:15:00")"#);
}

#[test]
fn test_0003() {
  let scope = &te_scope(r#"{}"#);
  te_value(false, scope, r#"@"-PT1H""#, r#"duration("-PT1H")"#);
}

#[test]
fn test_0004() {
  let scope = &te_scope(r#"{}"#);
  te_value(false, scope, r#"@"P2Y""#, r#"duration("P2Y")"#);
}

#[test]
fn test_0005() {
  let scope = &te_scope(r#"{}"#);
  te_null(false, scope, r#"@"airbus""#, r#"invalid @ literal: airbus"#);
}

#[test]
fn fractional_seconds_are_exact() {
  let scope = &te_scope(r#"{}"#);
  te_string(false, scope, r#"string(@"10:00:00.0157")"#, "10:00:00.0157");
  te_string(false, scope, r#"string(@"2020-01-01T10:00:00.0157Z")"#, "2020-01-01T10:00:00.0157Z");
  te_string(false, scope, r#"string(@"PT0.000015839S")"#, "PT0.000015839S");
}

#[test]
fn midnight_may_be_written_24_00_00_with_a_zero_fraction_only() {
  let scope = &te_scope(r#"{}"#);
  te_null(false, scope, r#"@"24:00:00.5""#, r#"invalid @ literal: 24:00:00.5"#);
  te_null(false, scope, r#"@"2020-01-01T24:00:00.5Z""#, r#"invalid @ literal: 2020-01-01T24:00:00.5Z"#);
  te_string(false, scope, r#"string(@"2020-12-31T24:00:00Z")"#, "2021-01-01T00:00:00Z");
}

#[test]
fn fraction_digits_beyond_nanoseconds_warn() {
  let warning = truncation_warning("10:00:00.1234567891", "1");
  te_warnings(r#"@"10:00:00.1234567891""#, "10:00:00.123456789", &[&warning]);
  te_warnings(
    r#"@"2018-10-01T12:32:59.123123123123""#,
    "2018-10-01T12:32:59.123123123",
    &[&truncation_warning("2018-10-01T12:32:59.123123123123", "123")],
  );
  te_warnings(r#"@"PT999.999999999999S""#, "PT16M39.999999999S", &[&truncation_warning("PT999.999999999999S", "999")]);
  // only zeros dropped: nothing is lost
  te_warnings(r#"@"10:00:00.1234567890""#, "10:00:00.123456789", &[]);
  te_warnings(r#"@"10:00:00.123456789""#, "10:00:00.123456789", &[]);
  // a literal evaluated repeatedly warns on each evaluation, kept once in a row
  te_warnings(
    r#"for i in 1..3 return @"10:00:00.1234567891""#,
    "[10:00:00.123456789, 10:00:00.123456789, 10:00:00.123456789]",
    &[&warning],
  );
}
