//! Implementation of FEEL date.

use crate::FeelDaysAndTimeDuration;
use crate::defs::*;
use crate::errors::*;
use crate::feel_dt_duration::NANOSECONDS_IN_DAY;
use crate::feel_ym_duration::FeelYearsAndMonthsDuration;
use chrono::{DateTime, Datelike, Days, FixedOffset, LocalResult, Months, NaiveDate, TimeZone, Weekday};
use dsntk_common::DsntkError;
use rheon_feel_number::FeelNumber;
use std::cmp::Ordering;
use std::fmt;
use std::fmt::Display;
use std::ops::{Add, Sub};
use std::str::FromStr;

/// FEEL date.
#[derive(Debug, Clone)]
pub struct FeelDate(Year, Month, Day);

impl Display for FeelDate {
  /// Converts [FeelDate] into [String].
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{:04}-{:02}-{:02}", self.0, self.1, self.2)
  }
}

impl FromStr for FeelDate {
  type Err = DsntkError;
  /// Converts [String] into [FeelDate].
  fn from_str(date: &str) -> Result<Self, Self::Err> {
    let captures = RE_DATE.captures(date).ok_or_else(|| err_invalid_date_literal(date))?;
    if let Some(year_match) = captures.name("year")
      && let Ok(mut year) = year_match.as_str().parse::<Year>()
    {
      if captures.name("sign").is_some() {
        year = -year;
      }
      if let Some(month_match) = captures.name("month")
        && let Ok(month) = month_match.as_str().parse::<Month>()
        && let Some(day_match) = captures.name("day")
        && let Ok(day) = day_match.as_str().parse::<Day>()
        && is_valid_date(year, month, day)
      {
        return Ok(FeelDate(year, month, day));
      }
    }
    Err(err_invalid_date_literal(date))
  }
}

impl TryFrom<(FeelNumber, FeelNumber, FeelNumber)> for FeelDate {
  type Error = DsntkError;
  /// Converts a tuple of numbers into [FeelDate].
  fn try_from(value: (FeelNumber, FeelNumber, FeelNumber)) -> Result<Self, Self::Error> {
    let year = value.0.try_into()?;
    if value.1 > FeelNumber::zero() && value.2 > FeelNumber::zero() {
      let month = value.1.try_into()?;
      let day = value.2.try_into()?;
      if is_valid_date(year, month, day) {
        return Ok(Self(year, month, day));
      }
    }
    Err(err_invalid_date(value.0, value.1, value.2))
  }
}

impl PartialEq for FeelDate {
  fn eq(&self, other: &Self) -> bool {
    self.0 == other.0 && self.1 == other.1 && self.2 == other.2
  }
}

impl Eq for FeelDate {}

impl PartialOrd for FeelDate {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for FeelDate {
  fn cmp(&self, other: &Self) -> Ordering {
    let y = self.0.cmp(&other.0);
    let m = self.1.cmp(&other.1);
    let d = self.2.cmp(&other.2);
    match (y, m, d) {
      (Ordering::Equal, Ordering::Equal, Ordering::Equal) => Ordering::Equal,
      (Ordering::Equal, Ordering::Equal, Ordering::Less) => Ordering::Less,
      (Ordering::Equal, Ordering::Equal, Ordering::Greater) => Ordering::Greater,
      (Ordering::Equal, Ordering::Less, _) => Ordering::Less,
      (Ordering::Equal, Ordering::Greater, _) => Ordering::Greater,
      (Ordering::Less, _, _) => Ordering::Less,
      (Ordering::Greater, _, _) => Ordering::Greater,
    }
  }
}

impl Sub<&FeelDate> for &FeelDate {
  type Output = Option<FeelYearsAndMonthsDuration>;
  /// Subtracts two [FeelDates](FeelDate), the result is [FeelYearsAndMonthsDuration], or `None`
  /// when it is out of range.
  fn sub(self, other: &FeelDate) -> Self::Output {
    let mut months;
    if self.0 < other.0 {
      months = 12 * (other.0 as i64 - self.0 as i64) + (other.1 as i64 - self.1 as i64);
      if self.2 > other.2 {
        months -= 1;
      }
      months *= -1;
    } else {
      months = 12 * (self.0 as i64 - other.0 as i64) + (self.1 as i64 - other.1 as i64);
      if other.2 > self.2 {
        months -= 1;
      }
    }
    FeelYearsAndMonthsDuration::from_m(months)
  }
}

impl Add<FeelYearsAndMonthsDuration> for FeelDate {
  type Output = Option<Self>;
  /// Adds [FeelYearsAndMonthsDuration] to [FeelDate]; the day of the month is kept, or the last
  /// day of a shorter month is taken.
  fn add(self, other: FeelYearsAndMonthsDuration) -> Self::Output {
    self.add_months(other.as_months())
  }
}

impl Sub<FeelYearsAndMonthsDuration> for FeelDate {
  type Output = Option<Self>;
  /// Subtracts [FeelYearsAndMonthsDuration] from [FeelDate]; the day of the month is kept, or the
  /// last day of a shorter month is taken.
  fn sub(self, other: FeelYearsAndMonthsDuration) -> Self::Output {
    self.add_months((-other).as_months())
  }
}

impl Add<FeelDaysAndTimeDuration> for FeelDate {
  type Output = Option<FeelDate>;
  /// Adds [FeelDaysAndTimeDuration] to [FeelDate] (DMN 1.6 Table 58).
  fn add(self, rhs: FeelDaysAndTimeDuration) -> Self::Output {
    self.add_nanos(rhs.as_nanos())
  }
}

impl Sub<FeelDaysAndTimeDuration> for FeelDate {
  type Output = Option<FeelDate>;
  /// Subtracts [FeelDaysAndTimeDuration] from [FeelDate] (DMN 1.6 Table 58).
  fn sub(self, rhs: FeelDaysAndTimeDuration) -> Self::Output {
    self.add_nanos((-rhs).as_nanos())
  }
}

impl TryFrom<FeelDate> for DateTime<FixedOffset> {
  type Error = DsntkError;
  /// Converts [FeelDate] date into [DateTime] in UTC zone.
  fn try_from(date: FeelDate) -> Result<Self, Self::Error> {
    if let Some(fixed_offset) = FixedOffset::east_opt(0)
      && let LocalResult::Single(date_time) = fixed_offset.with_ymd_and_hms(date.0, date.1, date.2, 0, 0, 0)
    {
      return Ok(date_time);
    }
    Err(err_invalid_feel_date(date))
  }
}

impl TryFrom<FeelDate> for NaiveDate {
  type Error = DsntkError;
  /// Converts [FeelDate] date into [NaiveDate].
  fn try_from(date: FeelDate) -> Result<Self, Self::Error> {
    NaiveDate::from_ymd_opt(date.0, date.1, date.2).ok_or_else(|| err_invalid_feel_date(date))
  }
}

impl FeelDate {
  /// Creates a [FeelDate] from provided `year`, `month` and `day` values.
  pub fn new(year: Year, month: Month, day: Day) -> Option<Self> {
    if is_valid_date(year, month, day) { Some(Self(year, month, day)) } else { None }
  }

  /// Returns today's date in local time.
  pub fn today() -> Self {
    let today = local_now();
    Self(today.year(), today.month(), today.day())
  }

  pub fn year(&self) -> Year {
    self.0
  }

  pub fn month(&self) -> Month {
    self.1
  }

  pub fn day(&self) -> Day {
    self.2
  }

  pub fn day_of_week(&self) -> Option<DayOfWeek> {
    NaiveDate::from_ymd_opt(self.0, self.1, self.2).map(|naive_date| match naive_date.weekday() {
      Weekday::Mon => ("Monday".to_string(), 1_u8),
      Weekday::Tue => ("Tuesday".to_string(), 2_u8),
      Weekday::Wed => ("Wednesday".to_string(), 3_u8),
      Weekday::Thu => ("Thursday".to_string(), 4_u8),
      Weekday::Fri => ("Friday".to_string(), 5_u8),
      Weekday::Sat => ("Saturday".to_string(), 6_u8),
      Weekday::Sun => ("Sunday".to_string(), 7_u8),
    })
  }

  pub fn day_of_year(&self) -> Option<DayOfYear> {
    NaiveDate::from_ymd_opt(self.0, self.1, self.2).map(|naive_date| naive_date.ordinal() as u16)
  }

  pub fn week_of_year(&self) -> Option<WeekOfYear> {
    NaiveDate::from_ymd_opt(self.0, self.1, self.2).map(|naive_date| naive_date.iso_week().week() as u8)
  }

  pub fn month_of_year(&self) -> Option<MonthOfYear> {
    if let Some(naive_date) = NaiveDate::from_ymd_opt(self.0, self.1, self.2) {
      month_of_year(naive_date.month())
    } else {
      None
    }
  }

  pub fn as_tuple(&self) -> (Year, Month, Day) {
    (self.0, self.1, self.2)
  }

  pub fn add_days(&self, days: u64) -> Option<Self> {
    if let Some(naive_date) = NaiveDate::from_ymd_opt(self.0, self.1, self.2)
      && let Some(updated_date) = naive_date.checked_add_days(Days::new(days))
    {
      return Some(Self(updated_date.year(), updated_date.month(), updated_date.day()));
    }
    None
  }

  pub fn sub_days(&self, days: u64) -> Option<Self> {
    if let Some(naive_date) = NaiveDate::from_ymd_opt(self.0, self.1, self.2)
      && let Some(updated_date) = naive_date.checked_sub_days(Days::new(days))
    {
      return Some(Self(updated_date.year(), updated_date.month(), updated_date.day()));
    }
    None
  }

  /// Returns the date of the instant `nanos` nanoseconds after (or before) this date, or `None`
  /// when it is out of range. DMN 1.6 Table 58 defines a date plus or minus a days and time
  /// duration as `date(valuedt⁻¹(valuedt(e1) ± valuedtd(e2)))`, a date being the date and time at
  /// UTC midnight (§10.3.2.3.5), so the nanoseconds are rounded down to whole days:
  /// `2020-01-02` minus `PT0.5S` is `2020-01-01`.
  fn add_nanos(&self, nanos: i64) -> Option<Self> {
    let days = nanos.div_euclid(NANOSECONDS_IN_DAY);
    let naive_date = NaiveDate::from_ymd_opt(self.0, self.1, self.2)?;
    let count = Days::new(days.unsigned_abs());
    let updated_date = if days < 0 {
      naive_date.checked_sub_days(count)
    } else {
      naive_date.checked_add_days(count)
    }?;
    Some(Self(updated_date.year(), updated_date.month(), updated_date.day()))
  }

  /// Returns this date moved by `months` months, forward or back, or `None` when the result is
  /// out of range.
  ///
  /// DMN 1.6 §9.4 gives a date plus or minus a years and months duration the semantics of
  /// op:add-yearMonthDuration-to-date and op:subtract-yearMonthDuration-from-date, which keep the
  /// day of the month and take the last day of the resulting month when it has fewer days (XML
  /// Schema Part 2, Appendix E): `2020-03-31` minus one month is `2020-02-29`. Table 58 writes the
  /// step as `date(e1.year +/- e2.years + floor((e1.month +/- e2.months)/12), e1.month +/-
  /// e2.months - floor((e1.month +/- e2.months)/12) * 12, e1.day)`, which has no value for such a
  /// day; the month-end date of §9.4 is the result here.
  pub(crate) fn add_months(&self, months: i64) -> Option<Self> {
    let naive_date = NaiveDate::from_ymd_opt(self.0, self.1, self.2)?;
    let count = Months::new(u32::try_from(months.unsigned_abs()).ok()?);
    let updated_date = if months < 0 {
      naive_date.checked_sub_months(count)
    } else {
      naive_date.checked_add_months(count)
    }?;
    Some(Self(updated_date.year(), updated_date.month(), updated_date.day()))
  }
}

/// Returns `true` when specified `year`, `month` and `day` form a valid date.
pub fn is_valid_date(year: Year, month: Month, day: Day) -> bool {
  NaiveDate::from_ymd_opt(year, month, day).is_some()
}

#[cfg(test)]
mod tests {
  use super::*;
  use dsntk_common::Result;

  #[test]
  fn test_display() {
    assert_eq!("9999-01-01", format!("{}", FeelDate(9999, 1, 1)));
    assert_eq!("-9999-01-01", format!("{}", FeelDate(-9999, 1, 1)));
    assert_eq!("999999999-01-01", format!("{}", FeelDate(999_999_999, 1, 1)));
    assert_eq!("-999999999-01-01", format!("{}", FeelDate(-999_999_999, 1, 1)));
  }

  #[test]
  fn test_add_months() {
    assert_eq!("9999-02-01", format!("{}", FeelDate(9999, 1, 1).add_months(1).unwrap()));
    assert_eq!("99999-02-01", format!("{}", FeelDate(99999, 1, 1).add_months(1).unwrap()));
    assert_eq!("99999-12-01", format!("{}", FeelDate(99999, 1, 1).add_months(11).unwrap()));
    assert_eq!("100000-01-01", format!("{}", FeelDate(99999, 1, 1).add_months(12).unwrap()));
    assert_eq!("199999-02-01", format!("{}", FeelDate(199999, 1, 1).add_months(1).unwrap()));
    assert_eq!("262142-12-01", format!("{}", FeelDate(262142, 11, 1).add_months(1).unwrap()));
    assert_eq!("+262142-12-31", NaiveDate::MAX.to_string());
    assert_eq!("2020-01-31", format!("{}", FeelDate(2020, 1, 31).add_months(0).unwrap()));
    assert_eq!("2020-02-29", format!("{}", FeelDate(2020, 1, 31).add_months(1).unwrap()));
    assert_eq!("2021-02-28", format!("{}", FeelDate(2020, 3, 31).add_months(11).unwrap()));
    assert_eq!(None, FeelDate(262142, 12, 1).add_months(1));
    assert_eq!(None, FeelDate(2020, 1, 1).add_months(i64::MAX));
  }

  #[test]
  fn test_add_negative_months() {
    assert_eq!("-9999-11-01", format!("{}", FeelDate(-9999, 12, 1).add_months(-1).unwrap()));
    assert_eq!("-99999-02-01", format!("{}", FeelDate(-99999, 3, 1).add_months(-1).unwrap()));
    assert_eq!("-99999-12-01", format!("{}", FeelDate(-99998, 1, 1).add_months(-1).unwrap()));
    assert_eq!("-100000-01-01", format!("{}", FeelDate(-100000, 12, 1).add_months(-11).unwrap()));
    assert_eq!("-199999-11-01", format!("{}", FeelDate(-199999, 12, 1).add_months(-1).unwrap()));
    assert_eq!("-262143-01-01", format!("{}", FeelDate(-262143, 2, 1).add_months(-1).unwrap()));
    assert_eq!("-262143-01-01", NaiveDate::MIN.to_string());
    assert_eq!("2020-02-29", format!("{}", FeelDate(2020, 3, 31).add_months(-1).unwrap()));
    assert_eq!("2019-02-28", format!("{}", FeelDate(2020, 1, 31).add_months(-11).unwrap()));
    assert_eq!(None, FeelDate(-262143, 1, 1).add_months(-1));
    assert_eq!(None, FeelDate(2020, 1, 1).add_months(-i64::MAX));
  }

  #[test]
  fn test_is_valid_date() {
    assert!(!is_valid_date(999_999_999, 12, 13));
    assert!(!is_valid_date(1_000_000_000, 1, 1));
    assert!(!is_valid_date(-999_999_999, 1, 1));
    assert!(!is_valid_date(-1_000_000_000, 12, 31));
    assert!(!is_valid_date(2021, 2, 29));
    assert!(is_valid_date(262142, 12, 31));
    assert!(!is_valid_date(262143, 1, 1));
    assert!(is_valid_date(-262143, 1, 1));
    assert!(!is_valid_date(-262144, 12, 31));
  }

  #[allow(unused_parens)]
  #[test]
  fn test_eq() {
    assert!((FeelDate(2021, 2, 1) == FeelDate(2021, 2, 1)));
    assert!((FeelDate(2021, 2, 1) != FeelDate(2021, 2, 2)));
    assert!((FeelDate(2021, 2, 1) != FeelDate(2021, 3, 1)));
    assert!((FeelDate(2021, 2, 1) != FeelDate(2022, 2, 1)));
    assert!((FeelDate(999_999_999, 12, 31) != FeelDate(-999_999_999, 1, 1)));
  }

  #[allow(unused_parens)]
  #[test]
  fn test_compare() {
    assert!((FeelDate(2021, 2, 1) == FeelDate(2021, 2, 1)));
    assert!(FeelDate(2021, 2, 1) < FeelDate(2021, 2, 2));
    assert!(FeelDate(2021, 2, 1) < FeelDate(2021, 3, 1));
    assert!(FeelDate(2021, 2, 5) < FeelDate(2022, 2, 5));
    assert!(FeelDate(2021, 2, 2) > FeelDate(2021, 2, 1));
    assert!(FeelDate(2021, 3, 1) > FeelDate(2021, 2, 1));
    assert!(FeelDate(2022, 2, 1) > FeelDate(2021, 2, 1));
    assert!(FeelDate(2021, 2, 1) >= FeelDate(2021, 2, 1));
    assert!(FeelDate(2021, 2, 2) >= FeelDate(2021, 2, 1));
    assert!(FeelDate(2021, 2, 1) <= FeelDate(2021, 2, 1));
    assert!(FeelDate(2021, 2, 1) <= FeelDate(2021, 2, 2));
  }

  #[test]
  fn test_conversion() {
    let date = FeelDate(2023, 2, 8);
    let date_time: DateTime<FixedOffset> = date.try_into().unwrap();
    assert_eq!("2023-02-08 00:00:00 +00:00", date_time.to_string());
    let date = FeelDate(262144, 1, 1);
    let date_time: Result<DateTime<FixedOffset>> = date.try_into();
    assert_eq!("<TemporalError> invalid date 262144-1-1", date_time.err().unwrap().to_string());
    let date = FeelDate(-262144, 12, 31);
    let date_time: Result<NaiveDate> = date.try_into();
    assert_eq!("<TemporalError> invalid date -262144-12-31", date_time.err().unwrap().to_string());
  }
}
