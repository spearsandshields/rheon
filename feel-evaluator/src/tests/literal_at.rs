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
