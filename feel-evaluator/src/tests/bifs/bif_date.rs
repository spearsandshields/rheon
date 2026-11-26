use super::super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  let scope = &scope!();
  te_date(false, scope, r#"date("2012-12-25")"#, 2012, 12, 25);
}

#[test]
fn _0002() {
  let scope = &scope!();
  te_date(false, scope, r#"date(from: "2012-12-25")"#, 2012, 12, 25);
}

#[test]
fn _0003() {
  let scope = &scope!();
  te_date(false, scope, r#"date("2012-12-25")"#, 2012, 12, 25);
}

#[test]
fn _0004() {
  let scope = &scope!();
  te_date(false, scope, r#"date(2012,12,25)"#, 2012, 12, 25);
}

#[test]
fn _0005() {
  let scope = &scope!();
  te_date(false, scope, r#"date(year: 2012, month: 12, day: 25)"#, 2012, 12, 25);
}

#[test]
fn _0006() {
  let scope = &scope!();
  te_date(false, scope, r#"date(month: 12, day: 25, year: 2012)"#, 2012, 12, 25);
}

#[test]
fn _0007() {
  let scope = &scope!();
  te_date(false, scope, r#"date("262142-12-31")"#, 262142, 12, 31);
}

#[test]
fn _0008() {
  let scope = &scope!();
  te_date(false, scope, r#"date("-262143-01-01")"#, -262143, 1, 1);
}

#[test]
fn _0009() {
  te_be_value(false, &scope!(), r#"date("999999999-12-31")"#, r#"date("999999999-12-31")"#);
}

#[test]
fn _0010() {
  te_be_value(false, &scope!(), r#"date(999999999,12,31)"#, r#"date("999999999-12-31")"#);
}

#[test]
fn _0011() {
  let scope = &scope!();
  te_date(false, scope, r#"date(-262143,1,1)"#, -262143, 1, 1);
}

#[test]
fn _0012() {
  te_be_value(false, &scope!(), r#"date("-999999999-01-01")"#, r#"date("-999999999-01-01")"#);
}

#[test]
fn _0013() {
  te_be_value(false, &scope!(), r#"date(-999999999,01,01)"#, r#"date("-999999999-01-01")"#);
}

#[test]
fn _0014() {
  let scope = &scope!();
  te_date(false, scope, r#"date(date and time("2012-12-25T12:23:18"))"#, 2012, 12, 25);
}

#[test]
fn _0015() {
  let scope = &scope!();
  te_date(false, scope, r#"date(from: date and time("2012-12-25T12:23:18"))"#, 2012, 12, 25);
}

#[test]
fn _0016() {
  let scope = &scope!();
  te_date(false, scope, r#"date(date and time("2012-12-25T12:23:18Z"))"#, 2012, 12, 25);
}

#[test]
fn _0017() {
  let scope = &scope!();
  te_date(false, scope, r#"date(date and time("2012-12-25T12:23:18z"))"#, 2012, 12, 25);
}

#[test]
fn _0018() {
  let scope = &scope!();
  te_bool(false, scope, r#"date("2012-12-25") in [date("2012-12-24")..date("2012-12-26")]"#, true);
}

#[test]
fn _0019() {
  let scope = &scope!();
  te_bool(false, scope, r#"date("2000-12-25") in [date("2012-12-24")..date("2012-12-26")]"#, false);
}

#[test]
fn _0020() {
  let scope = &scope!();
  te_bool(false, scope, r#"date("2020-12-25") in [date("2012-12-24")..date("2012-12-26")]"#, false);
}

#[test]
fn _0021() {
  let scope = &scope!();
  te_bool(false, scope, r#"date("2012-12-31") in (date("2012-12-25")..date("2013-02-14"))"#, true);
}

#[test]
fn _0022() {
  let scope = &scope!();
  te_null(false, scope, r#"date("2017-13-10")"#, r#"[core::date] invalid date string '2017-13-10'"#);
}

#[test]
fn _0023() {
  let scope = &scope!();
  te_null(false, scope, r#"date("2017/12/10")"#, r#"[core::date] invalid date string '2017/12/10'"#);
}

#[test]
fn _0024() {
  let scope = &scope!();
  te_null(false, scope, r#"date("2017,12,31")"#, r#"[core::date] invalid date string '2017,12,31'"#);
}

#[test]
fn _0025() {
  let scope = &scope!();
  te_date(false, scope, r#"date("2012-12-25")"#, 2012, 12, 25);
  te_number(false, scope, r#"date("2012-12-25").day"#, 25, 0);
  te_number(false, scope, r#"date("2012-12-25").month"#, 12, 0);
  te_number(false, scope, r#"date("2012-12-25").year"#, 2012, 0);
}

#[test]
fn _0026() {
  let scope = &te_scope(r#"{fromString: "2012-12-25"}"#);
  te_date(false, scope, r#"date(fromString)"#, 2012, 12, 25);
  te_number(false, scope, r#"date(fromString).day"#, 25, 0);
  te_number(false, scope, r#"date(fromString).month"#, 12, 0);
  te_number(false, scope, r#"date(fromString).year"#, 2012, 0);
}

#[test]
fn _0027() {
  te_null(
    false,
    &scope!(),
    r#"date(10)"#,
    r#"[core::date] invalid argument type, expected string, date or date and time, actual type is number"#,
  );
}

#[test]
fn _0028() {
  te_date(false, &scope!(), r#"date(date("2021-01-06"))"#, 2021, 1, 6);
}

#[test]
fn _0029() {
  let scope = &scope!();
  te_null(false, scope, r#"date(2020,2,30)"#, r#"[core::date_3] invalid date y=2020 m=2 d=30"#);
}

#[test]
fn _0030() {
  let scope = &scope!();
  te_null(
    false,
    scope,
    r#"date(384756328456345,790485703985734,45490654698475)"#,
    r#"[core::date_3] invalid date y=384756328456345 m=790485703985734 d=45490654698475"#,
  );
}

#[test]
fn _0031() {
  let scope = &scope!();
  te_null(
    false,
    scope,
    r#"date(true,2,30)"#,
    r#"[core::date] invalid argument type, expected number (year), actual type is boolean"#,
  );
}

#[test]
fn _0032() {
  let scope = &scope!();
  te_null(
    false,
    scope,
    r#"date(2020,"02",30)"#,
    r#"[core::date] invalid argument type, expected number (month), actual type is string"#,
  );
}

#[test]
fn _0033() {
  let scope = &scope!();
  te_null(
    false,
    scope,
    r#"date(2020,2,null)"#,
    r#"[core::date] invalid argument type, expected number (day), actual type is Null"#,
  );
}

#[test]
fn _0034() {
  let scope = &scope!();
  te_null(false, scope, r#"date()"#, r#"expected 1,3 parameters, actual number of parameters is 0"#);
}

#[test]
fn _0035() {
  let scope = &scope!();
  te_null(false, scope, r#"date(2022,01)"#, r#"expected 1,3 parameters, actual number of parameters is 2"#);
}

#[test]
fn _0036() {
  let scope = &scope!();
  te_null(false, scope, r#"date(2022,1,1,1)"#, r#"expected 1,3 parameters, actual number of parameters is 4"#);
}

#[test]
fn _0037() {
  let scope = &scope!();
  te_null(
    false,
    scope,
    r#"date(f: "2021-01-26")"#,
    r#"expected the parameters of one of date(from): date, date(year, month, day): date, found the parameter name 'f'"#,
  );
}

#[test]
fn _0038() {
  let scope = &scope!();
  te_null(
    false,
    scope,
    r#"date(f: date and time("2012-12-25T12:23:18"))"#,
    r#"expected the parameters of one of date(from): date, date(year, month, day): date, found the parameter name 'f'"#,
  );
}

#[test]
fn _0039() {
  let scope = &scope!();
  te_null(
    false,
    scope,
    r#"date(m: 12, day: 25, year: 2012)"#,
    r#"expected the parameters of one of date(from): date, date(year, month, day): date, found the parameter name 'm'"#,
  );
}

#[test]
fn _0040() {
  let scope = &scope!();
  te_null(
    false,
    scope,
    r#"date(month: 12, d: 25, year: 2012)"#,
    r#"expected the parameters of one of date(from): date, date(year, month, day): date, found the parameter name 'd'"#,
  );
}

#[test]
fn _0041() {
  let scope = &scope!();
  te_null(
    false,
    scope,
    r#"date(month: 12, day: 25, y: 2012)"#,
    r#"expected the parameters of one of date(from): date, date(year, month, day): date, found the parameter name 'y'"#,
  );
}

/// The years of a date are those of FEEL, from -999,999,999 to 999,999,999 (DMN 1.6, 10.3.4.1):
/// such a date is built, shown, compared, read and moved as every date is.
#[test]
fn a_date_has_the_years_of_feel() {
  let scope = &scope!();
  te_be_value(false, scope, r#"string(date("999999999-12-31"))"#, r#""999999999-12-31""#);
  te_be_value(false, scope, r#"string(date("-999999999-01-01"))"#, r#""-999999999-01-01""#);
  te_be_value(false, scope, r#"string(date(999999999, 12, 31))"#, r#""999999999-12-31""#);
  te_be_value(false, scope, r#"string(@"262143-01-01")"#, r#""262143-01-01""#);
  te_bool(false, scope, r#"date("999999999-12-31") = date(999999999, 12, 31)"#, true);
  te_bool(false, scope, r#"date("999999999-12-31") > date("999999999-12-30")"#, true);
  te_bool(false, scope, r#"date("-999999999-01-01") < date("2024-01-01")"#, true);
  te_bool(false, scope, r#"date("300000-06-15") in [date("300000-01-01")..date("300000-12-31")]"#, true);
  te_be_value(false, scope, r#"date("999999999-12-31").year"#, "999999999");
  te_be_value(false, scope, r#"date("999999999-12-31").month"#, "12");
  te_be_value(false, scope, r#"day of week(date("999999999-12-31"))"#, r#""Friday""#);
  te_be_value(false, scope, r#"day of year(date("999999999-12-31"))"#, "365");
  te_be_value(false, scope, r#"week of year(date("999999999-12-31"))"#, "52");
  te_be_value(false, scope, r#"month of year(date("999999999-12-31"))"#, r#""December""#);
  te_be_value(false, scope, r#"string(date("999999999-12-30") + duration("P1D"))"#, r#""999999999-12-31""#);
  te_be_value(false, scope, r#"string(date("999999998-12-31") + duration("P1Y"))"#, r#""999999999-12-31""#);
  te_be_value(false, scope, r#"string(date("300000-03-01") - duration("P1D"))"#, r#""300000-02-29""#);
  te_be_value(false, scope, r#"date("300000-03-01") - date("300000-02-28")"#, r#"duration("P2D")"#);
  te_be_value(false, scope, r#"date("-300000-01-01") - date("-300000-01-02")"#, r#"duration("-P1D")"#);
  // beyond the years of FEEL there is no date
  te_null(false, scope, r#"date("1000000000-01-01")"#, "[core::date] invalid date string '1000000000-01-01'");
  te_null(
    false,
    scope,
    r#"date("999999999-12-31") + duration("P1D")"#,
    "invalid result while adding days and time duration to date",
  );
  te_null(false, scope, "date(1000000000, 1, 1)", "[core::date_3] invalid date y=1000000000 m=1 d=1");
}

/// A date and time has an instant only in the years that the calendar library holds (to
/// 262,142): beyond them there is no date and time, whatever it is built from.
#[test]
fn a_date_and_time_has_no_year_beyond_the_instants() {
  let scope = &scope!();
  for text in [
    r#"date and time("262143-01-01T00:00:00")"#,
    r#"date and time("999999999-12-31T23:59:59Z")"#,
    r#"date and time(date("262143-01-01"), time("00:00:00"))"#,
    r#"date and time(date("262143-01-01"), time("00:00:00"), "Z")"#,
    r#"date and time("262143-01-01")"#,
    r#"date and time("262142-12-31T23:00:00Z") + duration("PT1H")"#,
    r#"date and time("262142-12-31T23:00:00Z") + duration("P1M")"#,
    r#"{f: function(d: date and time) d, r: f(date("262143-01-01"))}.r"#,
  ] {
    let node = rheon_feel_parser::parse_expression(scope, text).unwrap();
    assert!(crate::tests::eval(scope, &node).is_null(), "{text}");
  }
  te_be_value(false, scope, r#"string(date and time("262142-12-31T23:00:00Z"))"#, r#""262142-12-31T23:00:00Z""#);
}

#[test]
fn _0042() {
  // the date string is an xs:date, which may have a zone; a date holds none, so it is dropped with a
  // warning (10.3.4.1, 10.3.2.3.5)
  te_warnings(
    r#"date("2020-01-01Z")"#,
    "2020-01-01",
    &["the date '2020-01-01Z' has the zone 'Z', which a date does not hold; the date is taken without it"],
  );
  te_warnings(
    r#"date("2020-01-01+02:00")"#,
    "2020-01-01",
    &["the date '2020-01-01+02:00' has the zone '+02:00', which a date does not hold; the date is taken without it"],
  );
  te_warnings(
    r#"@"2020-01-01Z""#,
    "2020-01-01",
    &["the date '2020-01-01Z' has the zone 'Z', which a date does not hold; the date is taken without it"],
  );
  te_warnings(
    r#"date("2020-06-30-05:30")"#,
    "2020-06-30",
    &["the date '2020-06-30-05:30' has the zone '-05:30', which a date does not hold; the date is taken without it"],
  );
  te_warnings(r#"date("2020-01-01")"#, "2020-01-01", &[]);
  te_null(false, &scope!(), r#"date("2020-01-01+15:00")"#, "[core::date] invalid date string '2020-01-01+15:00'");
  te_null(
    false,
    &scope!(),
    r#"date("2020-01-01@Europe/Paris")"#,
    "[core::date] invalid date string '2020-01-01@Europe/Paris'",
  );
}

#[test]
fn _0043() {
  // a year of four digits may have leading zeros, and a negative year keeps four digits after its
  // sign (XML Schema Part 2, yearFrag), so a date's text reads back as the date
  te_date(false, &scope!(), r#"date("0044-01-01")"#, 44, 1, 1);
  te_date(false, &scope!(), r#"date("-0044-03-15")"#, -44, 3, 15);
  te_date(false, &scope!(), r#"date("0001-01-01")"#, 1, 1, 1);
  te_string(false, &scope!(), r#"string(date(-44, 3, 15))"#, "-0044-03-15");
  te_string(false, &scope!(), r#"string(date(-5, 1, 1))"#, "-0005-01-01");
  te_bool(false, &scope!(), r#"date(string(date(44, 1, 1))) = date(44, 1, 1)"#, true);
  // the year 0000 and a longer year with a leading zero are no years
  te_null(false, &scope!(), r#"date("0000-01-01")"#, "[core::date] invalid date string '0000-01-01'");
  te_null(false, &scope!(), r#"date("00044-01-01")"#, "[core::date] invalid date string '00044-01-01'");
}
