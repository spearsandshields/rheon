//! Test cases for `FEEL` time type.

use crate::feel_offset::FeelOffset;
use crate::{FeelTime, FeelZone};

#[test]
fn _0001() {
  let time_a = FeelTime::offset_opt(10, 11, 12, 0, 7).unwrap();
  let time_b = FeelTime::zone_opt(10, 11, 12, 0, FeelZone::Zone("Europe/Sofa".to_string())).unwrap();
  assert!(!(time_a == time_b));
}

#[test]
fn _0002() {
  let time_a = FeelTime::offset_opt(10, 11, 12, 0, 7).unwrap();
  let time_b = FeelTime::zone_opt(10, 11, 12, 0, FeelZone::Zone("Europe/Sofa".to_string())).unwrap();
  assert!((time_a - time_b).is_none());
}

#[test]
fn _0003() {
  assert!(FeelTime::local_opt(26, 11, 12, 0).is_none());
  assert!(FeelTime::local_opt(24, 1, 0, 0).is_none());
}

#[test]
fn _0004() {
  assert!(FeelTime::offset_opt(26, 11, 12, 0, 0).is_none());
  assert!(FeelTime::offset_opt(24, 1, 0, 0, 0).is_none());
}

#[test]
fn _0005() {
  assert!(FeelTime::zone_opt(26, 11, 12, 0, FeelZone::Utc).is_none());
  assert!(FeelTime::zone_opt(24, 1, 0, 0, FeelZone::Local).is_none());
}

#[test]
fn a_time_of_day_is_below_24_hours() {
  assert!(FeelTime::local_opt(10, 0, 0, 1_000_000_000).is_none());
  assert!(FeelTime::utc_opt(10, 0, 0, u64::MAX).is_none());
  assert!(FeelTime::offset_opt(10, 0, 0, 1_000_000_000, 3_600).is_none());
  assert!(FeelTime::zone_opt(23, 59, 59, 1_000_000_000, FeelZone::Utc).is_none());
  assert_eq!(999_999_999, FeelTime::local_opt(23, 59, 59, 999_999_999).unwrap().nanos());
  assert!(crate::FeelDateTime::utc(2020, 1, 1, 10, 0, 0, 1_000_000_000).is_none());
  assert!(crate::FeelDateTime::local(2020, 1, 1, 10, 0, 0, 1_000_000_000).is_none());
  assert!(crate::FeelDateTime::offset((2020, 1, 1), (10, 0, 0, 1_000_000_000), 0).is_none());
}

#[test]
fn midnight_may_be_written_24_00_00_with_a_zero_fraction_only() {
  for zone in [FeelZone::Utc, FeelZone::Local, FeelZone::Offset(FeelOffset::from_seconds(3_600).unwrap())] {
    assert!(FeelTime::zone_opt(24, 0, 0, 1, zone.clone()).is_none());
    assert!(FeelTime::zone_opt(24, 0, 0, 500_000_000, zone.clone()).is_none());
    assert_eq!((0, 0, 0, 0, zone.clone()), FeelTime::zone_opt(24, 0, 0, 0, zone).unwrap().as_tuple());
  }
  assert_eq!("00:00:00Z", FeelTime::utc_opt(24, 0, 0, 0).unwrap().to_string());
  assert!("24:00:00.5".parse::<FeelTime>().is_err());
  assert!("24:00:00.000000001Z".parse::<FeelTime>().is_err());
  assert_eq!("00:00:00", "24:00:00".parse::<FeelTime>().unwrap().to_string());
  assert_eq!("00:00:00Z", "24:00:00.000Z".parse::<FeelTime>().unwrap().to_string());
  assert!(crate::FeelDateTime::try_from("2020-01-01T24:00:00.5Z").is_err());
  assert!(crate::FeelDateTime::try_from("2020-01-01T24:00:00.000000001").is_err());
  assert_eq!("2021-01-01T00:00:00Z", crate::FeelDateTime::try_from("2020-12-31T24:00:00Z").unwrap().to_string());
  assert_eq!("2021-01-01T00:00:00Z", crate::FeelDateTime::try_from("2020-12-31T24:00:00.000Z").unwrap().to_string());
}
