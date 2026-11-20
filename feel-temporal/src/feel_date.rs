//! Implementation of FEEL date.

use crate::FeelDaysAndTimeDuration;
use crate::defs::*;
use crate::errors::*;
use crate::feel_dt_duration::NANOSECONDS_IN_DAY;
use crate::feel_ym_duration::FeelYearsAndMonthsDuration;
use crate::parts::split_date;
use chrono::{DateTime, Datelike, FixedOffset, LocalResult, NaiveDate, TimeZone};
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
    let captures = split_date(date).ok_or_else(|| err_invalid_date_literal(date))?;
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
  /// when it is out of range. The whole months from the earlier date to the later one: the
  /// difference of their months, one fewer when the day of the later date is below the day of
  /// the earlier one; negative when `self` is the earlier date, so `a - b` is `-(b - a)`.
  fn sub(self, other: &FeelDate) -> Self::Output {
    let (earlier, later) = if self < other { (self, other) } else { (other, self) };
    let mut months = 12 * (later.0 as i64 - earlier.0 as i64) + (later.1 as i64 - earlier.1 as i64);
    if later.2 < earlier.2 {
      months -= 1;
    }
    FeelYearsAndMonthsDuration::from_m(if self < other { -months } else { months })
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
    // the day 0 is a Thursday
    let (name, number) = match (self.day_number()? + 3).rem_euclid(7) {
      0 => ("Monday", 1_u8),
      1 => ("Tuesday", 2),
      2 => ("Wednesday", 3),
      3 => ("Thursday", 4),
      4 => ("Friday", 5),
      5 => ("Saturday", 6),
      _ => ("Sunday", 7),
    };
    Some((name.to_string(), number))
  }

  pub fn day_of_year(&self) -> Option<DayOfYear> {
    Some((self.day_number()? - day_number(self.0, 1, 1) + 1) as u16)
  }

  /// The week of the year of ISO 8601: the week of a date is that of its Thursday.
  pub fn week_of_year(&self) -> Option<WeekOfYear> {
    let (ordinal, weekday) = (i64::from(self.day_of_year()?), i64::from(self.day_of_week()?.1));
    // the weeks of a year: 53 when it starts on a Thursday, or on a Wednesday when it is a leap year
    let weeks = |year: Year| {
      let first = (day_number(year, 1, 1) + 3).rem_euclid(7);
      if first == 3 || (first == 2 && is_leap_year(year)) { 53_i64 } else { 52 }
    };
    let week = (ordinal - weekday + 10) / 7;
    let week = if week < 1 {
      weeks(self.0 - 1)
    } else if week > weeks(self.0) {
      1
    } else {
      week
    };
    Some(week as u8)
  }

  pub fn month_of_year(&self) -> Option<MonthOfYear> {
    if is_valid_date(self.0, self.1, self.2) { month_of_year(self.1) } else { None }
  }

  pub fn as_tuple(&self) -> (Year, Month, Day) {
    (self.0, self.1, self.2)
  }

  pub fn add_days(&self, days: u64) -> Option<Self> {
    self.moved_by_days(i64::try_from(days).ok()?)
  }

  pub fn sub_days(&self, days: u64) -> Option<Self> {
    self.moved_by_days(i64::try_from(days).ok()?.checked_neg()?)
  }

  /// Returns the date of the instant `nanos` nanoseconds after (or before) this date, or `None`
  /// when it is out of range. DMN 1.6 Table 58 defines a date plus or minus a days and time
  /// duration as `date(valuedt⁻¹(valuedt(e1) ± valuedtd(e2)))`, a date being the date and time at
  /// UTC midnight (§10.3.2.3.5), so the nanoseconds are rounded down to whole days:
  /// `2020-01-02` minus `PT0.5S` is `2020-01-01`.
  fn add_nanos(&self, nanos: i64) -> Option<Self> {
    self.moved_by_days(nanos.div_euclid(NANOSECONDS_IN_DAY))
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
    if !is_valid_date(self.0, self.1, self.2) {
      return None;
    }
    // the months since the year 0
    let total = (i64::from(self.0) * 12 + i64::from(self.1) - 1).checked_add(months)?;
    let year = Year::try_from(total.div_euclid(12)).ok()?;
    let month = total.rem_euclid(12) as Month + 1;
    Self::new(year, month, self.2.min(days_in_month(year, month)))
  }

  /// The time from `other` to this date, both at midnight: whole days, negative when this date
  /// is the earlier one; `None` when either is no date or the time is beyond a duration.
  pub fn time_since(&self, other: &FeelDate) -> Option<FeelDaysAndTimeDuration> {
    let days = self.day_number()? - other.day_number()?;
    FeelDaysAndTimeDuration::from_n(days.checked_mul(NANOSECONDS_IN_DAY)?)
  }

  /// The number of days from 1970-01-01 to this date; `None` when it is no date.
  fn day_number(&self) -> Option<i64> {
    is_valid_date(self.0, self.1, self.2).then(|| day_number(self.0, self.1, self.2))
  }

  /// The date `days` days after this one, or before it; `None` when it is out of range.
  fn moved_by_days(&self, days: i64) -> Option<Self> {
    let (year, month, day) = date_of_day_number(self.day_number()?.checked_add(days)?)?;
    Self::new(year, month, day)
  }
}

/// The first and the last year of FEEL (DMN 1.6, 10.3.4.1, `date(year, month, day)`).
const YEARS: std::ops::RangeInclusive<Year> = -999_999_999..=999_999_999;

/// Whether `year` is a leap year of the proleptic Gregorian calendar.
fn is_leap_year(year: Year) -> bool {
  year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// The number of days of the month `month` (1 to 12) of `year`.
fn days_in_month(year: Year, month: Month) -> Day {
  match month {
    4 | 6 | 9 | 11 => 30,
    2 if is_leap_year(year) => 29,
    2 => 28,
    _ => 31,
  }
}

/// The number of days from 1970-01-01 to a date of the proleptic Gregorian calendar, by eras of
/// 400 years (146,097 days) whose years begin in March, so that the leap day is the last day.
fn day_number(year: Year, month: Month, day: Day) -> i64 {
  let (year, month, day) = (i64::from(year) - i64::from(month <= 2), i64::from(month), i64::from(day));
  let era = year.div_euclid(400);
  let year_of_era = year.rem_euclid(400);
  let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
  let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
  era * 146_097 + day_of_era - 719_468
}

/// The date of a number of days from 1970-01-01 ([day_number]); `None` when its year is beyond
/// those of a [Year].
fn date_of_day_number(days: i64) -> Option<(Year, Month, Day)> {
  let days = days.checked_add(719_468)?;
  let era = days.div_euclid(146_097);
  let day_of_era = days.rem_euclid(146_097);
  let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
  let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
  // the month of a year that begins in March
  let shifted = (5 * day_of_year + 2) / 153;
  let day = day_of_year - (153 * shifted + 2) / 5 + 1;
  let month = if shifted < 10 { shifted + 3 } else { shifted - 9 };
  let year = era.checked_mul(400)?.checked_add(year_of_era + i64::from(month <= 2))?;
  Some((Year::try_from(year).ok()?, month as Month, day as Day))
}

/// Returns `true` when specified `year`, `month` and `day` form a valid date: a day of the
/// proleptic Gregorian calendar in the years of FEEL.
pub fn is_valid_date(year: Year, month: Month, day: Day) -> bool {
  YEARS.contains(&year) && (1..=12).contains(&month) && (1..=days_in_month(year, month)).contains(&day)
}

#[cfg(test)]
mod tests {
  use super::*;
  use chrono::{Days, Months, Weekday};
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
    assert_eq!("262143-01-01", format!("{}", FeelDate(262142, 12, 1).add_months(1).unwrap()));
    assert_eq!("999999999-12-31", format!("{}", FeelDate(999_999_999, 11, 30).add_months(1).unwrap().add_days(1).unwrap()));
    assert_eq!(None, FeelDate(999_999_999, 12, 1).add_months(1));
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
    assert_eq!("-262144-12-01", format!("{}", FeelDate(-262143, 1, 1).add_months(-1).unwrap()));
    assert_eq!(None, FeelDate(-999_999_999, 1, 1).add_months(-1));
    assert_eq!(None, FeelDate(2020, 1, 1).add_months(-i64::MAX));
  }

  #[test]
  fn test_is_valid_date() {
    // the years of FEEL (DMN 1.6, 10.3.4.1, date(year, month, day)): -999,999,999 to 999,999,999
    assert!(is_valid_date(999_999_999, 12, 31));
    assert!(!is_valid_date(1_000_000_000, 1, 1));
    assert!(is_valid_date(-999_999_999, 1, 1));
    assert!(!is_valid_date(-1_000_000_000, 12, 31));
    assert!(!is_valid_date(2021, 2, 29));
    assert!(is_valid_date(2020, 2, 29));
    assert!(!is_valid_date(1900, 2, 29));
    assert!(is_valid_date(2000, 2, 29));
    assert!(is_valid_date(262143, 1, 1));
    assert!(is_valid_date(-262144, 12, 31));
    assert!(!is_valid_date(2020, 0, 1) && !is_valid_date(2020, 13, 1) && !is_valid_date(2020, 1, 0) && !is_valid_date(2020, 4, 31));
  }

  /// The calendar of this module is that of chrono wherever chrono has dates: the day of the
  /// week, of the year and the week of the year, and the dates some days or months away.
  #[test]
  fn the_calendar_is_that_of_chrono_in_its_range() {
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    let mut next = |below: u64| {
      state ^= state << 13;
      state ^= state >> 7;
      state ^= state << 17;
      state % below
    };
    let name = |weekday: Weekday| match weekday {
      Weekday::Mon => ("Monday", 1_u8),
      Weekday::Tue => ("Tuesday", 2),
      Weekday::Wed => ("Wednesday", 3),
      Weekday::Thu => ("Thursday", 4),
      Weekday::Fri => ("Friday", 5),
      Weekday::Sat => ("Saturday", 6),
      Weekday::Sun => ("Sunday", 7),
    };
    let feel = |date: NaiveDate| FeelDate(date.year(), date.month(), date.day());
    let mut compared = 0;
    for round in 0..200_000 {
      // near the present, near the year 0, and over the whole range of chrono
      let year = match round % 3 {
        0 => 1900 + next(300) as i32,
        1 => next(900) as i32 - 450,
        _ => next(524_000) as i32 - 262_000,
      };
      let (month, day) = (1 + next(12) as u32, 1 + next(31) as u32);
      let naive = NaiveDate::from_ymd_opt(year, month, day);
      assert_eq!(naive.is_some(), is_valid_date(year, month, day), "{year}-{month}-{day}");
      let Some(naive) = naive else { continue };
      let date = FeelDate(year, month, day);
      let (weekday, number) = name(naive.weekday());
      assert_eq!(Some((weekday.to_string(), number)), date.day_of_week(), "{date}");
      assert_eq!(Some(naive.ordinal() as u16), date.day_of_year(), "{date}");
      assert_eq!(Some(naive.iso_week().week() as u8), date.week_of_year(), "{date}");
      let days = if round % 2 == 0 { next(800) } else { next(40_000_000) };
      assert_eq!(
        naive.checked_add_days(Days::new(days)).map(feel),
        date.add_days(days).filter(|date| NaiveDate::try_from(date.clone()).is_ok()),
        "{date} + {days}"
      );
      assert_eq!(
        naive.checked_sub_days(Days::new(days)).map(feel),
        date.sub_days(days).filter(|date| NaiveDate::try_from(date.clone()).is_ok()),
        "{date} - {days}"
      );
      let months = if round % 2 == 0 { next(50) } else { next(3_000_000) } as u32;
      assert_eq!(
        naive.checked_add_months(Months::new(months)).map(feel),
        date.add_months(months as i64).filter(|date| NaiveDate::try_from(date.clone()).is_ok()),
        "{date} + {months} months"
      );
      assert_eq!(
        naive.checked_sub_months(Months::new(months)).map(feel),
        date.add_months(-(months as i64)).filter(|date| NaiveDate::try_from(date.clone()).is_ok()),
        "{date} - {months} months"
      );
      compared += 1;
    }
    assert!(compared > 190_000, "{compared}");
  }

  /// The dates beyond the range of chrono: the ends of the years of FEEL.
  #[test]
  fn the_first_and_the_last_date_of_feel() {
    let (first, last) = (FeelDate(-999_999_999, 1, 1), FeelDate(999_999_999, 12, 31));
    assert_eq!(Some(last.clone()), "999999999-12-31".parse().ok());
    assert_eq!(Some(first.clone()), "-999999999-01-01".parse().ok());
    assert!("1000000000-01-01".parse::<FeelDate>().is_err());
    assert_eq!(None, last.add_days(1));
    assert_eq!(None, first.sub_days(1));
    assert_eq!(Some(FeelDate(999_999_999, 12, 30)), last.sub_days(1));
    assert_eq!(Some(FeelDate(-999_999_999, 1, 2)), first.add_days(1));
    assert_eq!(None, last.add_days(u64::MAX));
    assert_eq!(None, first.sub_days(u64::MAX));
    // the calendar repeats every 400 years: 146,097 days, a whole number of weeks
    for (year, month, day) in [(2024, 2, 29), (1999, 12, 31), (2000, 1, 1), (1, 1, 1)] {
      let (near, far) = (FeelDate(year, month, day), FeelDate(year + 999_990_000, month, day));
      assert_eq!(near.day_of_week(), far.day_of_week(), "{far}");
      assert_eq!(near.day_of_year(), far.day_of_year(), "{far}");
      assert_eq!(near.week_of_year(), far.week_of_year(), "{far}");
      assert_eq!(Some(far.clone()), near.add_days(146_097 * (999_990_000 / 400)), "{far}");
      assert_eq!(Some(near.clone()), far.sub_days(146_097 * (999_990_000 / 400)), "{near}");
      assert_eq!(Some(far.clone()), near.add_months(12 * 999_990_000), "{far}");
    }
    assert_eq!(Some(("December".to_string(), 12)), last.month_of_year());
    assert_eq!(Some(365), last.day_of_year());
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
