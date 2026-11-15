//! Implementation of a FEEL time offset.

use crate::feel_dt_duration::{FeelDaysAndTimeDuration, NANOSECONDS_IN_SECOND};
use std::fmt;

/// The largest magnitude of a time offset: 14:00, in seconds (DMN 1.6 Table 67, `time offset`:
/// "in the interval [duration("-PT14H")..duration("PT14H")] as per the XML Schema Part 2 dateTime
/// datatype").
const MAX_OFFSET_SECONDS: i32 = 14 * 3_600;

/// A time offset from UTC: whole seconds within ±14:00.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeelOffset(i32);

impl FeelOffset {
  /// The offset of UTC, written `+00:00`.
  pub const ZERO: FeelOffset = FeelOffset(0);

  /// The offset of `seconds` seconds east of UTC, or `None` beyond ±14:00. Every offset is built
  /// here.
  pub(crate) fn from_seconds(seconds: i32) -> Option<Self> {
    (-MAX_OFFSET_SECONDS..=MAX_OFFSET_SECONDS).contains(&seconds).then_some(Self(seconds))
  }

  /// The offset of `duration`, or `None` when it is beyond ±14:00 or has a fraction of a second,
  /// which no time offset has (XML Schema Part 2 §3.2.7.3: "Timezones are durations with
  /// (integer-valued) hour and minute properties").
  pub fn from_duration(duration: &FeelDaysAndTimeDuration) -> Option<Self> {
    let nanos = duration.as_nanos();
    if nanos % NANOSECONDS_IN_SECOND != 0 {
      return None;
    }
    i32::try_from(nanos / NANOSECONDS_IN_SECOND).ok().and_then(Self::from_seconds)
  }

  /// The number of seconds east of UTC.
  pub(crate) fn seconds(self) -> i32 {
    self.0
  }

  /// Writes an offset of `seconds` seconds east of UTC: its sign (`-` when the offset is negative,
  /// `+` otherwise, so zero is `+00:00`), then the hours and minutes of its magnitude, then `:` and
  /// the seconds when they are not zero (`-00:30`, `+05:30:15`). It takes a plain count of seconds,
  /// so it also writes an offset that the zone database resolves, which may have seconds and may
  /// lie beyond ±14:00 (local mean time before the standard zones).
  pub fn write_seconds(out: &mut impl fmt::Write, seconds: i32) -> fmt::Result {
    let sign = if seconds < 0 { '-' } else { '+' };
    let magnitude = seconds.unsigned_abs();
    let (hours, minutes, seconds) = (magnitude / 3_600, magnitude % 3_600 / 60, magnitude % 60);
    if seconds == 0 {
      write!(out, "{sign}{hours:02}:{minutes:02}")
    } else {
      write!(out, "{sign}{hours:02}:{minutes:02}:{seconds:02}")
    }
  }
}

impl fmt::Display for FeelOffset {
  /// Writes the offset as `±hh:mm`, with `:ss` when its seconds are not zero.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    Self::write_seconds(f, self.0)
  }
}

#[cfg(test)]
mod tests {
  use super::FeelOffset;
  use crate::FeelDaysAndTimeDuration;

  fn text(seconds: i32) -> String {
    FeelOffset::from_seconds(seconds).unwrap().to_string()
  }

  #[test]
  fn an_offset_prints_its_own_sign() {
    assert_eq!("-00:30", text(-1_800));
    assert_eq!("+00:30", text(1_800));
    assert_eq!("-00:00:30", text(-30));
    assert_eq!("+00:00:30", text(30));
    assert_eq!("-01:30", text(-5_400));
    assert_eq!("-05:00:01", text(-18_001));
    assert_eq!("+05:01", text(18_060));
    assert_eq!("+00:00", text(0));
    assert_eq!("+00:00", FeelOffset::ZERO.to_string());
  }

  #[test]
  fn an_offset_is_within_14_hours() {
    assert_eq!("+14:00", text(50_400));
    assert_eq!("-14:00", text(-50_400));
    assert_eq!(None, FeelOffset::from_seconds(50_401));
    assert_eq!(None, FeelOffset::from_seconds(-50_401));
    assert_eq!(None, FeelOffset::from_seconds(i32::MIN));
    assert_eq!(None, FeelOffset::from_seconds(i32::MAX));
    assert_eq!(Some(-50_400), FeelOffset::from_seconds(-50_400).map(FeelOffset::seconds));
  }

  #[test]
  fn an_offset_of_a_duration_is_whole_seconds_within_14_hours() {
    let seconds = |text: &str| FeelOffset::from_duration(&FeelDaysAndTimeDuration::try_from(text).unwrap()).map(FeelOffset::seconds);
    assert_eq!(Some(50_400), seconds("PT14H"));
    assert_eq!(Some(-50_400), seconds("-PT14H"));
    assert_eq!(Some(9_955), seconds("PT2H45M55S"));
    assert_eq!(Some(-30), seconds("-PT30S"));
    assert_eq!(Some(0), seconds("PT0S"));
    assert_eq!(None, seconds("PT14H1S"));
    assert_eq!(None, seconds("-PT14H1S"));
    assert_eq!(None, seconds("P1D"));
    assert_eq!(None, seconds("PT0.5S"));
    assert_eq!(None, seconds("-PT1H0.5S"));
    assert_eq!(None, seconds("PT0.000000001S"));
    assert_eq!(None, seconds("PT13H59M59.999999999S"));
    assert_eq!(None, seconds("PT1000000H"));
    assert_eq!(None, seconds("-PT1000000H"));
  }

  #[test]
  fn any_count_of_seconds_is_written() {
    let write = |seconds| {
      let mut out = String::new();
      FeelOffset::write_seconds(&mut out, seconds).unwrap();
      out
    };
    assert_eq!("+00:09:21", write(561));
    assert_eq!("-15:56:08", write(-57_368));
    assert_eq!("-596523:14:08", write(i32::MIN));
    assert_eq!("+596523:14:07", write(i32::MAX));
  }
}
