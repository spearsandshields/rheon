//! Implementation of FEEL time.

use super::feel_date::FeelDate;
use super::feel_zone::FeelZone;
use crate::defs::*;
use crate::errors::*;
use crate::feel_date_time::FeelDateTime;
use crate::feel_dt_duration::FeelDaysAndTimeDuration;
use crate::feel_offset::FeelOffset;
use crate::parts::split_time;
use chrono::{NaiveTime, Timelike};
use dsntk_common::{DsntkError, Result};
use std::cmp::Ordering;
use std::fmt;
use std::fmt::Display;
use std::ops::{Add, Sub};
use std::str::FromStr;

/// FEEL time.
#[derive(Debug, Clone)]
pub struct FeelTime(u8, u8, u8, u64, FeelZone);

impl Display for FeelTime {
  /// Implements [Display] for FEEL time.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if self.3 > 0 {
      write!(f, "{:02}:{:02}:{:02}.{}{}", self.0, self.1, self.2, nanos_to_string(self.3), self.4)
    } else {
      write!(f, "{:02}:{:02}:{:02}{}", self.0, self.1, self.2, self.4)
    }
  }
}

impl FromStr for FeelTime {
  type Err = DsntkError;
  /// Converts [String] into [FeelTime]; fraction digits beyond nanoseconds are truncated (see
  /// [FeelTime::parse_reporting_truncation]).
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    Self::parse_reporting_truncation(s).map(|(time, _)| time)
  }
}

impl PartialEq for FeelTime {
  /// `valuet(e1) = valuet(e2)` (DMN 1.7 Beta 1, Table 54 and 10.3.2.3.4): the two as instants of the
  /// day in whole seconds (TCK 0068 time_005: the value resolution is seconds), a local time taking
  /// the offset of the other (the rule of Table 55, which the owner applies to `=` too, Q-T1), so
  /// `10:00:00 = 10:00:00+02:00` and `10:00:00+02:00 = 08:00:00Z` are true. When a zone has no
  /// known offset, the fields and zones are compared as written.
  fn eq(&self, rhs: &Self) -> bool {
    match self.compared_instants(rhs, false) {
      Some((lhs, rhs)) => lhs == rhs,
      None => self.written_alike(rhs) && (self.0, self.1, self.2) == (rhs.0, rhs.1, rhs.2),
    }
  }
}

impl PartialOrd for FeelTime {
  /// `valuet(e1) < valuet(e2)` (DMN 1.7 Beta 1, Table 55): the two as instants of the day, the
  /// fraction of the second included; "if one input has a null timezone offset, that input uses the timezone offset of the
  /// other input" (`10:00:00 < 11:00:00Z` is true). When a zone has no known offset, only times
  /// written in the same zone are ordered, field by field.
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    match self.compared_instants(other, true) {
      Some((lhs, rhs)) => Some(lhs.cmp(&rhs)),
      None if self.written_alike(other) => Some((self.0, self.1, self.2, self.3).cmp(&(other.0, other.1, other.2, other.3))),
      None => None,
    }
  }
}

impl Sub<FeelTime> for FeelTime {
  type Output = Option<FeelDaysAndTimeDuration>;
  /// Subtracts [FeelTime] values, result is always [FeelDaysAndTimeDuration].
  fn sub(self, other: Self) -> Self::Output {
    let (me, other) = if self.4 == FeelZone::Local && other.4 == FeelZone::Local {
      (self.utc_instant(true)?, other.utc_instant(true)?)
    } else {
      (self.instant_at(self.offset()?, true)?, other.instant_at(other.offset()?, true)?)
    };
    FeelDaysAndTimeDuration::from_n(me - other)
  }
}

impl Add<FeelDaysAndTimeDuration> for FeelTime {
  type Output = FeelTime;
  /// Adds [FeelDaysAndTimeDuration] to [FeelTime]; the result wraps around midnight
  /// (`23:59:59 + PT1S = 00:00:00`), nanoseconds included.
  fn add(self, rhs: FeelDaysAndTimeDuration) -> Self::Output {
    self.add_nanos(rhs.as_nanos() as i128)
  }
}

impl Sub<FeelDaysAndTimeDuration> for FeelTime {
  type Output = FeelTime;
  /// Subtracts [FeelDaysAndTimeDuration] from [FeelTime]; the result wraps around midnight
  /// (`00:00:00 - PT1S = 23:59:59`), nanoseconds included.
  fn sub(self, rhs: FeelDaysAndTimeDuration) -> Self::Output {
    self.add_nanos(-(rhs.as_nanos() as i128))
  }
}

/// Number of nanoseconds in a day.
const NANOS_PER_DAY: i128 = 24 * 60 * 60 * 1_000_000_000;

impl FeelTime {
  /// Returns this time moved by `nanos` nanoseconds, modulo 24 hours, in the same zone.
  fn add_nanos(self, nanos: i128) -> Self {
    let seconds_of_day = (self.0 as i128) * 3600 + (self.1 as i128) * 60 + self.2 as i128;
    let total = (seconds_of_day * 1_000_000_000 + self.3 as i128 + nanos).rem_euclid(NANOS_PER_DAY);
    let seconds = total / 1_000_000_000;
    let nanos = (total % 1_000_000_000) as u64;
    FeelTime((seconds / 3600) as u8, ((seconds % 3600) / 60) as u8, (seconds % 60) as u8, nanos, self.4)
  }

  /// The offset of the zone of this time in seconds: of the host's zone today for a local time.
  fn offset(&self) -> Option<i32> {
    let time_tuple = (self.0 as u32, self.1 as u32, self.2 as u32, self.3 as u32);
    match &self.4 {
      FeelZone::Utc => Some(0),
      FeelZone::Local => get_local_offset_t(time_tuple),
      FeelZone::Offset(offset) => Some(offset.seconds()),
      FeelZone::Zone(zone_name) => get_zone_offset_t(zone_name, time_tuple),
    }
  }

  /// Returns this time as an instant of the day in UTC, in nanoseconds from midnight (below zero
  /// or above a day for a time that an offset moves to another day), when the zone has a known
  /// offset. The nanoseconds are included only when `with_nanos` is `true`.
  ///
  /// Two local times are in one zone, so each is its own instant: they are compared and
  /// subtracted as written, whatever the offset of the host's zone is today.
  fn utc_instant(&self, with_nanos: bool) -> Option<i64> {
    let offset = match &self.4 {
      FeelZone::Utc | FeelZone::Local => 0,
      FeelZone::Offset(offset) => offset.seconds(),
      FeelZone::Zone(zone_name) => get_zone_offset_t(zone_name, (self.0 as u32, self.1 as u32, self.2 as u32, if with_nanos { self.3 as u32 } else { 0 }))?,
    };
    self.instant_at(offset, with_nanos)
  }

  /// The two times as instants of the day in UTC, in nanoseconds (the fraction of the second only
  /// with `with_nanos`), for `=` and `<` (Tables 54 and 55): two local times as written (they are
  /// in one zone), and a local time at the offset of the other when only one is local. `None` when
  /// a zone has no known offset.
  fn compared_instants(&self, other: &FeelTime, with_nanos: bool) -> Option<(i64, i64)> {
    let (lhs, rhs) = match (&self.4, &other.4) {
      (FeelZone::Local, FeelZone::Local) => (0, 0),
      (FeelZone::Local, _) => {
        let offset = other.known_offset()?;
        (offset, offset)
      }
      (_, FeelZone::Local) => {
        let offset = self.known_offset()?;
        (offset, offset)
      }
      _ => (self.known_offset()?, other.known_offset()?),
    };
    Some((self.instant_at(lhs, with_nanos)?, other.instant_at(rhs, with_nanos)?))
  }

  /// The offset of this time's own zone in seconds; `None` for a local time and for a zone
  /// without a known offset.
  fn known_offset(&self) -> Option<i32> {
    match &self.4 {
      FeelZone::Utc => Some(0),
      FeelZone::Local => None,
      FeelZone::Offset(offset) => Some(offset.seconds()),
      FeelZone::Zone(zone_name) => get_zone_offset_t(zone_name, (self.0 as u32, self.1 as u32, self.2 as u32, 0)),
    }
  }

  /// Whether the two are written in the same zone: both local, both UTC, the same offset or the
  /// same zone name.
  pub(crate) fn zone_written_alike(&self, other: &FeelTime) -> bool {
    self.written_alike(other)
  }

  /// [FeelTime::zone_written_alike].
  fn written_alike(&self, other: &FeelTime) -> bool {
    match (&self.4, &other.4) {
      (FeelZone::Utc, FeelZone::Utc) | (FeelZone::Local, FeelZone::Local) => true,
      (FeelZone::Offset(lhs), FeelZone::Offset(rhs)) => lhs == rhs,
      (FeelZone::Zone(lhs), FeelZone::Zone(rhs)) => lhs == rhs,
      _ => false,
    }
  }

  /// The instant of the day in UTC of this time at an offset in seconds; `None` for an offset
  /// of a day or more.
  fn instant_at(&self, offset: i32, with_nanos: bool) -> Option<i64> {
    if self.0 > 23 || self.1 > 59 || self.2 > 59 || offset.abs() >= 86_400 {
      return None;
    }
    let seconds = i64::from(self.0) * 3600 + i64::from(self.1) * 60 + i64::from(self.2) - i64::from(offset);
    Some(seconds * 1_000_000_000 + if with_nanos { self.3 as i64 } else { 0 })
  }

  /// Parses a time literal, as [FromStr] does, and also returns the fraction digits beyond
  /// nanoseconds that were truncated, when one of them is not zero.
  ///
  /// ```
  /// use rheon_feel_temporal::FeelTime;
  /// let (time, dropped) = FeelTime::parse_reporting_truncation("10:00:00.1234567891").unwrap();
  /// assert_eq!(("10:00:00.123456789", Some("1")), (time.to_string().as_str(), dropped));
  /// assert_eq!(None, FeelTime::parse_reporting_truncation("10:00:00.1234567890").unwrap().1);
  /// ```
  pub fn parse_reporting_truncation(text: &str) -> Result<(Self, Option<&str>), DsntkError> {
    if let Some(captures) = split_time(text)
      && let Some(hour_match) = captures.name("hours")
      && let Ok(hour) = hour_match.as_str().parse::<u8>()
      && let Some(min_match) = captures.name("minutes")
      && let Ok(min) = min_match.as_str().parse::<u8>()
      && let Some(sec_match) = captures.name("seconds")
      && let Ok(sec) = sec_match.as_str().parse::<u8>()
      && let Some((nanos, dropped)) = parse_fraction_nanos(fraction_digits(&captures))
      && let Some(zone) = FeelZone::from_captures(&captures)
      && let Some(time) = FeelTime::zone_opt(hour, min, sec, u64::from(nanos), zone)
    {
      return Ok((time, dropped));
    }
    Err(err_invalid_time_literal(text))
  }

  /// Creates a time in `zone`, or `None` when the parts are not a time of day. A time of day has
  /// an hour below 24, a minute and a second below 60 and nanoseconds below 10^9, or is 24:00:00
  /// with a zero fraction, which is midnight (XML Schema Part 2 §3.2.7: "'24' is permitted if the
  /// minutes and seconds represented are zero"; DMN 1.6 §10.3.2.3.4: a time is a value of the XML
  /// Schema time datatype). Every time is built here or is midnight, so every time is valid.
  pub fn zone_opt(hour: u8, minute: u8, second: u8, nanos: u64, zone: FeelZone) -> Option<Self> {
    is_valid_time(hour, minute, second, nanos).then_some(Self(if hour == 24 { 0 } else { hour }, minute, second, nanos, zone))
  }

  /// Creates a UTC time (see [FeelTime::zone_opt]).
  pub fn utc_opt(hour: u8, minute: u8, second: u8, nanos: u64) -> Option<Self> {
    Self::zone_opt(hour, minute, second, nanos, FeelZone::Utc)
  }

  /// Creates a local time (see [FeelTime::zone_opt]).
  pub fn local_opt(hour: u8, minute: u8, second: u8, nanos: u64) -> Option<Self> {
    Self::zone_opt(hour, minute, second, nanos, FeelZone::Local)
  }

  /// Creates a time with an offset of `offset` seconds (see [FeelTime::zone_opt]), or `None` when
  /// the offset is beyond ±14:00 either; the zero offset is UTC.
  pub fn offset_opt(hour: u8, minute: u8, second: u8, nanos: u64, offset: i32) -> Option<Self> {
    Self::zone_opt(hour, minute, second, nanos, FeelZone::from(FeelOffset::from_seconds(offset)?))
  }

  /// Midnight, 00:00:00, in `zone`.
  pub fn midnight(zone: FeelZone) -> Self {
    Self(0, 0, 0, 0, zone)
  }

  /// The time of day `time` of a clock, in `zone`. chrono represents a leap second as nanoseconds
  /// from 10^9 on; the XML Schema value space has no leap second, so it is kept as the last
  /// nanosecond of the second before.
  pub(crate) fn of_clock(time: NaiveTime, zone: FeelZone) -> Self {
    Self(
      time.hour() as u8,
      time.minute() as u8,
      time.second() as u8,
      u64::from(time.nanosecond().min(999_999_999)),
      zone,
    )
  }

  pub fn hour(&self) -> u8 {
    self.0
  }

  pub fn minute(&self) -> u8 {
    self.1
  }

  pub fn second(&self) -> u8 {
    self.2
  }

  pub fn nanos(&self) -> u64 {
    self.3
  }

  /// Returns `true` when both times are written in the same zone (the same offset, the same
  /// zone name, both UTC or both local) and have the same hour, minute and second: the FEEL
  /// `is()` semantics, stricter than `==`, which compares instants.
  pub fn is(&self, rhs: &FeelTime) -> bool {
    let same_zone = match (&self.4, &rhs.4) {
      (FeelZone::Utc, FeelZone::Utc) | (FeelZone::Local, FeelZone::Local) => true,
      (FeelZone::Utc, FeelZone::Zone(zone_name)) | (FeelZone::Zone(zone_name), FeelZone::Utc) => zone_name == ETC_UTC,
      (FeelZone::Offset(offset_1), FeelZone::Offset(offset_2)) => offset_1 == offset_2,
      (FeelZone::Zone(zone_name_1), FeelZone::Zone(zone_name_2)) => zone_name_1 == zone_name_2,
      _ => false,
    };
    // the same element of the semantic domain: the fraction of the second too (10.3.2.2)
    same_zone && self.0 == rhs.0 && self.1 == rhs.1 && self.2 == rhs.2 && self.3 == rhs.3
  }

  pub fn zone(&self) -> &FeelZone {
    &self.4
  }

  pub fn as_tuple(&self) -> (u8, u8, u8, u64, FeelZone) {
    (self.0, self.1, self.2, self.3, self.4.clone())
  }

  pub fn feel_time_offset(&self) -> Option<i32> {
    feel_time_offset(&FeelDateTime::new(FeelDate::today(), self.clone()))
  }

  pub fn feel_time_zone(&self) -> Option<String> {
    feel_time_zone(&FeelDateTime::new(FeelDate::today(), self.clone()))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn eq_time(h: u8, m: u8, s: u8, n: u64, z: FeelZone, time: &str) {
    let expected = FeelTime(h, m, s, n, z);
    let actual = FeelTime::from_str(time).expect("should not fail");
    assert_eq!(expected, actual);
  }

  /// The comparison and the difference of two times with offsets are those of the same times on
  /// one date, as date and times with these offsets.
  #[test]
  fn times_with_offsets_are_compared_and_subtracted_as_instants() {
    use chrono::{FixedOffset, TimeZone};
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = |below: u64| {
      state ^= state << 13;
      state ^= state >> 7;
      state ^= state << 17;
      state % below
    };
    let mut equal = 0;
    for _ in 0..200_000 {
      let mut one = || {
        let (h, m, s, n) = (next(24) as u8, next(4) as u8 * 15, next(3) as u8, next(3) * 500_000_000 % 1_000_000_000);
        let offset = (next(57) as i32 - 28) * 1800;
        let zone = if offset == 0 && next(2) == 0 {
          FeelZone::Utc
        } else {
          FeelZone::Offset(FeelOffset::from_duration(&FeelDaysAndTimeDuration::from_s(offset)).unwrap())
        };
        let instant = FixedOffset::east_opt(offset).unwrap().with_ymd_and_hms(2024, 3, 10, h as u32, m as u32, s as u32).unwrap();
        (FeelTime(h, m, s, n, zone), instant, n)
      };
      let ((a, instant_a, nanos_a), (b, instant_b, nanos_b)) = (one(), one());
      let with_nanos = |instant: chrono::DateTime<FixedOffset>, nanos: u64| instant.timestamp() * 1_000_000_000 + nanos as i64;
      assert_eq!(Some(with_nanos(instant_a, nanos_a).cmp(&with_nanos(instant_b, nanos_b))), a.partial_cmp(&b), "{a} and {b}");
      assert_eq!(instant_a == instant_b, a == b, "{a} and {b}");
      equal += usize::from(a == b);
      let difference = FeelDaysAndTimeDuration::from_n(with_nanos(instant_a, nanos_a) - with_nanos(instant_b, nanos_b));
      assert_eq!(difference, a.clone() - b.clone(), "{a} and {b}");
    }
    assert!(equal > 100, "too few equal pairs: {equal}");
  }

  #[test]
  fn local_times_are_compared_and_subtracted_as_written() {
    let time = |text: &str| FeelTime::from_str(text).unwrap();
    assert_eq!(Some(Ordering::Less), time("00:59:00").partial_cmp(&time("01:00:00")));
    assert_eq!(Some(Ordering::Greater), time("10:00:00.000000002").partial_cmp(&time("10:00:00.000000001")));
    assert_eq!(Some(Ordering::Equal), time("10:00:00").partial_cmp(&time("10:00:00")));
    assert_eq!(FeelDaysAndTimeDuration::from_n(-3_600_000_000_000), time("09:00:00") - time("10:00:00"));
    // a local time takes the offset of the other (Table 55)
    assert_eq!(Some(Ordering::Equal), time("10:00:00").partial_cmp(&time("10:00:00Z")));
    assert_eq!(Some(Ordering::Less), time("10:00:00").partial_cmp(&time("11:00:00+01:00")));
  }

  #[test]
  fn test_time_from_str() {
    eq_time(18, 37, 9, 0, FeelZone::Local, "18:37:09");
    eq_time(16, 37, 9, 0, FeelZone::Utc, "16:37:09z");
    eq_time(16, 37, 9, 0, FeelZone::Utc, "16:37:09Z");
    eq_time(16, 37, 9, 0, FeelZone::Zone("Etc/UTC".to_string()), "16:37:09@Etc/UTC");
    eq_time(17, 37, 9, 0, FeelZone::Zone("Europe/London".to_string()), "17:37:09@Europe/London");
    eq_time(18, 37, 9, 0, FeelZone::Zone("Europe/Warsaw".to_string()), "18:37:09@Europe/Warsaw");
    eq_time(18, 37, 9, 0, FeelZone::Zone("Africa/Johannesburg".to_string()), "18:37:09@Africa/Johannesburg");
    eq_time(10, 37, 9, 0, FeelZone::Zone("America/Vancouver".to_string()), "10:37:09@America/Vancouver");
    eq_time(13, 37, 9, 0, FeelZone::Zone("America/New_York".to_string()), "13:37:09@America/New_York");
  }

  #[test]
  fn test_time_from_str_errors() {
    assert_eq!(Err(err_invalid_time_literal("24:37:09")), FeelTime::from_str("24:37:09"));
    assert_eq!(Err(err_invalid_time_literal("18:60:09")), FeelTime::from_str("18:60:09"));
    assert_eq!(Err(err_invalid_time_literal("05:12:60")), FeelTime::from_str("05:12:60"));
  }

  #[test]
  fn test_eq() {
    assert!((FeelTime(0, 0, 0, 0, FeelZone::Utc) == FeelTime(0, 0, 0, 0, FeelZone::Utc)));
    // a local time takes the offset of the other (Table 55, applied to `=` by the owner, Q-T1)
    assert!((FeelTime(0, 0, 0, 0, FeelZone::Utc) == FeelTime(0, 0, 0, 0, FeelZone::Local)));
    assert!((FeelTime(0, 0, 0, 0, FeelZone::Utc) != FeelTime::offset_opt(0, 0, 0, 0, 18_000).unwrap()));
    assert!((FeelTime(0, 0, 0, 0, FeelZone::Utc) != FeelTime(0, 0, 0, 0, FeelZone::Zone("Europe/Warsaw".to_string()))));
  }

  #[test]
  fn test_subtract() {
    let t1 = FeelTime(0, 0, 0, 0, FeelZone::Utc);
    let t2 = FeelTime(0, 0, 0, 0, FeelZone::Utc);
    let d = FeelDaysAndTimeDuration::default();
    assert_eq!(d, (t1 - t2).unwrap());
    let t1 = FeelTime(0, 0, 59, 0, FeelZone::Utc);
    let t2 = FeelTime(0, 0, 39, 0, FeelZone::Utc);
    let d = FeelDaysAndTimeDuration::from_s(20);
    assert_eq!(d, (t1 - t2).unwrap());
  }
}
