//! Implementation of FEEL timezone.

use crate::defs::*;
use crate::feel_offset::FeelOffset;
use regex::Captures;
use std::fmt;

/// FEEL time zone.
#[derive(Debug, Clone)]
pub enum FeelZone {
  /// UTC time zone.
  Utc,
  /// Local time zone.
  Local,
  /// Time zone defined as an offset from UTC: whole seconds within ±14:00 (DMN 1.6 Table 67,
  /// `time offset`). [From] makes the zero offset [FeelZone::Utc]; written as this variant it
  /// prints `+00:00`.
  Offset(FeelOffset),
  /// Time zone defined as a value from IANA database.
  Zone(String),
}

impl fmt::Display for FeelZone {
  /// Converts [FeelZone] into its text representation.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      FeelZone::Utc => write!(f, "Z"),
      FeelZone::Local => write!(f, ""),
      FeelZone::Offset(offset) => write!(f, "{offset}"),
      FeelZone::Zone(zone) => write!(f, "@{zone}"),
    }
  }
}

impl PartialEq for FeelZone {
  /// Returns `true` when two time zones are considered equal.
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (FeelZone::Utc, FeelZone::Utc) => true,
      (FeelZone::Utc, FeelZone::Zone(zone_name)) | (FeelZone::Zone(zone_name), FeelZone::Utc) => zone_name == ETC_UTC,
      (FeelZone::Utc, FeelZone::Offset(offset)) | (FeelZone::Offset(offset), FeelZone::Utc) => *offset == FeelOffset::ZERO,
      (FeelZone::Local, FeelZone::Local) => true,
      (FeelZone::Offset(offset), FeelZone::Zone(zone_name)) | (FeelZone::Zone(zone_name), FeelZone::Offset(offset)) => {
        (*offset == FeelOffset::ZERO && zone_name == ETC_UTC)
          || if let Some(zone_offset) = get_zone_offset(zone_name) {
            offset.seconds() == zone_offset
          } else {
            false
          }
      }
      (FeelZone::Offset(_), FeelZone::Offset(_)) => true,
      (FeelZone::Zone(zone_name1), FeelZone::Zone(zone_name2)) => zone_name1 == zone_name2,
      _ => false,
    }
  }
}

impl From<FeelOffset> for FeelZone {
  /// The zone of `offset`: UTC for the zero offset, a zone with that offset otherwise.
  fn from(offset: FeelOffset) -> Self {
    if offset == FeelOffset::ZERO { Self::Utc } else { Self::Offset(offset) }
  }
}

impl FeelZone {
  /// Creates [FeelZone] from timezone captures taken from regular expression.
  pub fn from_captures(captures: &Captures) -> Option<Self> {
    if captures.name("zulu").is_some() {
      return Some(FeelZone::Utc);
    }
    if let Some(sign_match) = captures.name("offSign")
      && let Some(hours_match) = captures.name("offHours")
      && let Ok(hours) = hours_match.as_str().parse::<i32>()
      && let Some(minutes_match) = captures.name("offMinutes")
      && let Ok(minutes) = minutes_match.as_str().parse::<i32>()
    {
      let mut offset = 3600 * hours + 60 * minutes;
      if let Some(seconds_match) = captures.name("offSeconds")
        && let Ok(seconds) = seconds_match.as_str().parse::<i32>()
      {
        offset += seconds;
      }
      if sign_match.as_str() == "-" {
        offset = -offset;
      }
      return FeelOffset::from_seconds(offset).map(Self::from);
    }
    if let Some(zone_match) = captures.name("zone") {
      return if zone_match.as_str().parse::<chrono_tz::Tz>().is_ok() {
        Some(FeelZone::Zone(zone_match.as_str().to_string()))
      } else {
        None
      };
    }
    Some(FeelZone::Local)
  }

  /// The zone written as `designator`: `Z` or `z`, an offset such as `+02:00` or `-05:30:15`, or
  /// `@` and an IANA zone name, by the rules of the zone suffix of a time literal (an offset within
  /// ±14:00, a zone name the zone database knows). `None` for any other text, the empty one
  /// included.
  pub fn from_designator(designator: &str) -> Option<Self> {
    Self::from_captures(&RE_ZONE.captures(designator)?)
  }

  /// Returns `true` when this [FeelZone] has defined offset (is not local).
  pub fn has_offset(&self) -> bool {
    *self != FeelZone::Local
  }
}

#[cfg(test)]
mod tests {
  use crate::defs::*;
  use crate::feel_offset::FeelOffset;
  use crate::feel_zone::FeelZone;

  /// The seconds of the offset of `zone`, when it is a zone with an offset.
  fn offset_seconds(zone: Option<FeelZone>) -> Option<i32> {
    match zone {
      Some(FeelZone::Offset(offset)) => Some(offset.seconds()),
      _ => None,
    }
  }

  /// The zone of an offset of `seconds` seconds.
  fn offset(seconds: i32) -> FeelZone {
    FeelZone::Offset(FeelOffset::from_seconds(seconds).unwrap())
  }

  macro_rules! assert_zone {
    ($zone:expr, $time:expr) => {
      assert_eq!($zone, FeelZone::from_captures(&RE_TIME.captures($time).unwrap()));
    };
  }

  #[test]
  fn test_format_offset() {
    assert_eq!("+05:00", offset(18_000).to_string());
    assert_eq!("-05:00", offset(-18_000).to_string());
    assert_eq!("+05:00:01", offset(18_001).to_string());
    assert_eq!("-05:00:01", offset(-18_001).to_string());
    assert_eq!("-00:30", offset(-1_800).to_string());
    assert_eq!("-00:00:30", offset(-30).to_string());
    assert_eq!("-14:00", offset(-50_400).to_string());
    assert_eq!("+14:00", offset(50_400).to_string());
  }

  #[test]
  fn test_format_utc() {
    assert_eq!("Z", FeelZone::from(FeelOffset::ZERO).to_string());
    assert_eq!("Z", FeelZone::Utc.to_string());
  }

  #[test]
  fn test_format_local() {
    assert_eq!("", FeelZone::Local.to_string());
  }

  #[test]
  fn test_from_captures() {
    assert_zone!(Some(FeelZone::Local), "00:00:00");
    assert_zone!(Some(FeelZone::Utc), "00:00:00Z");
    assert_zone!(Some(FeelZone::Utc), "00:00:00z");
    assert_zone!(Some(FeelZone::Utc), "00:00:00+00:00");
    assert_zone!(Some(FeelZone::Utc), "00:00:00-00:00");
    assert_zone!(Some(FeelZone::Utc), "00:00:00-00:00");
    assert_zone!(Some(offset(18_000)), "00:00:00+05:00");
    assert_zone!(Some(offset(18_001)), "00:00:00+05:00:01");
    assert_zone!(Some(offset(18_060)), "00:00:00+05:01:00");
    assert_zone!(Some(offset(-18_000)), "00:00:00-05:00");
    assert_zone!(Some(offset(-18_001)), "00:00:00-05:00:01");
    assert_zone!(Some(offset(-18_060)), "00:00:00-05:01:00");
    assert_zone!(Some(offset(50_400)), "00:00:00+14:00");
    assert_zone!(Some(offset(-50_400)), "00:00:00-14:00");
    assert_zone!(None, "00:00:00+14:01");
    assert_zone!(None, "00:00:00+14:00:01");
    assert_zone!(None, "00:00:00-14:01");
    assert_zone!(None, "00:00:00-14:00:01");
    assert_zone!(Some(FeelZone::Zone("Europe/Warsaw".to_string())), "00:00:00@Europe/Warsaw");
    assert_zone!(None, "00:00:00@abc/xyz");
  }

  #[test]
  fn zone_designators() {
    assert_eq!(Some(FeelZone::Utc), FeelZone::from_designator("Z"));
    assert_eq!(Some(FeelZone::Utc), FeelZone::from_designator("z"));
    assert_eq!(Some(FeelZone::Utc), FeelZone::from_designator("-00:00"));
    assert_eq!(Some(7_200), offset_seconds(FeelZone::from_designator("+02:00")));
    assert_eq!(Some(-19_800), offset_seconds(FeelZone::from_designator("-05:30")));
    assert_eq!(Some(3_630), offset_seconds(FeelZone::from_designator("+01:00:30")));
    assert_eq!(Some(50_400), offset_seconds(FeelZone::from_designator("+14:00")));
    assert_eq!(Some(FeelZone::Zone("Europe/Paris".to_string())), FeelZone::from_designator("@Europe/Paris"));
    for designator in [
      "",
      " Z",
      "Z ",
      "@",
      "+14:01",
      "-14:00:01",
      "+15:00",
      "+2:00",
      "@Mars/Olympus",
      "@America/Port-au-Prince",
      "@@Europe/Paris",
      "Z@Europe/Paris",
      "+02:00Z",
      "Europe/Paris",
      "10:00:00Z",
    ] {
      assert_eq!(None, FeelZone::from_designator(designator), "{designator:?}");
    }
  }

  #[test]
  fn test_eq() {
    assert!((FeelZone::Local == FeelZone::Local));
    assert!((FeelZone::Local != FeelZone::Utc));
    assert!((FeelZone::Local != offset(18_400)));
    assert!((FeelZone::Local != FeelZone::Zone("Europe/Warsaw".to_string())));
    assert!((FeelZone::Utc == FeelZone::Utc));
    assert!((FeelZone::Utc != offset(18_400)));
    assert!((FeelZone::Utc != FeelZone::Zone("Europe/Warsaw".to_string())));
    assert!((offset(1) == offset(1)));
    assert!((offset(1) != FeelZone::Zone("Europe/Warsaw".to_string())));
    assert!((offset(2) != FeelZone::Zone("Europe/Sofa".to_string())));
    assert!((offset(1) == offset(2)));
    assert!((FeelZone::Zone("Europe/Warsaw".to_string()) == FeelZone::Zone("Europe/Warsaw".to_string())));
    assert!((FeelZone::Zone("Europe/Warsaw".to_string()) != FeelZone::Zone("Australia/Sydney".to_string())));
  }

  #[test]
  fn test_debug() {
    assert_eq!(r#"Local"#, format!("{:?}", FeelZone::Local));
    assert_eq!(r#"Utc"#, format!("{:?}", FeelZone::Utc));
    assert_eq!(r#"Offset(FeelOffset(18000))"#, format!("{:?}", offset(18_000)));
    assert_eq!(r#"Zone("Europe/Warsaw")"#, format!("{:?}", FeelZone::Zone("Europe/Warsaw".to_string())));
  }
}
