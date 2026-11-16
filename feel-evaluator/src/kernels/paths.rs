//! Names and paths: a name that no scope holds, a path `e.name` and a descendant path `e...name`
//! (DMN 1.6 Table 65 and Table 66).

use super::temporal_value;
use rheon_feel::Name;
use rheon_feel::bif::Bif;
use rheon_feel::bip::*;
use rheon_feel::context::FeelContext;
use rheon_feel::message::arg;
use rheon_feel::value_null;
use rheon_feel::values::{TypeOf, Value};
use rheon_feel_temporal::FeelDaysAndTimeDuration;
use std::str::FromStr;

/// The value of a name that no scope holds: the built-in function of that name, or null with an
/// error.
pub(crate) fn free_name(name: &Name) -> Value {
  if let Ok(bif) = Bif::from_str(&name.to_string()) {
    Value::BuiltInFunction(bif)
  } else {
    value_null!("context has no value for key '{}'", arg(name))
  }
}

/// The value of a qualified name that no scope holds.
pub(crate) fn free_qualified_name() -> Value {
  value_null!("no value for qualified name")
}

/// FEEL `lhv.name`: the entry of a context, the entries of the contexts of a list (null for an
/// item that has none), or a property of another value.
pub(crate) fn path(lhv: Value, name: &Name) -> Value {
  match lhv {
    Value::Context(context) => {
      if let Some(value) = context.get_entry(name) {
        value.clone()
      } else {
        value_null!(note: "build_path: no entry {} in context: {}", arg(name), arg(&context))
      }
    }
    Value::List(items) => {
      let mut result = vec![];
      for item in items {
        if let Value::Context(context) = item {
          if let Some(value) = context.get_entry(name) {
            result.push(value.clone());
          } else {
            result.push(value_null!());
          }
        } else {
          result.push(property(item, name));
        }
      }
      Value::List(result.into())
    }
    other => property(other, name),
  }
}

/// `e...name` (DMN 1.6, grammar rule 68, DMN16-72): the values of every entry named `name` in the
/// context `value` and in all its nested contexts, as a list, each matching entry before the entries
/// nested in its value. A value that is no context is null.
pub(crate) fn descendants(value: Value, name: &Name) -> Value {
  fn collect(context: &FeelContext, name: &Name, result: &mut Vec<Value>) {
    for (key, value) in context.iter() {
      if key == name {
        result.push(value.clone());
      }
      if let Value::Context(nested) = value {
        collect(nested, name, result);
      }
    }
  }
  match value {
    Value::Context(context) => {
      let mut result = vec![];
      collect(&context, name, &mut result);
      Value::List(result.into())
    }
    other => value_null!("descendant expression: expected context, actual type is {}", arg(&TypeOf(&other))),
  }
}

/// A property of a value: of a temporal value, a range or a unary comparison (DMN 1.6 Table 66).
fn property(value: Value, name: &Name) -> Value {
  let property = Bip::new(name.as_str());
  match value {
    Value::Date(date) => match property {
      Bip::Year => Value::Number(date.year().into()),
      Bip::Month => Value::Number(date.month().into()),
      Bip::Day => Value::Number(date.day().into()),
      Bip::Weekday => {
        if let Some(day_of_week) = date.day_of_week() {
          Value::Number(day_of_week.1.into())
        } else {
          value_null!("could not retrieve weekday for date")
        }
      }
      Bip::Value => temporal_value::of_date(&date),
      other => value_null!("no such property in date: {}", arg(&other)),
    },
    Value::DateTime(date_time) => match property {
      Bip::Year => Value::Number(date_time.year().into()),
      Bip::Month => Value::Number(date_time.month().into()),
      Bip::Day => Value::Number(date_time.day().into()),
      Bip::Weekday => {
        if let Some(day_of_week) = date_time.day_of_week() {
          Value::Number(day_of_week.1.into())
        } else {
          value_null!("could not retrieve weekday for date and time")
        }
      }
      Bip::Hour => Value::Number(date_time.hour().into()),
      Bip::Minute => Value::Number(date_time.minute().into()),
      Bip::Second => temporal_value::second(date_time.second().into(), date_time.nanos()),
      Bip::TimeOffset => {
        if let Some(offset) = date_time.feel_time_offset() {
          Value::DaysAndTimeDuration(FeelDaysAndTimeDuration::from_s(offset))
        } else {
          value_null!("could not retrieve time offset for date and time")
        }
      }
      Bip::Timezone => {
        if let Some(feel_time_zone) = date_time.feel_time_zone() {
          Value::String(feel_time_zone.into())
        } else {
          value_null!("could not retrieve timezone for date and time")
        }
      }
      Bip::Value => temporal_value::of_date_time(&date_time),
      other => value_null!("no such property in date and time: {}", arg(&other)),
    },
    Value::Time(time) => match property {
      Bip::Hour => Value::Number(time.hour().into()),
      Bip::Minute => Value::Number(time.minute().into()),
      Bip::Second => temporal_value::second(time.second().into(), time.nanos()),
      Bip::TimeOffset => {
        if let Some(offset) = time.feel_time_offset() {
          Value::DaysAndTimeDuration(FeelDaysAndTimeDuration::from_s(offset))
        } else {
          value_null!("could not retrieve time offset for time")
        }
      }
      Bip::Timezone => {
        if let Some(feel_time_zone) = time.feel_time_zone() {
          Value::String(feel_time_zone.into())
        } else {
          value_null!("could not retrieve timezone for time")
        }
      }
      Bip::Value => temporal_value::of_time(&time),
      other => value_null!("no such property in time: {}", arg(&other)),
    },
    Value::DaysAndTimeDuration(dt_duration) => match property {
      Bip::Days => Value::Number(dt_duration.get_days().into()),
      Bip::Hours => Value::Number(dt_duration.get_hours().into()),
      Bip::Minutes => Value::Number(dt_duration.get_minutes().into()),
      Bip::Seconds => temporal_value::second(dt_duration.get_seconds() as u64, dt_duration.get_nanoseconds()),
      Bip::Value => temporal_value::of_dt_duration(&dt_duration),
      other => value_null!("no such property in days and time duration: {}", arg(&other)),
    },
    Value::YearsAndMonthsDuration(ym_duration) => match property {
      Bip::Years => Value::Number(ym_duration.years().into()),
      Bip::Months => Value::Number(ym_duration.months().into()),
      Bip::Value => Value::Number(ym_duration.as_months().into()),
      other => value_null!("no such property in years and months duration: {}", arg(&other)),
    },
    Value::Range(rs, cs, re, ce) => match property {
      Bip::Start => *rs,
      Bip::StartIncluded => Value::Boolean(cs.closed()),
      Bip::End => *re,
      Bip::EndIncluded => Value::Boolean(ce.closed()),
      other => value_null!("no such property in range: {}", arg(&other)),
    },
    Value::UnaryEqual(value) => match property {
      Bip::Start => *value,
      Bip::End => *value,
      Bip::StartIncluded | Bip::EndIncluded => Value::Boolean(true),
      other => value_null!("no such property in unary equal: {}", arg(&other)),
    },
    Value::UnaryNotEqual(value) => match property {
      Bip::Start => *value,
      Bip::End => *value,
      Bip::StartIncluded | Bip::EndIncluded => Value::Boolean(true),
      other => value_null!("no such property in unary not equal: {}", arg(&other)),
    },
    Value::UnaryLess(value) => match property {
      Bip::End => *value,
      Bip::StartIncluded | Bip::EndIncluded => Value::Boolean(false),
      other => value_null!("no such property in unary less: {}", arg(&other)),
    },
    Value::UnaryLessOrEqual(value) => match property {
      Bip::End => *value,
      Bip::StartIncluded => Value::Boolean(false),
      Bip::EndIncluded => Value::Boolean(true),
      other => value_null!("no such property in unary less or equal: {}", arg(&other)),
    },
    Value::UnaryGreater(value) => match property {
      Bip::Start => *value,
      Bip::StartIncluded | Bip::EndIncluded => Value::Boolean(false),
      other => value_null!("no such property in unary greater: {}", arg(&other)),
    },
    Value::UnaryGreaterOrEqual(value) => match property {
      Bip::Start => *value,
      Bip::StartIncluded => Value::Boolean(true),
      Bip::EndIncluded => Value::Boolean(false),
      other => value_null!("no such property in unary greater or equal: {}", arg(&other)),
    },
    v @ Value::Null(_) => v,
    other => value_null!("unexpected type: {}, for property: {}", arg(&TypeOf(&other)), arg(&property)),
  }
}
