//! The numbers of temporal values: the `value` property (DMN 1.6 §10.3.2.3.4–§10.3.2.3.8, Table
//! 66), `valuet`, `valuedt`, `valuedtd` and `valueymd` of the spec, and the `second` and `seconds`
//! properties, decimals in the interval [0..60) (Table 67).

use rheon_feel::diagnostics;
use rheon_feel::message::{arg, compose};
use rheon_feel::values::Value;
use rheon_feel::{FeelNumber, value_null};
use rheon_feel_temporal::{FeelDate, FeelDateTime, FeelDaysAndTimeDuration, FeelTime, FeelYearsAndMonthsDuration, FeelZone};

const NANOS_IN_SECOND: i64 = 1_000_000_000;
const SECONDS_IN_DAY: i64 = 86_400;

/// Seconds (with the fractional nanoseconds) as an exact number.
fn seconds(nanos: i64) -> Value {
  let whole = nanos.div_euclid(NANOS_IN_SECOND);
  let fraction = nanos.rem_euclid(NANOS_IN_SECOND);
  let n = FeelNumber::from(whole);
  Value::Number(if fraction == 0 { n } else { n + FeelNumber::new(fraction, 9) })
}

/// `valueymd⁻¹` (DMN 1.6 §10.3.2.3.8): the years and months duration of `months`, the product
/// or the quotient of Table 60 as FEEL computes it, `None` beyond the range of a duration. A
/// fraction of a month is truncated toward zero, as the DMN TCK expects, and the evaluation
/// warns that the operation `left operator right` dropped it.
pub(super) fn ym_duration(months: FeelNumber, left: &Value, operator: char, right: &Value) -> Option<FeelYearsAndMonthsDuration> {
  let duration = i64::try_from(&months).ok().and_then(FeelYearsAndMonthsDuration::from_m)?;
  if !months.is_integer() {
    warn_dropped_months(months.frac(), left, operator, right);
  }
  Some(duration)
}

/// Records the warning that the operation `left operator right` dropped the fraction of a month
/// `dropped`; a warning is the rare path, kept out of the arithmetic.
#[cold]
#[inline(never)]
fn warn_dropped_months(dropped: FeelNumber, left: &Value, operator: char, right: &Value) {
  diagnostics::warn(
    diagnostics::WarningCode::MonthsTruncated,
    &compose(format_args!(
      "fractional months of '{} {operator} {}' truncated to whole months; dropped fraction: '{}'",
      arg(left),
      arg(right),
      arg(&dropped)
    )),
  );
}

/// The second of a minute with its fraction, from the whole seconds and the nanoseconds:
/// `second` of a time or a date and time and `seconds` of a days and time duration.
pub(super) fn second(whole: u64, nanos: u64) -> Value {
  seconds((whole * NANOS_IN_SECOND as u64 + nanos) as i64)
}

/// `valuedtd`: a days and time duration as a number of seconds.
pub(super) fn of_dt_duration(duration: &FeelDaysAndTimeDuration) -> Value {
  seconds(duration.as_nanos())
}

/// `valuedt`: seconds since 1970-01-01T00:00:00Z of a date and time that has a UTC form (an
/// offset or a time zone); a local date and time has none, so its value is null.
pub(super) fn of_date_time(date_time: &FeelDateTime) -> Value {
  if !date_time.time().zone().has_offset() {
    return value_null!("value: a local date and time has no UTC form");
  }
  let Some(epoch) = FeelDateTime::utc(1970, 1, 1, 0, 0, 0, 0) else {
    return value_null!("value: epoch");
  };
  match date_time.nanos_since(&epoch) {
    Some(nanos) => seconds(nanos),
    None => value_null!("value: date and time out of range"),
  }
}

/// A date is taken as the date and time at UTC midnight (§10.3.2.3.5).
pub(super) fn of_date(date: &FeelDate) -> Value {
  of_date_time(&FeelDateTime::new(date.clone(), FeelTime::midnight(FeelZone::Utc)))
}

/// `valuet`: seconds since midnight in [0..86400). A time with an offset is taken in its UTC
/// form, which keeps `value` consistent with time subtraction; a local time uses its clock time,
/// and a time in a zone whose offset is not known has no value (null), never the value of UTC.
pub(super) fn of_time(time: &FeelTime) -> Value {
  let clock = (time.hour() as i64 * 3600 + time.minute() as i64 * 60 + time.second() as i64) * NANOS_IN_SECOND + time.nanos() as i64;
  let offset = match (time.zone(), time.feel_time_offset()) {
    (FeelZone::Local, _) => 0,
    (_, Some(offset)) => offset as i64 * NANOS_IN_SECOND,
    (_, None) => return value_null!("value: the time zone of the time has no known offset"),
  };
  seconds((clock - offset).rem_euclid(SECONDS_IN_DAY * NANOS_IN_SECOND))
}

#[cfg(test)]
mod tests {
  use super::*;

  fn time(zone: FeelZone) -> FeelTime {
    FeelTime::zone_opt(10, 0, 0, 500_000_000, zone).unwrap()
  }

  /// DMN 1.6 §10.3.2.3.4: "If no time offset is specified, the time is interpreted as a local
  /// time of day", so a local time is valued by its clock time; a time in a zone is valued in
  /// its UTC form, and a zone whose offset cannot be resolved gives no value instead of the
  /// value of UTC. Such a zone is not reachable from an expression whose result does not depend
  /// on the date of the evaluation (a zone name is checked when it is read, and a time in a
  /// gap of daylight saving time resolves on the current date).
  #[test]
  fn the_value_of_a_time_needs_its_offset() {
    assert_eq!(Value::Number("36000.5".parse().unwrap()), of_time(&time(FeelZone::Local)));
    assert_eq!(Value::Number("36000.5".parse().unwrap()), of_time(&time(FeelZone::Utc)));
    let plus_one = FeelTime::offset_opt(10, 0, 0, 500_000_000, 3600).unwrap();
    assert_eq!(Value::Number("32400.5".parse().unwrap()), of_time(&plus_one));
    assert_eq!(Value::Number("32400.5".parse().unwrap()), of_time(&time(FeelZone::Zone("Etc/GMT-1".to_string()))));
    assert_eq!(
      Value::Null(Some("value: the time zone of the time has no known offset".to_string())),
      of_time(&time(FeelZone::Zone("No/Such_Zone".to_string())))
    );
  }
}
