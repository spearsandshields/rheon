#![doc = include_str!("../docs/README.md")]
#![deny(rustdoc::broken_intra_doc_links)]

#[macro_use]
extern crate dsntk_macros;

mod defs;
mod errors;
mod feel_date;
mod feel_date_time;
mod feel_dt_duration;
mod feel_offset;
mod feel_time;
mod feel_ym_duration;
mod feel_zone;
mod parts;

#[cfg(test)]
mod tests;

pub use defs::{Day, DayOfWeek, DayOfYear, Month, MonthOfYear, WeekOfYear, Year};
pub use defs::{local_now, local_offset_override, parse_instant, set_clock_override, set_local_offset_override};
pub use feel_date::FeelDate;
pub use feel_date_time::FeelDateTime;
pub use feel_dt_duration::FeelDaysAndTimeDuration;
pub use feel_offset::FeelOffset;
pub use feel_time::FeelTime;
pub use feel_ym_duration::FeelYearsAndMonthsDuration;
pub use feel_zone::FeelZone;

/// What a time, or a date and time, is equal to (DMN 1.7 Beta 1, Tables 54 and 55, in whole
/// seconds): the keys under which equal values meet, so that a set of them is looked up, not
/// compared pairwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EqualityKey {
  /// A local value: equal to the values, local or zoned, written with the same wall-clock time
  /// (a local one takes the offset of the other).
  Local { wall: i64 },
  /// A value with a known offset: equal to the zoned values of the same instant, and to the local
  /// values of the same wall-clock time.
  Zoned { wall: i64, instant: i64 },
  /// A value whose zone gives it no single offset: compared with every value of its kind.
  Loose,
}
