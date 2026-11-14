//! Implementation of FEEL years and months duration.

use crate::errors::*;
use dsntk_common::DsntkError;
use regex::Regex;
use std::sync::LazyLock;
use std::{fmt, ops};

/// Regular expression pattern for parsing years and months duration.
const REGEX_YEARS_AND_MONTHS: &str = r#"^(?P<sign>-)?P((?P<years>[0-9]+)Y)?((?P<months>[0-9]+)M)?$"#;

/// Number of months in a year.
const MONTHS_IN_YEAR: i64 = 12;

/// Regular expression for parsing years and months duration.
static RE_YEARS_AND_MONTHS: LazyLock<Regex> = LazyLock::new(|| Regex::new(REGEX_YEARS_AND_MONTHS).unwrap());

/// Years and months duration in FEEL.
///
/// Holds the number of months in the duration, in `-(2^63 - 1)..=2^63 - 1`: the range is
/// symmetric, so the absolute value and the negation of every duration are durations too.
/// `Default` is the zero duration.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd)]
pub struct FeelYearsAndMonthsDuration(i64);

impl FeelYearsAndMonthsDuration {
  /// Creates a years and months duration of `years` years and `months` months, or `None` when it
  /// is out of range.
  pub fn from_ym(years: i64, months: i64) -> Option<Self> {
    years.checked_mul(MONTHS_IN_YEAR)?.checked_add(months).and_then(Self::from_m)
  }

  /// Creates a years and months duration of `months` months, or `None` when it is out of range
  /// (`i64::MIN`). Every duration is built here.
  pub fn from_m(months: i64) -> Option<Self> {
    (months != i64::MIN).then_some(Self(months))
  }

  /// Returns the number of years in this duration.
  pub fn years(&self) -> i64 {
    self.0 / MONTHS_IN_YEAR
  }

  /// Returns the number of months in this duration.
  pub fn months(&self) -> i64 {
    self.0 % MONTHS_IN_YEAR
  }

  /// Returns the total number of months of this duration.
  pub fn as_months(&self) -> i64 {
    self.0
  }

  /// Returns absolute value of the duration.
  pub fn abs(&self) -> Self {
    Self(self.0.abs())
  }

  /// Returns `true` when duration is negative.
  pub fn is_negative(&self) -> bool {
    self.0 < 0
  }

  /// Returns the sum of durations, or `None` when the result is out of range.
  pub fn checked_add(&self, rhs: &Self) -> Option<Self> {
    self.0.checked_add(rhs.0).and_then(Self::from_m)
  }

  /// Returns the difference of durations, or `None` when the result is out of range.
  pub fn checked_sub(&self, rhs: &Self) -> Option<Self> {
    self.0.checked_sub(rhs.0).and_then(Self::from_m)
  }
}

impl ops::Neg for FeelYearsAndMonthsDuration {
  type Output = Self;
  /// Returns the arithmetic negation of this duration.
  fn neg(self) -> Self {
    Self(-self.0)
  }
}

impl fmt::Display for FeelYearsAndMonthsDuration {
  /// Converts [FeelYearsAndMonthsDuration] into string.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let sign = if self.0 < 0 { "-" } else { "" };
    let mut month = self.0.unsigned_abs();
    let year = month / MONTHS_IN_YEAR as u64;
    month -= year * MONTHS_IN_YEAR as u64;
    match (year > 0, month > 0) {
      (false, false) => write!(f, "P0M"),
      (false, true) => write!(f, "{sign}P{month}M"),
      (true, false) => write!(f, "{sign}P{year}Y"),
      (true, true) => write!(f, "{sign}P{year}Y{month}M"),
    }
  }
}

impl TryFrom<&str> for FeelYearsAndMonthsDuration {
  type Error = DsntkError;
  /// Converts a text into [FeelYearsAndMonthsDuration].
  fn try_from(value: &str) -> Result<Self, Self::Error> {
    // The literal is valid when it has a component and every component it has fits: a component
    // that does not fit makes the whole literal invalid, it is never left out.
    if let Some(captures) = RE_YEARS_AND_MONTHS.captures(value)
      && (captures.name("years").is_some() || captures.name("months").is_some())
    {
      let months_of = |name: &str, months_per_unit: i64| match captures.name(name) {
        Some(component) => component.as_str().parse::<i64>().ok()?.checked_mul(months_per_unit),
        None => Some(0),
      };
      if let Some(total_months) = months_of("years", MONTHS_IN_YEAR)
        .zip(months_of("months", 1))
        .and_then(|(years, months)| years.checked_add(months))
        && let Some(duration) = Self::from_m(if captures.name("sign").is_some() { -total_months } else { total_months })
      {
        return Ok(duration);
      }
    }
    Err(err_invalid_years_and_months_duration_literal(value))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Utility function for testing years and months durations equality.
  fn equals(months: i64, text: &str) {
    let expected = FeelYearsAndMonthsDuration(months);
    let actual = FeelYearsAndMonthsDuration::try_from(text).unwrap();
    assert_eq!(expected, actual);
  }

  /// Utility function for testing invalid years and months durations.
  fn invalid(text: &str) {
    let actual = FeelYearsAndMonthsDuration::try_from(text);
    assert!(actual.is_err());
  }

  /// Utility function for testing equality of textual forms of years and months durations.
  fn equals_str(expected: &str, years: i64, months: i64) {
    let actual: String = FeelYearsAndMonthsDuration::from_ym(years, months).unwrap().to_string();
    assert_eq!(expected, actual);
  }

  #[test]
  fn test_parsing_from_string_should_pass() {
    equals(1, "P1M");
    equals(-1, "-P1M");
    equals(12, "P1Y");
    equals(-12, "-P1Y");
    equals(15, "P1Y3M");
    equals(-15, "-P1Y3M");
    equals(999999999 * MONTHS_IN_YEAR, "P999999999Y");
    equals(-999999999 * MONTHS_IN_YEAR, "-P999999999Y");
  }

  #[test]
  fn test_parsing_from_string_should_fail() {
    invalid("P");
    invalid("-P");
    invalid("K1Y1M");
    invalid("-K1Y1M");
    invalid("P1M1Y");
    invalid("-P1M1Y");
    invalid("P10");
    invalid("-P10");
    invalid("PY");
    invalid("-PY");
    invalid("PM");
    invalid("-PM");
    invalid("P1Y3MT1D");
    invalid("-P1Y3MT1D");
    invalid("P768614336404564650Y8M");
    invalid("-P768614336404564650Y8M");
    invalid("P99999999999999999999Y3M");
    invalid("P1Y99999999999999999999M");
    invalid("P9223372036854775808M");
  }

  #[test]
  fn test_converting_to_string_should_pass() {
    equals_str("P0M", 0, 0);
    equals_str("P1M", 0, 1);
    equals_str("-P1M", 0, -1);
    equals_str("P1Y", 1, 0);
    equals_str("-P1Y", -1, 0);
    equals_str("P2Y3M", 2, 3);
    equals_str("-P2Y3M", -2, -3);
  }

  #[test]
  fn test_eq_should_pass() {
    let a = FeelYearsAndMonthsDuration::from_ym(0, 0).unwrap();
    let b = FeelYearsAndMonthsDuration::from_ym(0, 0).unwrap();
    assert!((a == b));
    let a = FeelYearsAndMonthsDuration::from_ym(2, 3).unwrap();
    let b = FeelYearsAndMonthsDuration::from_ym(2, 3).unwrap();
    assert!((a == b));
    let a = FeelYearsAndMonthsDuration::from_ym(-3, 4).unwrap();
    let b = FeelYearsAndMonthsDuration::from_ym(-3, 4).unwrap();
    assert!((a == b));
    let a = FeelYearsAndMonthsDuration::from_m(0).unwrap();
    let b = FeelYearsAndMonthsDuration::from_m(0).unwrap();
    assert!((a == b));
    let a = FeelYearsAndMonthsDuration::from_m(-0).unwrap();
    let b = FeelYearsAndMonthsDuration::from_m(0).unwrap();
    assert!((a == b));
    let a = FeelYearsAndMonthsDuration::from_m(-15).unwrap();
    let b = FeelYearsAndMonthsDuration::from_m(-15).unwrap();
    assert!((a == b));
  }

  #[test]
  fn test_ne_should_pass() {
    let a = FeelYearsAndMonthsDuration::from_ym(0, 0).unwrap();
    let b = FeelYearsAndMonthsDuration::from_ym(0, 1).unwrap();
    assert!((a != b));
    let a = FeelYearsAndMonthsDuration::from_m(1).unwrap();
    let b = FeelYearsAndMonthsDuration::from_m(0).unwrap();
    assert!((a != b));
  }

  #[test]
  fn test_lt_should_pass() {
    let a = FeelYearsAndMonthsDuration::from_ym(0, 0).unwrap();
    let b = FeelYearsAndMonthsDuration::from_ym(0, 1).unwrap();
    assert!(a < b);
    let a = FeelYearsAndMonthsDuration::from_m(4).unwrap();
    let b = FeelYearsAndMonthsDuration::from_m(5).unwrap();
    assert!(a < b);
  }

  #[test]
  fn test_le_should_pass() {
    let a = FeelYearsAndMonthsDuration::from_ym(0, 0).unwrap();
    let b = FeelYearsAndMonthsDuration::from_ym(0, 1).unwrap();
    assert!(a <= b);
    let a = FeelYearsAndMonthsDuration::from_ym(0, 1).unwrap();
    let b = FeelYearsAndMonthsDuration::from_ym(0, 1).unwrap();
    assert!(a <= b);
    let a = FeelYearsAndMonthsDuration::from_m(12).unwrap();
    let b = FeelYearsAndMonthsDuration::from_m(18).unwrap();
    assert!(a <= b);
    let a = FeelYearsAndMonthsDuration::from_m(16).unwrap();
    let b = FeelYearsAndMonthsDuration::from_m(16).unwrap();
    assert!(a <= b);
  }

  #[test]
  fn test_gt_should_pass() {
    let a = FeelYearsAndMonthsDuration::from_ym(0, 1).unwrap();
    let b = FeelYearsAndMonthsDuration::from_ym(0, 0).unwrap();
    assert!(a > b);
    let a = FeelYearsAndMonthsDuration::from_m(5).unwrap();
    let b = FeelYearsAndMonthsDuration::from_m(4).unwrap();
    assert!(a > b);
  }

  #[test]
  fn test_ge_should_pass() {
    let a = FeelYearsAndMonthsDuration::from_ym(0, 1).unwrap();
    let b = FeelYearsAndMonthsDuration::from_ym(0, 0).unwrap();
    assert!(a >= b);
    let a = FeelYearsAndMonthsDuration::from_ym(0, 1).unwrap();
    let b = FeelYearsAndMonthsDuration::from_ym(0, 1).unwrap();
    assert!(a >= b);
    let a = FeelYearsAndMonthsDuration::from_m(18).unwrap();
    let b = FeelYearsAndMonthsDuration::from_m(12).unwrap();
    assert!(a >= b);
    let a = FeelYearsAndMonthsDuration::from_m(16).unwrap();
    let b = FeelYearsAndMonthsDuration::from_m(16).unwrap();
    assert!(a >= b);
  }

  #[test]
  fn test_abs_should_pass() {
    let ym_duration = FeelYearsAndMonthsDuration::try_from("P2Y3M").unwrap();
    assert_eq!("P2Y3M", ym_duration.abs().to_string());
    let duration = FeelYearsAndMonthsDuration::try_from("-P2Y3M").unwrap();
    assert_eq!("P2Y3M", duration.abs().to_string());
  }

  #[test]
  fn test_properties() {
    let ym_duration = FeelYearsAndMonthsDuration::try_from("P2Y3M").unwrap();
    assert_eq!(2, ym_duration.years());
    assert_eq!(3, ym_duration.months());
    assert_eq!(27, ym_duration.as_months());
  }

  #[test]
  fn test_symmetric_range() {
    let max = FeelYearsAndMonthsDuration::from_m(i64::MAX).unwrap();
    let min = FeelYearsAndMonthsDuration::from_m(-i64::MAX).unwrap();
    assert_eq!(None, FeelYearsAndMonthsDuration::from_m(i64::MIN));
    assert_eq!(None, FeelYearsAndMonthsDuration::from_ym(-768614336404564650, -8));
    assert_eq!(Some(min.clone()), FeelYearsAndMonthsDuration::from_ym(-768614336404564650, -7));
    assert_eq!(None, FeelYearsAndMonthsDuration::from_ym(i64::MAX, 0));
    let one = FeelYearsAndMonthsDuration::from_m(1).unwrap();
    assert_eq!(None, min.checked_sub(&one));
    assert_eq!(None, min.checked_add(&-one.clone()));
    assert_eq!(None, max.checked_add(&one));
    assert_eq!("P768614336404564650Y7M", max.to_string());
    assert_eq!("-P768614336404564650Y7M", min.to_string());
    assert_eq!(max, min.abs());
    assert_eq!(max, -min.clone());
    assert_eq!(min, -max.clone());
    assert_eq!("P0M", FeelYearsAndMonthsDuration::default().to_string());
  }
}
