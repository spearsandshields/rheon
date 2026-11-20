use super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_bool(false, &scope!(), r#"1<2"#, true);
}

#[test]
fn _0002() {
  te_null(
    false,
    &scope!(),
    r#" 1 < "2" "#,
    "comparison '<' expected number on the right, as on the left, found string",
  );
}

#[test]
fn _0003() {
  te_null(
    false,
    &scope!(),
    r#" false < 21 "#,
    "comparison '<' expected a number, string, date, time, date and time, or duration on the left, found boolean",
  );
}

#[test]
fn _0004() {
  te_bool(false, &scope!(), r#"1.276<1.277"#, true);
}

#[test]
fn _0005() {
  te_bool(false, &scope!(), r#"-.5<.54635"#, true);
}

#[test]
fn _0006() {
  te_bool(false, &scope!(), r#"2<1"#, false);
}

#[test]
fn _0007() {
  te_bool(false, &scope!(), r#"(1+1)<2.01"#, true);
}

#[test]
fn _0008() {
  te_bool(false, &scope!(), r#"(1.1+2)<3.0"#, false);
}

#[test]
fn _0009() {
  te_bool(false, &scope!(), r#" ( 1 + 0.99 ) < 2.0"#, true);
}

#[test]
fn _0010() {
  te_bool(false, &scope!(), r#" ( ( ( 1.1 + 3.1 ) ) ) < 2.5"#, false);
}

#[test]
fn _0011() {
  te_bool(false, &scope!(), r#"(0.9+1)<(5.1-3)"#, true);
}

#[test]
fn _0012() {
  te_bool(false, &scope!(), r#"(1*2)<(10.0/4.9)"#, true);
}

#[test]
fn _0013() {
  te_bool(false, &scope!(), r#" "alfa" < "alfa" "#, false);
}

#[test]
fn _0014() {
  te_null(
    false,
    &scope!(),
    r#" "alfa" < 23 "#,
    "comparison '<' expected string on the right, as on the left, found number",
  );
}

#[test]
fn _0015() {
  te_bool(false, &scope!(), r#" "beta" < "alfa" "#, false);
}

#[test]
fn _0016() {
  te_bool(false, &scope!(), r#" "alfa" < "beta" "#, true);
}

#[test]
fn _0017() {
  te_bool(false, &scope!(), r#"@"2021-11-10" < @"2021-11-11""#, true);
}

#[test]
fn _0018() {
  te_null(
    false,
    &scope!(),
    r#" @"2021-11-10" < "2021-11-11" "#,
    "comparison '<' expected date on the right, as on the left, found string",
  );
}

#[test]
fn _0019() {
  te_bool(false, &scope!(), r#"@"2021-11-10" < @"2021-11-10""#, false);
}

#[test]
fn _0020() {
  te_bool(false, &scope!(), r#"@"2021-11-10" < @"2021-11-09""#, false);
}

#[test]
fn _0021() {
  te_bool(false, &scope!(), r#" @"2021-11-01T11:10:12" < @"2021-11-01T11:10:12" "#, false);
}

#[test]
fn _0022() {
  te_null(
    false,
    &scope!(),
    r#" @"2021-11-01T11:10:12" < "2021-11-01T11:10:12" "#,
    "comparison '<' expected date and time on the right, as on the left, found string",
  );
}

#[test]
fn _0023() {
  te_bool(false, &scope!(), r#" @"2021-11-01T11:10:13" < @"2021-11-01T11:10:12" "#, false);
}

#[test]
fn _0024() {
  te_bool(false, &scope!(), r#" @"2021-11-01T11:10:11" < @"2021-11-01T11:10:12" "#, true);
}

#[test]
fn _0025() {
  te_bool(false, &scope!(), r#" @"11:10:12" < @"11:10:12" "#, false);
}

#[test]
fn _0026() {
  te_null(
    false,
    &scope!(),
    r#" @"11:10:12" < "11:10:12" "#,
    "comparison '<' expected time on the right, as on the left, found string",
  );
}

#[test]
fn _0027() {
  te_bool(false, &scope!(), r#" @"11:10:13" < @"11:10:12" "#, false);
}

#[test]
fn _0028() {
  te_bool(false, &scope!(), r#" @"11:10:11" < @"11:10:12" "#, true);
}

#[test]
fn _0029() {
  te_bool(false, &scope!(), r#" @"10:10:10.23" < @"10:10:10.24" "#, true);
}

#[test]
fn _0030() {
  te_bool(false, &scope!(), r#" @"P1D" < @"P1D" "#, false);
}

#[test]
fn _0031() {
  te_null(
    false,
    &scope!(),
    r#" @"P1D" < "P1D" "#,
    "comparison '<' expected days and time duration on the right, as on the left, found string",
  );
}

#[test]
fn _0032() {
  te_bool(false, &scope!(), r#" @"P2D" < @"P1D" "#, false);
}

#[test]
fn _0033() {
  te_bool(false, &scope!(), r#" @"P1D" < @"P2D" "#, true);
}

#[test]
fn _0034() {
  te_bool(false, &scope!(), r#" @"P1Y" < @"P1Y" "#, false);
}

#[test]
fn _0035() {
  te_null(
    false,
    &scope!(),
    r#" @"P1Y" < "P1Y" "#,
    "comparison '<' expected years and months duration on the right, as on the left, found string",
  );
}

#[test]
fn _0036() {
  te_bool(false, &scope!(), r#" @"P2Y" < @"P1Y" "#, false);
}

#[test]
fn _0037() {
  te_bool(false, &scope!(), r#" @"P1Y" < @"P2Y" "#, true);
}

#[test]
fn _0038() {
  // Date and times with different offsets or zones are ordered as instants (regression:
  // ordering across offsets was always false although equality and subtraction honour them).
  te_bool(false, &scope!(), r#" @"2026-10-17T10:00:00+01:00" < @"2026-10-17T10:00:00Z" "#, true);
  te_bool(false, &scope!(), r#" @"2026-10-17T10:00:00Z" < @"2026-10-17T10:00:00+01:00" "#, false);
  te_bool(false, &scope!(), r#" @"2026-10-17T10:00:00+01:00" < @"2026-10-17T09:00:00Z" "#, false);
  te_bool(false, &scope!(), r#" @"2026-10-17T10:00:00+01:00" <= @"2026-10-17T09:00:00Z" "#, true);
  te_bool(false, &scope!(), r#" @"2026-10-17T10:00:00+01:00" > @"2026-10-17T08:59:59Z" "#, true);
  te_bool(false, &scope!(), r#" @"2026-10-17T10:00:00@Europe/Warsaw" < @"2026-10-17T10:00:00Z" "#, true);
  te_bool(false, &scope!(), r#" @"2026-10-17T10:00:00Z" < @"2026-10-17T10:00:00+00:00" "#, false);
  te_bool(false, &scope!(), r#" @"2026-10-17T10:00:00Z" <= @"2026-10-17T10:00:00+00:00" "#, true);
  te_bool(
    false,
    &scope!(),
    r#" @"2026-10-17T10:00:00+01:00" in [@"2026-10-17T09:00:00Z"..@"2026-10-17T09:00:00Z"] "#,
    true,
  );
  te_be_value(
    false,
    &scope!(),
    r#" sort([@"2026-10-17T10:00:00Z", @"2026-10-17T10:00:00+01:00"], function(a, b) a < b) "#,
    r#" [@"2026-10-17T10:00:00+01:00", @"2026-10-17T10:00:00Z"] "#,
  );
  // a local and a zoned date and time an hour apart have no order (like their subtraction): null
  te_null(
    false,
    &scope!(),
    r#" @"2026-10-17T10:00:00" < @"2026-10-17T11:00:00Z" "#,
    "comparison '<' expected two date and times that have an order, found 2026-10-17T10:00:00 and 2026-10-17T11:00:00Z: one without an offset and one with an offset have an order only when they are more than 14 hours apart",
  );
}

#[test]
fn _0039() {
  // Times with different offsets are ordered (and equal) as instants of the day in UTC.
  te_bool(false, &scope!(), r#" @"10:00:00+01:00" < @"10:00:00Z" "#, true);
  te_bool(false, &scope!(), r#" @"10:00:00Z" < @"10:00:00+01:00" "#, false);
  te_bool(false, &scope!(), r#" @"10:00:00+02:00" = @"08:00:00Z" "#, true);
  te_bool(false, &scope!(), r#" @"10:00:00+02:00" < @"08:00:00Z" "#, false);
  te_bool(false, &scope!(), r#" @"10:00:00+02:00" <= @"08:00:00Z" "#, true);
  te_bool(false, &scope!(), r#" @"10:00:00+02:00" > @"07:59:59Z" "#, true);
  te_bool(false, &scope!(), r#" @"10:00:00" < @"11:00:00" "#, true);
  // a time without an offset has no order against one with an offset an hour away
  let node = rheon_feel_parser::parse_expression(&scope!(), r#" @"10:00:00" < @"11:00:00Z" "#).unwrap();
  assert!(crate::tests::eval(&scope!(), &node).is_null());
}

/// A time without an offset is a time of day somewhere: against a time with an offset it has
/// an order only when the two are more than 14 hours apart, whatever the place is (DMN 1.6,
/// 10.3.2.3.4, which refers to the order of XML Schema). Otherwise the comparison is null,
/// not false; the two are never equal.
#[test]
fn a_time_without_an_offset_is_ordered_against_one_with_an_offset_only_beyond_14_hours() {
  let scope = &scope!();
  let unordered = |operator: &str, lhs: &str, rhs: &str| {
    format!(
      "comparison '{operator}' expected two times that have an order, found {lhs} and {rhs}: a time without an offset and a time with one have an order only when they are more than 14 hours apart"
    )
  };
  for (lhs, rhs) in [
    ("10:00:00", "11:00:00Z"),
    ("11:00:00Z", "10:00:00"),
    ("10:00:00", "10:00:00+02:00"),
    ("00:00:00", "14:00:00Z"),
    ("23:59:59", "09:59:59Z"),
  ] {
    for operator in ["<", "<=", ">", ">="] {
      te_null(false, scope, &format!(r#"time("{lhs}") {operator} time("{rhs}")"#), &unordered(operator, lhs, rhs));
    }
    te_bool(false, scope, &format!(r#"time("{lhs}") = time("{rhs}")"#), false);
    te_bool(false, scope, &format!(r#"time("{lhs}") != time("{rhs}")"#), true);
  }
  // more than 14 hours apart
  te_bool(false, scope, r#"time("00:00:00") < time("14:00:01Z")"#, true);
  te_bool(false, scope, r#"time("00:00:00") > time("14:00:01Z")"#, false);
  te_bool(false, scope, r#"time("14:00:01Z") > time("00:00:00")"#, true);
  te_bool(false, scope, r#"time("23:00:00") > time("08:59:59Z")"#, true);
  te_bool(false, scope, r#"time("08:59:59Z") <= time("23:00:00")"#, true);
  te_bool(false, scope, r#"time("00:00:00") <= time("23:00:00+01:00")"#, true);
  // two times of one kind are ordered as before
  te_bool(false, scope, r#"time("10:00:00") < time("11:00:00")"#, true);
  te_bool(false, scope, r#"time("10:00:00Z") < time("11:00:00+02:00")"#, false);
  // between and a range ask for the same order
  te_null(
    false,
    scope,
    r#"time("10:00:00") between time("09:00:00Z") and time("11:00:00Z")"#,
    &unordered("between", "09:00:00Z", "10:00:00"),
  );
  te_bool(false, scope, r#"time("23:30:00") between time("01:00:00Z") and time("23:59:59")"#, true);
}
