//! Implementation of FEEL time.

use super::feel_date::FeelDate;
use super::feel_zone::FeelZone;
use crate::defs::*;
use crate::errors::*;
use crate::feel_date_time::FeelDateTime;
use crate::feel_dt_duration::FeelDaysAndTimeDuration;
use chrono::{DateTime, FixedOffset};
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
  /// Converts [String] into [FeelTime].
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    if let Some(captures) = RE_TIME.captures(s)
      && let Some(hour_match) = captures.name("hours")
      && let Ok(mut hour) = hour_match.as_str().parse::<u8>()
      && let Some(min_match) = captures.name("minutes")
      && let Ok(min) = min_match.as_str().parse::<u8>()
      && let Some(sec_match) = captures.name("seconds")
      && let Ok(sec) = sec_match.as_str().parse::<u8>()
    {
      let mut fractional = 0.0;
      if let Some(frac_match) = captures.name("fractional")
        && let Ok(frac) = frac_match.as_str().parse::<f64>()
      {
        fractional = frac;
      }
      let nanos = (fractional * 1e9).trunc() as u64;
      if let Some(zone) = FeelZone::from_captures(&captures)
        && is_valid_time(hour, min, sec)
      {
        if hour == 24 {
          hour = 0;
        }
        let time = FeelTime(hour, min, sec, nanos, zone);
        return Ok(time);
      }
    }
    Err(err_invalid_time_literal(s))
  }
}

impl PartialEq for FeelTime {
  /// Two times with offsets or zones are equal when they denote the same instant of the day in
  /// UTC (`10:00:00+02:00 = 08:00:00Z`); nanoseconds are omitted during time comparison.
  /// Otherwise (a local time, or a zone without a known offset) the fields and zones are
  /// compared as written.
  fn eq(&self, rhs: &Self) -> bool {
    if self.4.has_offset()
      && rhs.4.has_offset()
      && let Some((lhs_instant, rhs_instant)) = self.utc_instant(false).zip(rhs.utc_instant(false))
    {
      return lhs_instant == rhs_instant;
    }
    let equal_zones = match (&self.4, &rhs.4) {
      (FeelZone::Utc, FeelZone::Utc) => true,
      (FeelZone::Utc, FeelZone::Zone(zone_name)) | (FeelZone::Zone(zone_name), FeelZone::Utc) => zone_name == ETC_UTC,
      (FeelZone::Utc, FeelZone::Offset(offset)) | (FeelZone::Offset(offset), FeelZone::Utc) => *offset == 0,
      (FeelZone::Local, FeelZone::Local) => true,
      (FeelZone::Offset(offset), FeelZone::Zone(zone_name)) | (FeelZone::Zone(zone_name), FeelZone::Offset(offset)) => {
        (*offset == 0 && zone_name == ETC_UTC)
          || if let Some(zone_offset) = get_zone_offset(zone_name) {
            *offset == zone_offset
          } else {
            false
          }
      }
      (FeelZone::Offset(offset1), FeelZone::Offset(offset2)) => offset1 == offset2,
      (FeelZone::Zone(zone_name1), FeelZone::Zone(zone_name2)) => zone_name1 == zone_name2,
      _ => false,
    };
    self.0 == rhs.0 && self.1 == rhs.1 && self.2 == rhs.2 && equal_zones
  }
}

impl PartialOrd for FeelTime {
  /// Returns the ordering of two times, compared as instants of the day in UTC
  /// (`10:00:00+01:00 < 10:00:00Z`), nanoseconds included. A local time is ordered only
  /// against another local time (like subtraction, a local and a zoned time are not
  /// comparable); when a zone has no known offset, only times written in the same zone are
  /// comparable, field by field.
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    if self.4.has_offset() == other.4.has_offset()
      && let Some((lhs_instant, rhs_instant)) = self.utc_instant(true).zip(other.utc_instant(true))
    {
      return Some(lhs_instant.cmp(&rhs_instant));
    }
    if self.4 == other.4 {
      Some((self.0, self.1, self.2, self.3).cmp(&(other.0, other.1, other.2, other.3)))
    } else {
      None
    }
  }
}

impl Sub<FeelTime> for FeelTime {
  type Output = Option<FeelDaysAndTimeDuration>;
  /// Subtracts [FeelTime] values, result is always [FeelDaysAndTimeDuration].
  fn sub(self, other: Self) -> Self::Output {
    let me_time_tuple = (self.0 as u32, self.1 as u32, self.2 as u32, self.3 as u32);
    let me_offset_opt = match &self.4 {
      FeelZone::Utc => Some(0),
      FeelZone::Local => get_local_offset_t(me_time_tuple),
      FeelZone::Offset(offset) => Some(*offset),
      FeelZone::Zone(zone_name) => get_zone_offset_t(zone_name, me_time_tuple),
    };
    let other_time_tuple = (other.0 as u32, other.1 as u32, other.2 as u32, other.3 as u32);
    let other_offset_opt = match &other.4 {
      FeelZone::Utc => Some(0),
      FeelZone::Local => get_local_offset_t(other_time_tuple),
      FeelZone::Offset(offset) => Some(*offset),
      FeelZone::Zone(zone_name) => get_zone_offset_t(zone_name, other_time_tuple),
    };
    if let Some((me_offset, other_offset)) = me_offset_opt.zip(other_offset_opt) {
      let me_date_opt = date_time_offset_t(me_time_tuple, me_offset);
      let other_date_opt = date_time_offset_t(other_time_tuple, other_offset);
      if let Some((me_date, other_date)) = me_date_opt.zip(other_date_opt)
        && let Some(nanos) = me_date.sub(other_date).num_nanoseconds()
      {
        return Some(FeelDaysAndTimeDuration::from_n(nanos));
      }
    }
    None
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

  /// Returns this time as an instant (on today's date) in UTC, when the zone has a known
  /// offset. The nanoseconds are included only when `with_nanos` is `true`.
  fn utc_instant(&self, with_nanos: bool) -> Option<DateTime<FixedOffset>> {
    let time_tuple = (self.0 as u32, self.1 as u32, self.2 as u32, if with_nanos { self.3 as u32 } else { 0 });
    let offset = match &self.4 {
      FeelZone::Utc => Some(0),
      FeelZone::Local => get_local_offset_t(time_tuple),
      FeelZone::Offset(offset) => Some(*offset),
      FeelZone::Zone(zone_name) => get_zone_offset_t(zone_name, time_tuple),
    }?;
    date_time_offset_t(time_tuple, offset)
  }

  /// Creates UTC time from specified time values.
  pub fn utc(hour: u8, minute: u8, second: u8, nanos: u64) -> Self {
    Self(hour, minute, second, nanos, FeelZone::Utc)
  }

  pub fn utc_opt(hour: u8, minute: u8, second: u8, nanos: u64) -> Option<Self> {
    if is_valid_time(hour, minute, second) {
      Some(Self(hour, minute, second, nanos, FeelZone::Utc))
    } else {
      None
    }
  }

  /// Creates local time from specified time values.
  pub fn local(hour: u8, minute: u8, second: u8, nanos: u64) -> Self {
    Self(hour, minute, second, nanos, FeelZone::Local)
  }

  pub fn local_opt(hour: u8, minute: u8, second: u8, nano: u64) -> Option<Self> {
    if is_valid_time(hour, minute, second) {
      Some(Self(if hour == 24 { 0 } else { hour }, minute, second, nano, FeelZone::Local))
    } else {
      None
    }
  }

  /// Creates a time from specified time and offset values.
  pub fn offset(hour: u8, minute: u8, second: u8, nanos: u64, offset: i32) -> Self {
    Self(hour, minute, second, nanos, FeelZone::Offset(offset))
  }

  pub fn offset_opt(hour: u8, minute: u8, second: u8, nano: u64, offset: i32) -> Option<Self> {
    if is_valid_time(hour, minute, second)
      && let Ok(zone) = FeelZone::try_from(offset)
    {
      return Some(Self(if hour == 24 { 0 } else { hour }, minute, second, nano, zone));
    }
    None
  }

  pub fn zone_opt(hour: u8, minute: u8, second: u8, nano: u64, zone: FeelZone) -> Option<Self> {
    if is_valid_time(hour, minute, second) {
      return Some(Self(if hour == 24 { 0 } else { hour }, minute, second, nano, zone));
    }
    None
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
    assert!((FeelTime(0, 0, 0, 0, FeelZone::Utc) != FeelTime(0, 0, 0, 0, FeelZone::Local)));
    assert!((FeelTime(0, 0, 0, 0, FeelZone::Utc) != FeelTime(0, 0, 0, 0, FeelZone::Offset(18_000))));
    assert!((FeelTime(0, 0, 0, 0, FeelZone::Utc) != FeelTime(0, 0, 0, 0, FeelZone::Zone("Europe/Warsaw".to_string()))));
  }

  #[test]
  fn test_subtract() {
    let t1 = FeelTime(0, 0, 0, 0, FeelZone::Utc);
    let t2 = FeelTime(0, 0, 0, 0, FeelZone::Utc);
    let d = FeelDaysAndTimeDuration::from_n(0);
    assert_eq!(d, (t1 - t2).unwrap());
    let t1 = FeelTime(0, 0, 59, 0, FeelZone::Utc);
    let t2 = FeelTime(0, 0, 39, 0, FeelZone::Utc);
    let d = FeelDaysAndTimeDuration::from_s(20);
    assert_eq!(d, (t1 - t2).unwrap());
  }
}
