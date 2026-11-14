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
