use super::super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_time(false, &scope!(), r#"time(11,59,45,null)"#, FeelTime::local_opt(11, 59, 45, 0).unwrap());
}

#[test]
fn _0002() {
  te_time(
    false,
    &scope!(),
    r#"time(hour: 11, minute: 59, second: 45, offset: null)"#,
    FeelTime::local_opt(11, 59, 45, 0).unwrap(),
  );
}

#[test]
fn _0003() {
  te_time(false, &scope!(), r#"time(hour: 11, minute: 59, second: 45)"#, FeelTime::local_opt(11, 59, 45, 0).unwrap());
}

#[test]
fn _0004() {
  let scope = &te_scope(r#"{Hours:12,Minutes:59,Seconds:1.3,Timezone:@"-PT1H"}"#);
  te_time(
    false,
    scope,
    r#"time(Hours,Minutes,Seconds,Timezone)"#,
    FeelTime::offset_opt(12, 59, 1, 300_000_000, -3600).unwrap(),
  );
}

#[test]
fn _0005() {
  let scope = &te_scope(r#"{Hours:12,Minutes:59,Seconds:1.9999999999999999999999999999999999999999999999999,Timezone:@"-PT1H"}"#);
  te_time(false, scope, r#"time(Hours,Minutes,Seconds,Timezone)"#, FeelTime::offset_opt(12, 59, 2, 0, -3600).unwrap());
}

#[test]
fn _0006() {
  let scope = &te_scope(r#"{Hours:12.999,Minutes:59.999,Seconds:1.0003,Timezone:@"-PT1H3S"}"#);
  te_time(
    false,
    scope,
    r#"time(Hours,Minutes,Seconds,Timezone)"#,
    FeelTime::offset_opt(12, 59, 1, 300_000, -3603).unwrap(),
  );
}

#[test]
fn _0007() {
  te_time(false, &scope!(), r#"time("23:59:00")"#, FeelTime::local_opt(23, 59, 0, 0).unwrap());
}

#[test]
fn _0008() {
  te_time(false, &scope!(), r#"time("23:59:59")"#, FeelTime::local_opt(23, 59, 59, 0).unwrap());
}

#[test]
fn _0009() {
  te_time(false, &scope!(), r#"time("24:00:00")"#, FeelTime::local_opt(0, 0, 0, 0).unwrap());
}

#[test]
fn _0010() {
  te_time(false, &scope!(), r#"  time  (  "23:59:00"         )   "#, FeelTime::local_opt(23, 59, 0, 0).unwrap());
}

#[test]
fn _0011() {
  te_time(false, &scope!(), r#"time("23:59:00Z")"#, FeelTime::utc_opt(23, 59, 0, 0).unwrap());
}

#[test]
fn _0012() {
  te_time(false, &scope!(), r#"time("23:59:00z")"#, FeelTime::utc_opt(23, 59, 0, 0).unwrap());
}

#[test]
fn _0013() {
  te_time(false, &scope!(), r#"time("11:22:33-00:00")"#, FeelTime::utc_opt(11, 22, 33, 0).unwrap());
}

#[test]
fn _0014() {
  te_time(false, &scope!(), r#"time(from: "11:22:33-00:00")"#, FeelTime::utc_opt(11, 22, 33, 0).unwrap());
}

#[test]
fn _0015() {
  te_time(false, &scope!(), r#"time("11:22:33+00:00")"#, FeelTime::utc_opt(11, 22, 33, 0).unwrap());
}

#[test]
fn _0016() {
  te_time(false, &scope!(), r#"time(time("11:00:00"))"#, FeelTime::local_opt(11, 0, 0, 0).unwrap());
}

#[test]
fn _0017() {
  te_time(false, &scope!(), r#"time(from: time("11:00:00"))"#, FeelTime::local_opt(11, 0, 0, 0).unwrap());
}

#[test]
fn _0018() {
  te_time(
    false,
    &scope!(),
    r#"time(date and time("2019-12-06T18:34:12"))"#,
    FeelTime::local_opt(18, 34, 12, 0).unwrap(),
  );
}

#[test]
fn _0019() {
  te_time(false, &scope!(), r#"time(date and time("2019-12-06T11:00:00Z"))"#, FeelTime::utc_opt(11, 0, 0, 0).unwrap());
}

#[test]
fn _0020() {
  te_time(
    false,
    &scope!(),
    r#"time(from: date and time("2019-12-06T11:00:00Z"))"#,
    FeelTime::utc_opt(11, 0, 0, 0).unwrap(),
  );
}

#[test]
fn _0021() {
  te_time(false, &scope!(), r#"time(date and time("2019-12-06T11:00:00z"))"#, FeelTime::utc_opt(11, 0, 0, 0).unwrap());
}

#[test]
fn _0022() {
  te_time(false, &scope!(), r#"time(date("2019-12-06"))"#, FeelTime::utc_opt(0, 0, 0, 0).unwrap());
}

#[test]
fn _0023() {
  te_bool(false, &scope!(), r#"time("12:21:12") in [time("12:21:12")..time("12:21:12")]"#, true);
}

#[test]
fn _0024() {
  te_bool(false, &scope!(), r#"time("12:21:11") in [time("12:21:12")..time("12:21:12")]"#, false);
}

#[test]
fn _0025() {
  te_bool(false, &scope!(), r#"time("12:21:13") in [time("12:21:12")..time("12:21:12")]"#, false);
}

#[test]
fn _0026() {
  te_bool(false, &scope!(), r#"time("12:21:12") in (time("12:21:11")..time("12:21:13"))"#, true);
}

#[test]
fn _0027() {
  te_null(
    false,
    &scope!(),
    r#"time("22:63:12")"#,
    "time: invalid time literal '22:63:12', expected hh:mm:ss with an optional fraction and an optional Z, @zone or ±hh:mm offset within -14:00..+14:00",
  );
}

#[test]
fn _0028() {
  te_null(
    false,
    &scope!(),
    r#"time("22:10:12+15:00")"#,
    "time: invalid time literal '22:10:12+15:00', expected hh:mm:ss with an optional fraction and an optional Z, @zone or ±hh:mm offset within -14:00..+14:00",
  );
}

#[test]
fn _0029() {
  te_null(
    false,
    &scope!(),
    r#"time("22:10:12-15:00")"#,
    "time: invalid time literal '22:10:12-15:00', expected hh:mm:ss with an optional fraction and an optional Z, @zone or ±hh:mm offset within -14:00..+14:00",
  );
}

#[test]
fn _0030() {
  te_null(false, &scope!(), r#"time(24,59,45,null)"#, "[core::time_4] hour must be 0..23, current value is: 24");
}

#[test]
fn _0031() {
  te_null(false, &scope!(), r#"time(23,60,45,null)"#, "[core::time_4] minute must be 0..59, current value is: 60");
}

#[test]
fn _0032() {
  te_null(false, &scope!(), r#"time(24,59,45,null)"#, "[core::time_4] hour must be 0..23, current value is: 24");
}

#[test]
fn _0033() {
  te_null(false, &scope!(), r#"time(23,60,45,null)"#, "[core::time_4] minute must be 0..59, current value is: 60");
}

#[test]
fn _0034() {
  te_null(false, &scope!(), r#"time(23,59,60,null)"#, "[core::time_4] second must be 0..59, current value is: 60");
}

#[test]
fn _0035() {
  te_null(false, &scope!(), r#"time(-12,12,12,null)"#, "[core::time_4] hour must be 0..23, current value is: -12");
}

#[test]
fn _0036() {
  te_null(false, &scope!(), r#"time(12,-12,12,null)"#, "[core::time_4] minute must be 0..59, current value is: -12");
}

#[test]
fn _0037() {
  te_null(false, &scope!(), r#"time(12,12,-12,null)"#, "[core::time_4] second must be 0..59, current value is: -12");
}

#[test]
fn _0038() {
  te_time(false, &scope!(), r#"time(11,59,45)"#, FeelTime::local_opt(11, 59, 45, 0).unwrap());
}

#[test]
fn _0039() {
  te_null(false, &scope!(), r#"time()"#, r#"expected 1,3,4 parameters, actual number of parameters is 0"#);
}

#[test]
fn _0040() {
  te_null(false, &scope!(), r#"time(12,12)"#, r#"expected 1,3,4 parameters, actual number of parameters is 2"#);
}

#[test]
fn _0041() {
  te_null(
    false,
    &scope!(),
    r#"time(12,12,12,12,12)"#,
    r#"expected 1,3,4 parameters, actual number of parameters is 5"#,
  );
}

#[test]
fn _0042() {
  te_null(false, &scope!(), r#"time(f: "11:22:33-00:00")"#, r#"invalid parameters in bif time"#);
}

#[test]
fn _0043() {
  te_null(
    false,
    &scope!(),
    r#"time(h: 11, minute: 59, second: 45, offset: null)"#,
    r#"invalid parameters in bif time"#,
  );
}

#[test]
fn _0044() {
  te_null(false, &scope!(), r#"time(hour: 11, m: 59, second: 45, offset: null)"#, r#"invalid parameters in bif time"#);
}

#[test]
fn _0045() {
  te_null(false, &scope!(), r#"time(hour: 11, minute: 59, s: 45, offset: null)"#, r#"invalid parameters in bif time"#);
}

#[test]
fn _0046() {
  te_null(
    false,
    &scope!(),
    r#"time("11", 59, 45, null)"#,
    r#"[core::time_4] hour must be a number, current type is: string"#,
  );
}

#[test]
fn _0047() {
  te_null(
    false,
    &scope!(),
    r#"time(11, "59", 45, null)"#,
    r#"[core::time_4] minutes must be a number, current type is: string"#,
  );
}

#[test]
fn _0048() {
  te_null(
    false,
    &scope!(),
    r#"time(11, 59, "45", null)"#,
    r#"[core::time_4] seconds must be a number, current type is: string"#,
  );
}

#[test]
fn _0049() {
  te_null(
    false,
    &scope!(),
    r#"time(23,59,45,"10")"#,
    "expected days and time duration or null, current offset type is string",
  );
}

#[test]
fn _0050() {
  te_null(false, &scope!(), r#"time(24,59,3)"#, r#"hour must be 0..23, current value is 24"#);
}

#[test]
fn _0051() {
  te_null(false, &scope!(), r#"time(12,61,3)"#, r#"minute must be 0..59, current value is 61"#);
}

#[test]
fn _0052() {
  te_null(false, &scope!(), r#"time(11,59,300)"#, r#"second must be 0..59, current value is 300"#);
}

#[test]
fn _0053() {
  te_null(false, &scope!(), r#"time("12",45,13)"#, r#"hour must be a number, current type is string"#);
}

#[test]
fn _0054() {
  te_null(false, &scope!(), r#"time(12,"45",13)"#, r#"minutes must be a number, current type is string"#);
}

#[test]
fn _0055() {
  te_null(false, &scope!(), r#"time(12,45,"13")"#, r#"seconds must be a number, current type is string"#);
}

#[test]
#[should_panic]
fn _0056() {
  te_null(false, &scope!(), r#"time(23,59,45,@"P3000000000000000000D")"#, "");
}

#[test]
fn _0057() {
  te_null(
    false,
    &scope!(),
    r#"time(true)"#,
    "[core::time] invalid argument type, expected string, date, date and time, time, actual type is boolean",
  );
}

#[test]
fn fractional_seconds_are_exact() {
  te_string(false, &scope!(), r#"string(time("10:00:00.0157"))"#, "10:00:00.0157");
  te_string(false, &scope!(), r#"string(time("10:00:00.000065"))"#, "10:00:00.000065");
  te_string(false, &scope!(), r#"string(time("10:00:00.99999999999999999"))"#, "10:00:00.999999999");
}

#[test]
fn an_offset_prints_its_own_sign() {
  // XML Schema Part 2 §3.2.7.3: the sign of a timezone is the sign of the offset, also below one hour
  te_string(false, &scope!(), r#"string(time("10:00:00-00:30"))"#, "10:00:00-00:30");
  te_string(false, &scope!(), r#"string(@"10:00:00-00:30")"#, "10:00:00-00:30");
  te_string(false, &scope!(), r#"string(time(10, 0, 0, duration("-PT30M")))"#, "10:00:00-00:30");
  te_string(false, &scope!(), r#"string(time(10, 0, 0, duration("-PT30S")))"#, "10:00:00-00:00:30");
  te_string(false, &scope!(), r#"string(time(10, 0, 0, duration("PT30S")))"#, "10:00:00+00:00:30");
  te_string(false, &scope!(), r#"string(date and time("2020-01-01T10:00:00-00:30"))"#, "2020-01-01T10:00:00-00:30");
  te_string(
    false,
    &scope!(),
    r#"string(date and time(date(2020,1,1), time(10,0,0), "-00:30"))"#,
    "2020-01-01T10:00:00-00:30",
  );
  te_string(false, &scope!(), r#"string(time("10:00:00-01:30"))"#, "10:00:00-01:30");
  te_string(false, &scope!(), r#"string(time("10:00:00+00:30"))"#, "10:00:00+00:30");
}

#[test]
fn midnight_may_be_written_24_00_00_with_a_zero_fraction_only() {
  te_null(
    false,
    &scope!(),
    r#"time("24:00:00.5")"#,
    "time: invalid time literal '24:00:00.5', expected hh:mm:ss with an optional fraction and an optional Z, @zone or ±hh:mm offset within -14:00..+14:00",
  );
  te_string(false, &scope!(), r#"string(time("24:00:00"))"#, "00:00:00");
  te_string(false, &scope!(), r#"string(time("24:00:00.000"))"#, "00:00:00");
}

#[test]
fn fraction_digits_beyond_nanoseconds_warn() {
  te_warnings(r#"time("10:00:00.1234567891")"#, "10:00:00.123456789", &[&truncation_warning("10:00:00.1234567891", "1")]);
  te_warnings(
    r#"time("10:00:00.99999999999999999")"#,
    "10:00:00.999999999",
    &[&truncation_warning("10:00:00.99999999999999999", "99999999")],
  );
  te_warnings(r#"time("10:00:00.1234567890")"#, "10:00:00.123456789", &[]);
  // a long literal is quoted as a preview with its size, in a bounded message
  let fraction = format!("1{}", "9".repeat(99_999));
  let scope = FeelScope::default();
  let text = format!(r#"time("10:00:00.{fraction}")"#);
  let evaluator = build_evaluator(&rheon_feel_parser::parse_textual_expression(&scope, &text, false).unwrap());
  let (value, diagnostics) = rheon_feel::diagnostics::collect(|| evaluator(&scope));
  assert_eq!("10:00:00.199999999", value.to_string());
  let [warning] = diagnostics.warnings() else {
    panic!("one warning expected: {:?}", diagnostics.warnings())
  };
  assert!(warning.chars().count() <= 1_000, "{warning}");
  assert!(warning.contains("… (100009 characters)'") && warning.contains("… (99991 characters)'"), "{warning}");
}

#[test]
fn seconds_with_more_than_nine_fraction_digits_warn() {
  te_warnings("time(10, 0, 0.1234567891)", "10:00:00.123456789", &[&truncation_warning("0.1234567891", "1")]);
  te_warnings(
    "time(10, 0, 10/3)",
    "10:00:03.333333333",
    &[&truncation_warning("3.333333333333333333333333333333333", "333333333333333333333333")],
  );
  te_warnings(
    r#"time(10, 0, 0.1234567891, duration("PT1H"))"#,
    "10:00:00.123456789+01:00",
    &[&truncation_warning("0.1234567891", "1")],
  );
  te_warnings(r#"time(10, 0, 0.1234567891, null)"#, "10:00:00.123456789", &[&truncation_warning("0.1234567891", "1")]);
  // nothing is lost
  te_warnings("time(10, 0, 0.123456789)", "10:00:00.123456789", &[]);
  te_warnings("time(10, 0, 0.1234567890)", "10:00:00.123456789", &[]);
  te_warnings("time(10, 0, 59.5)", "10:00:59.5", &[]);
  // no time, no warning
  let scope = FeelScope::default();
  let text = r#"time(10, 0, 0.1234567891, duration("PT19H"))"#;
  let evaluator = build_evaluator(&rheon_feel_parser::parse_textual_expression(&scope, text, false).unwrap());
  let (value, diagnostics) = rheon_feel::diagnostics::collect(|| evaluator(&scope));
  assert_eq!("null(time: offset is out of range: PT19H)", value.to_string());
  assert!(diagnostics.warnings().is_empty(), "{:?}", diagnostics.warnings());
}
