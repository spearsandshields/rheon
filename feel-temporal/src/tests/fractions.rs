//! Fractional seconds of time, date and time and days and time duration texts are read exactly:
//! the nanoseconds are the integer of the first nine fraction digits.

use crate::{FeelDateTime, FeelDaysAndTimeDuration, FeelTime};

/// The canonical text of `nanos` as fraction digits: nine digits without trailing zeros.
fn fraction_text(nanos: u64) -> String {
  format!("{nanos:09}").trim_end_matches('0').to_string()
}

/// The time `10:00:00.<digits>` holds the nanoseconds `nanos` and shows them canonically.
fn assert_time_fraction(digits: &str, nanos: u64) {
  let time = format!("10:00:00.{digits}").parse::<FeelTime>().unwrap_or_else(|e| panic!("{digits}: {e}"));
  assert_eq!(nanos, time.nanos(), "10:00:00.{digits}");
  let fraction = fraction_text(nanos);
  let expected = if fraction.is_empty() { "10:00:00".to_string() } else { format!("10:00:00.{fraction}") };
  assert_eq!(expected, time.to_string(), "10:00:00.{digits}");
}

#[test]
fn every_fraction_of_up_to_six_digits_is_exact() {
  for n in 0..1_000_000_u64 {
    assert_time_fraction(&format!("{n:06}"), n * 1_000);
  }
  for n in 0..10_000_u64 {
    assert_time_fraction(&format!("{n:04}"), n * 100_000);
  }
}

#[test]
fn sampled_fractions_of_seven_to_nine_digits_are_exact() {
  // a fixed linear congruential sequence: the same 100,000 values on every run
  let mut state = 0x2545_f491_4f6c_dd1d_u64;
  for i in 0..100_000_u64 {
    state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    let digits = 7 + (i % 3) as usize;
    let n = (state >> 20) % 10_u64.pow(digits as u32);
    assert_time_fraction(&format!("{n:0digits$}"), n * 10_u64.pow(9 - digits as u32));
  }
}

#[test]
fn fractions_of_the_three_texts() {
  assert_eq!(15_700_000, "10:00:00.0157".parse::<FeelTime>().unwrap().nanos());
  assert_eq!(65_000, "10:00:00.000065".parse::<FeelTime>().unwrap().nanos());
  let date_time = FeelDateTime::try_from("2020-01-01T10:00:00.0157Z").unwrap();
  assert_eq!(15_700_000, date_time.time().nanos());
  assert_eq!("2020-01-01T10:00:00.0157Z", date_time.to_string());
  let duration = FeelDaysAndTimeDuration::try_from("PT0.000015839S").unwrap();
  assert_eq!(15_839, duration.as_nanos());
  assert_eq!("PT0.000015839S", duration.to_string());
  assert_eq!(1_015_839, FeelDaysAndTimeDuration::try_from("PT0.001015839S").unwrap().as_nanos());
  assert_eq!(-1_999_999_999, FeelDaysAndTimeDuration::try_from("-PT1.999999999S").unwrap().as_nanos());
}

#[test]
fn digits_beyond_nanoseconds_are_truncated() {
  let time = "10:00:00.99999999999999999".parse::<FeelTime>().unwrap();
  assert_eq!(999_999_999, time.nanos());
  assert_eq!("10:00:00.999999999", time.to_string());
  let date_time = FeelDateTime::try_from("2020-01-01T23:59:59.99999999999999999Z").unwrap();
  assert_eq!("2020-01-01T23:59:59.999999999Z", date_time.to_string());
  let duration = FeelDaysAndTimeDuration::try_from("PT1.99999999999999999S").unwrap();
  assert_eq!("PT1.999999999S", duration.to_string());
  assert_eq!(123_456_789, "10:00:00.1234567891".parse::<FeelTime>().unwrap().nanos());
  let long = format!("10:00:00.{}", "9".repeat(100_000));
  assert_eq!(999_999_999, long.parse::<FeelTime>().unwrap().nanos());
}

#[test]
fn a_date_and_time_in_json_is_iso_8601_with_the_zone_in_brackets() {
  // DMN 1.7 Beta 1, Table 48
  let text = |s: &str| FeelDateTime::try_from(s).unwrap().json_text();
  assert_eq!("2007-12-03T10:15:30+01:00[Europe/Paris]", text("2007-12-03T10:15:30@Europe/Paris"));
  assert_eq!("2007-07-03T10:15:30+02:00[Europe/Paris]", text("2007-07-03T10:15:30@Europe/Paris"));
  assert_eq!("2007-12-03T10:15:30+01:00", text("2007-12-03T10:15:30+01:00"));
  assert_eq!("2007-12-03T10:15:30Z", text("2007-12-03T10:15:30Z"));
  assert_eq!("2007-12-03T10:15:30", text("2007-12-03T10:15:30"));
}
