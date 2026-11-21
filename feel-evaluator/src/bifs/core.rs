//! # Core implementation of build-in functions

use crate::bfeel::Dialect;
use crate::bifs::open_ranges::{self, Relation};
use crate::equality::{Equality, Seen};
use crate::evaluate_range_literal;
use crate::invocation::call_with;
use crate::kernels::{Order, order};
use crate::macros::invalid_argument_type;
use dsntk_common::DsntkError;
use rheon_feel::context::FeelContext;
use rheon_feel::diagnostics::{self, Abort};
use rheon_feel::message::{arg, compose};
use rheon_feel::values::{FeelList, TypeOf, VALUE_FALSE, VALUE_TRUE, Value, warn_truncated_fraction};
use rheon_feel::{FeelNumber, FeelScope, Name, ToFeelString, value_null, value_number, value_string};
use rheon_feel_number::RoundingMode;
use rheon_feel_regex::{Patterns, RegexError};
use rheon_feel_temporal::{
  DayOfWeek, DayOfYear, FeelDate, FeelDateTime, FeelDaysAndTimeDuration, FeelOffset, FeelTime, FeelYearsAndMonthsDuration, FeelZone, MonthOfYear, WeekOfYear,
};
use std::cmp::Ordering;
use std::str::FromStr;

/// Returns the absolute value of the argument.
pub fn abs(value: &Value) -> Value {
  match value {
    Value::Number(v) => Value::Number(v.abs()),
    Value::DaysAndTimeDuration(v) => Value::DaysAndTimeDuration(v.abs()),
    Value::YearsAndMonthsDuration(v) => Value::YearsAndMonthsDuration(v.abs()),
    _ => invalid_argument_type!("abs", "number", value),
  }
}

/// Returns `true` when value1 **is after** value2.
pub fn after(value1: &Value, value2: &Value) -> Value {
  open_ranges::relation(Relation::After, value1, value2)
}

/// Returns `false` if any item is `false`, `true` if empty or all items are true, else `null`.
pub fn all(values: &[Value]) -> Value {
  // "return false if any item is false, true if empty or all items are true, else null":
  // a false item decides even after a null or a non-boolean one (all([false,null,true]) = false)
  if values.iter().any(|value| matches!(value, Value::Boolean(false))) {
    return VALUE_FALSE;
  }
  if values.iter().all(|value| matches!(value, Value::Boolean(true))) {
    return VALUE_TRUE;
  }
  value_null!()
}

/// Returns `true` if any item is `true`, `false` if empty or all items are `false`, else `null`.
pub fn any(values: &[Value]) -> Value {
  // a true item decides even after a null or a non-boolean one (any([false,null,true]) = true)
  if values.iter().any(|value| matches!(value, Value::Boolean(true))) {
    return VALUE_TRUE;
  }
  if values.iter().all(|value| matches!(value, Value::Boolean(false))) {
    return VALUE_FALSE;
  }
  value_null!()
}

/// Returns new list with items appended.
pub fn append(list: &Value, values: &[Value]) -> Value {
  match list {
    Value::List(items) => {
      let mut appended = items.clone();
      for value in values {
        appended.push(value.clone());
      }
      Value::List(appended)
    }
    v @ Value::Null(_) => v.clone(),
    other => invalid_argument_type!("append", "list", other),
  }
}

/// Returns `true` when value1 **is before** value2.
pub fn before(value1: &Value, value2: &Value) -> Value {
  open_ranges::relation(Relation::Before, value1, value2)
}

/// Returns `n` with the given scale and rounding mode ceiling (DMN 1.6 Table 77).
pub fn ceiling(n: &Value, scale: &Value) -> Value {
  round_to_scale(n, scale, RoundingMode::Ceiling, "ceiling", "ceiling")
}

/// Returns `true` when two point are equal or two ranges are equal.
pub fn coincides(value1: &Value, value2: &Value) -> Value {
  open_ranges::relation(Relation::Coincides, value1, value2)
}

/// Returns new list that is a concatenation of the arguments.
pub fn concatenate(values: &[Value]) -> Value {
  let mut concatenated = vec![];
  for value in values {
    if let Value::List(items) = value {
      for item in items {
        concatenated.push(item.clone());
      }
    } else {
      return invalid_argument_type!("concatenate", "list", value);
    }
  }
  Value::List(concatenated.into())
}

/// Returns `true` when the input string contains the match.
pub fn contains(input_string_value: &Value, match_string_value: &Value) -> Value {
  if let Value::String(input_string) = input_string_value {
    if let Value::String(match_string) = match_string_value {
      Value::Boolean(input_string.contains(match_string.as_str()))
    } else {
      invalid_argument_type!("contains", "string", match_string_value)
    }
  } else {
    invalid_argument_type!("contains", "string", input_string_value)
  }
}

pub fn context(entries: &Value) -> Value {
  match entries {
    Value::List(entries) => {
      let mut result_ctx = FeelContext::default();
      for entry in entries {
        match entry {
          Value::Context(ctx) => {
            let Some(key) = ctx.get_entry(&"key".into()) else {
              return value_null!("context: no 'key' entry in context {}", arg(&ctx));
            };
            let Some(value) = ctx.get_entry(&"value".into()) else {
              return value_null!("context: no 'value' entry in context {}", arg(&ctx));
            };
            let key: Name = match key {
              Value::String(key) => key.as_str().into(),
              _ => return value_null!("context: 'key' entry is not a string, actual type is {}", arg(&TypeOf(key))),
            };
            if result_ctx.contains_entry(&key) {
              return value_null!("context: duplicated key: {}", arg(&key));
            }
            result_ctx.set_entry(&key, value.clone());
          }
          _ => return value_null!("context: invalid entry, expected context, actual type is {}", arg(&TypeOf(entry))),
        }
      }
      // the keys are strings: one in another form than FEEL reads a name is found by its name
      result_ctx.note_key_forms();
      result_ctx.into()
    }
    Value::Context(ctx) => {
      let mut result_ctx = FeelContext::default();
      let Some(key) = ctx.get_entry(&"key".into()) else {
        return value_null!("context: no 'key' entry in context {}", arg(&ctx));
      };
      let Some(value) = ctx.get_entry(&"value".into()) else {
        return value_null!("context: no 'value' entry in context {}", arg(&ctx));
      };
      let key: Name = match key {
        Value::String(key) => key.as_str().into(),
        _ => return value_null!("context: 'key' entry is not a string, actual type is {}", arg(&TypeOf(key))),
      };
      result_ctx.set_entry(&key, value.clone());
      // the keys are strings: one in another form than FEEL reads a name is found by its name
      result_ctx.note_key_forms();
      result_ctx.into()
    }
    null @ Value::Null(_) => null.clone(),
    other => invalid_argument_type!("context", "list or context", other),
  }
}

pub fn context_put(context: &Value, keys: &Value, value: &Value) -> Value {
  match context {
    Value::Context(ctx) => {
      // a copy of the context's own entries; the values in them stay shared
      let mut result_ctx = FeelContext::clone(ctx);
      match keys {
        Value::String(key) => result_ctx.set_entry(&key.as_str().into(), value.clone()),
        Value::List(items) => {
          if items.is_empty() {
            return value_null!();
          }
          let mut names = vec![];
          for item in items {
            match item {
              Value::String(s) => names.push(s.as_str().into()),
              null @ Value::Null(_) => return null.clone(),
              other => return invalid_argument_type!("context put", "string", other),
            }
          }
          // The composite definition (DMN 1.7 Beta 1, 10.3.4.10) puts under key' the result of
          // context put(context.key', keys', value). For a key' the context does not have, that
          // inner context is null, outside the domain, so key' gets null: the key list builds no
          // context. An entry on the path that holds a value that is no context makes the whole
          // result null, as TCK 1146 nested009 reads it.
          let mut at: &FeelContext = ctx;
          for (i, name) in names[..names.len() - 1].iter().enumerate() {
            match at.get_entry(name) {
              Some(Value::Context(inner)) => at = inner,
              Some(other) => return value_null!("context put: the entry '{}' on the key path holds {}, not a context", name, arg(&TypeOf(other))),
              None => {
                let missing = value_null!("context put: the context has no entry '{}' to put into", name);
                if let Err(reason) = result_ctx.apply_entries(&names[..=i], missing) {
                  return value_null!("context put: {}", reason);
                }
                result_ctx.note_key_forms();
                return result_ctx.into();
              }
            }
          }
          if let Err(reason) = result_ctx.apply_entries(&names, value.clone()) {
            return value_null!("context put: {}", reason);
          }
        }
        null @ Value::Null(_) => return null.clone(),
        other => return invalid_argument_type!("context put", "string or list<string>", other),
      }
      // the keys are strings: one in another form than FEEL reads a name is found by its name
      result_ctx.note_key_forms();
      result_ctx.into()
    }
    null @ Value::Null(_) => null.clone(),
    other => invalid_argument_type!("context put", "context", other),
  }
}

pub fn context_merge(contexts: &Value) -> Value {
  match contexts {
    Value::List(items) => {
      // the domain is a list of contexts (DMN 1.7 Beta 1, 10.3.4.10): the empty list merges to the
      // empty context, as context([]) is {}, and an item that is no context, null included, is
      // outside the domain
      let mut result_ctx = FeelContext::default();
      for item in items {
        match item {
          Value::Context(ctx) => result_ctx.zip(ctx),
          other => return invalid_argument_type!("context merge", "context", other),
        }
      }
      // the keys are strings: one in another form than FEEL reads a name is found by its name
      result_ctx.note_key_forms();
      result_ctx.into()
    }
    Value::Context(ctx) => Value::Context(ctx.clone()),
    null @ Value::Null(_) => null.clone(),
    other => invalid_argument_type!("context merge", "list<context>", other),
  }
}

/// Returns size of list, or zero if list is empty. A value that is no list is a list of one item (the
/// conversion to a singleton list, DMN 1.7 Beta 1, 10.3.2.9.4); null is no list of anything, so
/// `count(null)` is null, as `reverse(null)` and the other list functions are.
pub fn count(list: &Value) -> Value {
  match list {
    Value::List(items) => Value::Number(items.len().into()),
    Value::Null(_) => invalid_argument_type!("count", "list", list),
    _ => Value::Number(1.into()),
  }
}

/// Returns date converted from string or date and time.
pub fn date_1(value: &Value) -> Value {
  match value {
    Value::String(text) => {
      if let Ok(date) = FeelDate::from_str(text) {
        Value::Date(date)
      } else {
        value_null!("[core::date] invalid date string '{}'", arg(&text))
      }
    }
    Value::Date(date) => Value::Date(date.clone()),
    Value::DateTime(date_time) => Value::Date(date_time.date()),
    _ => invalid_argument_type!("date", "string, date or date and time", value),
  }
}

/// Returns date created from year, month and day.
pub fn date_3(year_value: &Value, month_value: &Value, day_value: &Value) -> Value {
  if let Value::Number(year) = year_value {
    if let Value::Number(month) = month_value {
      if let Value::Number(day) = day_value {
        if let Ok(date) = FeelDate::try_from((*year, *month, *day)) {
          Value::Date(date)
        } else {
          value_null!("[core::date_3] invalid date y={} m={} d={}", arg(&year), arg(&month), arg(&day))
        }
      } else {
        invalid_argument_type!("date", "number (day)", day_value)
      }
    } else {
      invalid_argument_type!("date", "number (month)", month_value)
    }
  } else {
    invalid_argument_type!("date", "number (year)", year_value)
  }
}

/// Returns date and time created from string.
pub fn date_and_time_1(value: &Value) -> Value {
  if let Value::String(text) = value {
    if let Ok((date_time, dropped)) = FeelDateTime::parse_reporting_truncation(text) {
      warn_truncated_fraction(text.as_str(), dropped);
      return Value::DateTime(date_time);
    }
    if let Some(date_time) = FeelDate::from_str(text)
      .ok()
      .and_then(|date| FeelDateTime::try_new(date, FeelTime::midnight(FeelZone::Local)))
    {
      return Value::DateTime(date_time);
    }
    value_null!("[core::date and time] invalid date or date and time '{}'", arg(&text))
  } else {
    invalid_argument_type!("date and time", "string", value)
  }
}

/// The date and time of a date and a time; null for a date whose year has no instants
/// ([FeelDateTime::try_new]).
fn date_time_of(date: FeelDate, time: FeelTime) -> Value {
  let year = date.year();
  match FeelDateTime::try_new(date, time) {
    Some(date_time) => Value::DateTime(date_time),
    None => value_null!("[core::date and time] expected a year from -262143 to 262142, found the year {}", year),
  }
}

/// Returns date and time created from date and time.
pub fn date_and_time_2(date_value: &Value, time_value: &Value) -> Value {
  match date_value {
    Value::DateTime(date_time) => {
      if let Value::Time(time) = time_value {
        return date_time_of(date_time.date(), time.clone());
      }
      invalid_argument_type!("date and time", "time", time_value)
    }
    Value::Date(date) => {
      if let Value::Time(time) = time_value {
        return date_time_of(date.clone(), time.clone());
      }
      invalid_argument_type!("date and time", "time", time_value)
    }
    _ => invalid_argument_type!("date and time", "date and time or date", date_value),
  }
}

/// `date and time(date, time, timezone)` (DMN 1.6 §10.3.4.1, Table 73): the date (or the date part
/// of a date and time), a time without a time zone, and a time zone given as "Z", an offset such as
/// "+02:00", or an IANA zone identifier. The result is built from these parts: the date, and the
/// time of day in the zone that the designator names by the rules of the zone suffix of a literal.
/// It is what the literal `date and time("<date>T<time><zone>")` gives, and the same value for a
/// date whose year no literal can write (from -999 to 999).
pub fn date_and_time_3(date_value: &Value, time_value: &Value, timezone_value: &Value) -> Value {
  let date = match date_value {
    Value::Date(date) => date.clone(),
    Value::DateTime(date_time) => date_time.date(),
    other => return invalid_argument_type!("date and time", "date and time or date", other),
  };
  let time = match time_value {
    Value::Time(time) if !time.zone().has_offset() => time,
    Value::Time(_) => return value_null!("[core::date and time] the time must not have a time zone when a timezone is given"),
    other => return invalid_argument_type!("date and time", "time", other),
  };
  let Value::String(zone) = timezone_value else {
    return invalid_argument_type!("date and time", "string", timezone_value);
  };
  let zone = zone.trim();
  let zone_in_time = if zone.eq_ignore_ascii_case("z") || zone.starts_with('+') || zone.starts_with('-') {
    FeelZone::from_designator(zone)
  } else {
    FeelZone::from_designator(&format!("@{zone}"))
  };
  match zone_in_time.and_then(|zone| FeelTime::zone_opt(time.hour(), time.minute(), time.second(), time.nanos(), zone)) {
    Some(time) => date_time_of(date, time),
    None => value_null!("[core::date and time] invalid timezone '{}'", arg(&zone)),
  }
}

/// Returns the day of the week according to the Gregorian calendar enumeration:
/// `Monday`, `Tuesday`, `Wednesday`, `Thursday`, `Friday`, `Saturday`, `Sunday`.
pub fn day_of_week(value: &Value) -> Value {
  fn gregorian_day(opt_day_of_week: Option<DayOfWeek>) -> Value {
    if let Some(day_of_week) = opt_day_of_week {
      value_string!(day_of_week.0)
    } else {
      value_null!("[day of week] no weekday")
    }
  }
  match value {
    Value::Date(date) => gregorian_day(date.day_of_week()),
    Value::DateTime(date_time) => gregorian_day(date_time.day_of_week()),
    _ => invalid_argument_type!("day of week", "date, date and time", value),
  }
}

/// Returns the day of the year.
pub fn day_of_year(value: &Value) -> Value {
  fn gregorian_day_of_year(opt_day_of_year: Option<DayOfYear>) -> Value {
    if let Some(day_of_year) = opt_day_of_year {
      value_number!(day_of_year)
    } else {
      value_null!("[day of year] no day of year")
    }
  }
  match value {
    Value::Date(date) => gregorian_day_of_year(date.day_of_year()),
    Value::DateTime(date_time) => gregorian_day_of_year(date_time.day_of_year()),
    _ => invalid_argument_type!("day of year", "date, date and time", value),
  }
}

/// Returns `number` with the given `scale`, rounded half-even (DMN 1.6 Table 77).
pub fn decimal(number_value: &Value, scale_value: &Value) -> Value {
  if let Value::Number(number) = number_value {
    if let Value::Number(scale) = scale_value {
      let scale = scale.trunc();
      match i32::try_from(&scale).map(|s| number.round_to(s, RoundingMode::HalfEven)) {
        Ok(Ok(rounded)) => Value::Number(rounded),
        _ => value_null!("[core::decimal] scale is out of range: {}", arg(&scale)),
      }
    } else {
      value_null!("[core::decimal] scale value is not a number: {}", arg(&scale_value))
    }
  } else {
    value_null!("[core::decimal] number value is not a number: {}", arg(&number_value))
  }
}

/// Returns new list with removed duplicates.
pub fn distinct_values(value: &Value) -> Value {
  if let Value::List(items) = value {
    let mut seen = Seen::default();
    let mut result = vec![];
    for item in items {
      if seen.insert(item) {
        result.push(item.clone())
      }
    }
    Value::List(result.into())
  } else {
    invalid_argument_type!("distinct values", "list", value)
  }
}

/// Converts string value to a days and time or years and months duration.
pub fn duration(value: &Value) -> Value {
  if let Value::String(s) = value {
    if let Some(ym_duration) = FeelYearsAndMonthsDuration::parse(s.as_str()) {
      Value::YearsAndMonthsDuration(ym_duration)
    } else if let Ok((dt_duration, dropped)) = FeelDaysAndTimeDuration::parse_reporting_truncation(s) {
      warn_truncated_fraction(s.as_str(), dropped);
      Value::DaysAndTimeDuration(dt_duration)
    } else {
      value_null!("duration: invalid duration literal: '{}'", arg(&s))
    }
  } else {
    invalid_argument_type!("duration", "string", value)
  }
}

/// Returns `true` when a point is during the range or the first range is during the second.
pub fn during(value1: &Value, value2: &Value) -> Value {
  open_ranges::relation(Relation::During, value1, value2)
}

/// Returns `true` when the input string ends with specified match string.
pub fn ends_with(input_string_value: &Value, match_string_value: &Value) -> Value {
  if let Value::String(input_string) = input_string_value {
    if let Value::String(match_string) = match_string_value {
      Value::Boolean(input_string.ends_with(match_string.as_str()))
    } else {
      invalid_argument_type!("ends with", "string", match_string_value)
    }
  } else {
    invalid_argument_type!("ends with", "string", input_string_value)
  }
}

/// Returns true if number is even, false if it is odd.
pub fn even(number_value: &Value) -> Value {
  if let Value::Number(number) = number_value {
    Value::Boolean(number.even())
  } else {
    value_null!("even")
  }
}

/// Returns the Euler’s number e raised to the power of **value** given as a parameter.
pub fn exp(value: &Value) -> Value {
  if let Value::Number(num) = value {
    if let Some(n) = num.exp() {
      Value::Number(n)
    } else {
      value_null!("exp: result is not a finite number (decimal overflow)")
    }
  } else {
    invalid_argument_type!("exp", "number", value)
  }
}

/// Returns `true` when point is the end of a range or ranges have the same end value.
pub fn finishes(value1: &Value, value2: &Value) -> Value {
  open_ranges::relation(Relation::Finishes, value1, value2)
}

/// Returns `true` when two ranges or range and point have the same ending point.
pub fn finished_by(value1: &Value, value2: &Value) -> Value {
  open_ranges::relation(Relation::FinishedBy, value1, value2)
}

/// Returns new list with flattened nested lists.
pub fn flatten(value: &Value) -> Value {
  if let Value::List(_) = value {
    let mut flattened = vec![];
    flatten_value(value, &mut flattened);
    Value::List(flattened.into())
  } else {
    invalid_argument_type!("flatten", "list", value)
  }
}

/// Flattens nested lists.
fn flatten_value(value: &Value, flattened: &mut Vec<Value>) {
  if let Value::List(items) = value {
    for item in items {
      if let Value::List(_) = item {
        flatten_value(item, flattened);
      } else {
        flattened.push(item.clone())
      }
    }
  }
}

/// Returns `n` with the given scale and rounding mode flooring (DMN 1.6 Table 77).
pub fn floor(n: &Value, scale: &Value) -> Value {
  round_to_scale(n, scale, RoundingMode::Floor, "floor", "floor")
}

pub fn get_entries(context: &Value) -> Value {
  if let Value::Context(ctx) = context {
    let name_key: Name = "key".into();
    let name_value: Name = "value".into();
    let mut entries = vec![];
    ctx.get_entries().iter().for_each(|(name, value)| {
      let mut key_value_pair = FeelContext::default();
      key_value_pair.set_entry(&name_key, Value::String(name.to_string().into()));
      key_value_pair.set_entry(&name_value, (**value).clone());
      entries.push(key_value_pair.into());
    });
    Value::List(entries.into())
  } else {
    invalid_argument_type!("get entries", "context", context)
  }
}

pub fn get_value(context: &Value, key: &Value) -> Value {
  if let Value::Context(ctx) = context {
    if let Value::String(entry_key) = key {
      let name = Name::from(entry_key.as_str());
      if let Some(entry_value) = ctx.get_entry(&name) {
        entry_value.clone()
      } else {
        value_null!()
      }
    } else {
      invalid_argument_type!("get value", "string", key)
    }
  } else {
    invalid_argument_type!("get value", "context", context)
  }
}

pub fn includes(value1: &Value, value2: &Value) -> Value {
  open_ranges::relation(Relation::Includes, value1, value2)
}

/// Return ascending list of list positions containing match.
pub fn index_of(list: &Value, element: &Value) -> Value {
  if let Value::List(items) = list {
    let mut equality = Equality::default();
    let mut indexes = vec![];
    for (i, item) in items.iter().enumerate() {
      if equality.compare(item, element) == Some(true) {
        indexes.push(Value::Number((i + 1).into()));
      }
    }
    Value::List(indexes.into())
  } else {
    invalid_argument_type!("index of", "list", list)
  }
}

/// `insert after(list, position, newItem)` (DMN 1.7 beta 1, DMN17-100): a new list with
/// `newItem` inserted after the item at `position` (1-based; negative positions count from the end).
pub fn insert_after(list: &Value, position_value: &Value, new_item_value: &Value) -> Value {
  if let Value::List(items) = list
    && let Value::Number(position) = position_value
  {
    let mut items = items.to_vec();
    // a fractional position is truncated toward zero first (DMN 1.6 §10.3.2.9.4, DMN16-82), so -0.5 is 0
    let position = &position.trunc();
    // Positions are converted through 64-bit integers, so 32-bit targets (wasm32) behave like 64-bit ones.
    if position.is_positive()
      && let Ok(i) = u64::try_from(position)
      && i >= 1
      && i <= items.len() as u64
    {
      items.insert(i as usize, new_item_value.clone());
      return Value::List(items.into());
    }
    if position.is_negative()
      && let Ok(i) = u64::try_from(position.abs())
      && i >= 1
      && i <= items.len() as u64
    {
      items.insert(items.len() - i as usize + 1, new_item_value.clone());
      return Value::List(items.into());
    }
  }
  value_null!("index is out of range")
}

/// `insert before(list, position, newItem)`: a new list with `newItem` inserted before the item
/// at `position` (1-based; negative positions count from the end).
pub fn insert_before(list: &Value, position_value: &Value, new_item_value: &Value) -> Value {
  if let Value::List(items) = list
    && let Value::Number(position) = position_value
  {
    let mut items = items.to_vec();
    // a fractional position is truncated toward zero first (DMN 1.6 §10.3.2.9.4, DMN16-82), so -0.5 is 0
    let position = &position.trunc();
    // Positions are converted through 64-bit integers, so 32-bit targets (wasm32) behave like 64-bit ones.
    if position.is_positive()
      && let Ok(i) = u64::try_from(position)
      && i >= 1
      && i <= items.len() as u64
    {
      items.insert(i as usize - 1, new_item_value.clone());
      return Value::List(items.into());
    }
    if position.is_negative()
      && let Ok(i) = u64::try_from(position.abs())
      && i <= items.len() as u64
    {
      items.insert(items.len() - i as usize, new_item_value.clone());
      return Value::List(items.into());
    }
  }
  value_null!("index is out of range")
}

/// Returns `true` if both values are the same element in the FEEL semantic domain.
pub fn is(value1: &Value, value2: &Value) -> Value {
  match value1 {
    Value::Number(v1) => match value2 {
      Value::Number(v2) => Value::Boolean(v1 == v2),
      _ => Value::Boolean(false),
    },
    Value::String(v1) => match value2 {
      Value::String(v2) => Value::Boolean(v1 == v2),
      _ => Value::Boolean(false),
    },
    Value::Boolean(v1) => match value2 {
      Value::Boolean(v2) => Value::Boolean(v1 == v2),
      _ => Value::Boolean(false),
    },
    Value::Date(v1) => match value2 {
      Value::Date(v2) => Value::Boolean(v1 == v2),
      _ => Value::Boolean(false),
    },
    Value::Time(v1) => match value2 {
      Value::Time(v2) => Value::Boolean(v1.is(v2)),
      _ => Value::Boolean(false),
    },
    Value::DateTime(v1) => match value2 {
      Value::DateTime(v2) => Value::Boolean(v1.is(v2)),
      _ => Value::Boolean(false),
    },
    Value::DaysAndTimeDuration(v1) => match value2 {
      Value::DaysAndTimeDuration(v2) => Value::Boolean(v1 == v2),
      _ => Value::Boolean(false),
    },
    Value::YearsAndMonthsDuration(v1) => match value2 {
      Value::YearsAndMonthsDuration(v2) => Value::Boolean(v1 == v2),
      _ => Value::Boolean(false),
    },
    _ => invalid_argument_type!("is", "scalar", value1),
  }
}

/// Returns `true` when the list contain the specified element.
pub fn list_contains(list: &Value, element: &Value) -> Value {
  if let Value::List(items) = list {
    let mut equality = Equality::default();
    for item in items {
      if equality.compare(item, element) == Some(true) {
        return VALUE_TRUE;
      }
    }
    VALUE_FALSE
  } else {
    invalid_argument_type!("list contains", "list", list)
  }
}

pub fn list_replace(list: &Value, position_or_match: &Value, new_item: &Value) -> Value {
  let mut items = match list {
    Value::List(items) => items.to_vec(),
    Value::Null(_) => return invalid_argument_type!("list replace", "list", list),
    other => vec![other.clone()],
  };
  match position_or_match {
    Value::Number(position) => {
      if let Ok(index) = position.trunc().try_into() as Result<i64, _> {
        let len = items.len();
        match index.cmp(&0) {
          Ordering::Greater => {
            if index.unsigned_abs() <= len as u64 {
              let index = index as usize;
              items[index - 1] = new_item.clone();
              Value::List(items.into())
            } else {
              value_null!("position exceeds list bounds")
            }
          }
          Ordering::Less => {
            let index = index.unsigned_abs();
            if index <= len as u64 {
              items[len - index as usize] = new_item.clone();
              Value::List(items.into())
            } else {
              value_null!("position exceeds list bounds")
            }
          }
          _ => value_null!("position must not be zero"),
        }
      } else {
        value_null!("position exceeds integer range")
      }
    }
    Value::FunctionDefinition(function) => {
      if function.parameters().len() != 2 {
        return value_null!("list replace: matching function must have exactly two arguments");
      }
      let mut elements = vec![];
      // one empty scope for every call: a call leaves the scope as it found it
      let scope = FeelScope::default();
      for item in items {
        let matched = call_with(&scope, function, [item.copied(), new_item.copied()], Dialect::Feel);
        // the answer of a call in a stopped evaluation is not read; the null is never observed
        if diagnostics::stopped() {
          return Value::Null(None);
        }
        match matched {
          Value::Boolean(true) => elements.push(new_item.clone()),
          Value::Boolean(false) => elements.push(item),
          _ => return value_null!("list replace: matching function must return boolean value"),
        }
      }
      Value::List(elements.into())
    }
    _ => value_null!("position must be number or function"),
  }
}

/// Returns the natural logarithm (base `e`) of the number parameter.
pub fn log(number: &Value) -> Value {
  if let Value::Number(num) = number {
    if *num <= FeelNumber::zero() {
      value_null!("log: argument must be a positive number")
    } else if let Some(num_log) = num.ln() {
      Value::Number(num_log)
    } else {
      value_null!("log: result is not a finite number (decimal overflow)")
    }
  } else {
    invalid_argument_type!("log", "number", number)
  }
}

/// Returns lower-cased string.
pub fn lower_case(input_string_value: &Value) -> Value {
  match input_string_value {
    Value::List(items) => {
      if items.len() == 1
        && let Value::String(input_string) = &items[0]
      {
        return Value::String(input_string.to_lowercase().into());
      }
    }
    Value::String(input_string) => return Value::String(input_string.to_lowercase().into()),
    _ => {}
  }
  invalid_argument_type!("lower case", "string", input_string_value)
}

/// Returns `true` when the input matches the regex pattern with the flags (DMN 1.6, Table 75); a
/// null `flags` is no flags.
pub fn matches(input: &Value, pattern: &Value, flags: &Value) -> Value {
  // The input must be a string.
  let Value::String(input) = input else {
    return invalid_argument_type!("matches", "string", input);
  };
  // The pattern must be a string.
  let Value::String(pattern) = pattern else {
    return invalid_argument_type!("matches", "string", pattern);
  };
  // Flags if present must be a string.
  let flags = match flags {
    Value::Null(_) => "",
    Value::String(flags) => flags.as_str(),
    other => return invalid_argument_type!("matches", "string", other),
  };
  match diagnostics::kept(|patterns: &mut Patterns| rheon_feel_regex::is_match_with(patterns, input, pattern, flags)) {
    Ok(value) => Value::Boolean(value),
    Err(error) => regex_failure("matches", pattern, error, |reason| value_null!("{}", reason)),
  }
}

/// The result of a regex built-in that gives no answer. DMN 1.6, Table 75 note 2: "where XPath
/// specifies an error result, FEEL specifies a null result". An invalid pattern is the null that
/// `invalid_pattern` makes of its reason; invalid flags, an invalid replacement string and a
/// pattern of `replace` or `split` that matches the zero-length string are a null with their
/// reason.
///
/// A regex engine that gives up on a valid pattern stops the evaluation: an implementation limit
/// is not an F&O error, so it has no FEEL result; the evaluation reports the abort instead
/// (§7.3.8). The null returned then is never observed.
fn regex_failure(function: &str, pattern: &str, error: RegexError, invalid_pattern: impl FnOnce(DsntkError) -> Value) -> Value {
  match error {
    RegexError::InvalidPattern(reason) => invalid_pattern(reason),
    RegexError::InvalidArgument(reason) => value_null!("{}", reason),
    RegexError::LimitExceeded(limit) => {
      let reason = compose(format_args!("{function}: the pattern '{}' exceeds the regex engine's {limit}", arg(pattern)));
      diagnostics::abort(Abort::limit(reason));
      Value::Null(None)
    }
  }
}

/// Returns the largest of the values (DMN 1.7 Beta 1, 10.3.4.4): items of one ordered type, compared
/// as `>` compares them. An empty list, an item that is null or not of that type, and two items that
/// have no order give null, whatever their position (10.3.4: a parameter outside its domain).
pub fn max(values: &[Value]) -> Value {
  extreme("max", values, std::cmp::Ordering::Greater)
}

/// Returns the mean of numbers.
pub fn mean(values: &[Value]) -> Value {
  if values.is_empty() {
    return value_null!();
  }
  let mut sum = FeelNumber::zero();
  for value in values {
    if let Value::Number(n) = value {
      sum += *n;
    } else {
      return invalid_argument_type!("mean", "number", value);
    }
  }
  Value::Number(sum / values.len().into())
}

/// Returns `true` when range1 `meets` range2.
pub fn meets(value1: &Value, value2: &Value) -> Value {
  open_ranges::relation(Relation::Meets, value1, value2)
}

/// Returns the median of numbers.
pub fn median(values: &[Value]) -> Value {
  if values.is_empty() {
    return value_null!();
  }
  let mut list = vec![];
  for value in values {
    if let Value::Number(n) = value {
      list.push(*n);
    } else {
      return value_null!("median");
    }
  }
  list.sort_by(|x, y| x.partial_cmp(y).unwrap_or(Ordering::Equal));
  let index = values.len() / 2;
  if list.len() % 2 == 0 {
    Value::Number((list[index - 1] + list[index]) / FeelNumber::two())
  } else {
    Value::Number(list[index])
  }
}

/// Returns `true` when range2 is `met by` range1.
pub fn met_by(value1: &Value, value2: &Value) -> Value {
  open_ranges::relation(Relation::MetBy, value1, value2)
}

/// Returns the smallest of the values (DMN 1.7 Beta 1, 10.3.4.4), as [max] the largest.
pub fn min(values: &[Value]) -> Value {
  extreme("min", values, std::cmp::Ordering::Less)
}

/// The item of `values` that is `wanted` to every other one under FEEL's order ([order]).
fn extreme(function: &str, values: &[Value], wanted: std::cmp::Ordering) -> Value {
  let Some(mut best) = values.first() else {
    return value_null!();
  };
  for value in values {
    match order(value, best) {
      Order::Ordered(ordering) if ordering == wanted => best = value,
      Order::Ordered(_) => {}
      Order::Indeterminate => {
        return value_null!("core", function, "expected items that have an order, found {} and {}", arg(best), arg(value));
      }
      Order::Unordered => {
        let expected = if values.iter().all(|value| matches!(value, Value::Null(_))) || !is_ordered(best) {
          "number, string, date, time, date and time, or duration".to_string()
        } else {
          TypeOf(best).to_string()
        };
        let culprit = if is_ordered(best) { value } else { best };
        return invalid_argument_type!(function, expected, culprit);
      }
    }
  }
  best.clone()
}

/// Whether `value` is of one of the ordered types (Table 53).
fn is_ordered(value: &Value) -> bool {
  matches!(
    value,
    Value::Number(_) | Value::String(_) | Value::Date(_) | Value::DateTime(_) | Value::Time(_) | Value::DaysAndTimeDuration(_) | Value::YearsAndMonthsDuration(_)
  )
}

/// Returns the mode of numbers.
pub fn mode(values: &[Value]) -> Value {
  if values.is_empty() {
    return Value::List(FeelList::new());
  }
  // make sure all values are numbers and prepare the list of them
  let mut list = vec![];
  for value in values {
    if let Value::Number(n) = value {
      list.push(*n);
    } else {
      return invalid_argument_type!("mode", "number", value);
    }
  }
  // sort values in ascending order
  list.sort_by(|x, y| x.partial_cmp(y).unwrap_or(Ordering::Equal));
  // calculate the frequencies of the numbers
  let mut mode: Vec<(usize, FeelNumber)> = vec![];
  for x in list {
    if let Some((count, value)) = mode.pop() {
      if x == value {
        mode.push((count + 1, value));
      } else {
        mode.push((count, value));
        mode.push((1_usize, x));
      }
    } else {
      mode.push((1_usize, x));
    }
  }
  // sort frequencies in descending order, and when equal then by number in ascending order
  mode.sort_by(|x, y| match x.0.cmp(&y.0).reverse() {
    Ordering::Equal => x.1.partial_cmp(&y.1).unwrap_or(Ordering::Equal),
    other => other,
  });
  // there is minimum one element in the list, so unwrap is ok
  let max = mode.first().unwrap().0;
  // return items with maximum frequency
  Value::List(mode.iter().filter_map(|(c, v)| if *c == max { Some(Value::Number(*v)) } else { None }).collect())
}

/// Returns the remainder of the division of dividend by divisor: `dividend -
/// divisor*floor(dividend/divisor)` over the exact values, rounded once (DMN 1.6 Table 77).
pub fn modulo(dividend_value: &Value, divisor_value: &Value) -> Value {
  if let Value::Number(dividend) = *dividend_value {
    if let Value::Number(divisor) = *divisor_value {
      match dividend.modulo(&divisor) {
        Some(remainder) => Value::Number(remainder),
        None => value_null!("[core::modulo] division by zero"),
      }
    } else {
      invalid_argument_type!("modulo", "number", divisor_value)
    }
  } else {
    invalid_argument_type!("modulo", "number", dividend_value)
  }
}

/// ???
fn gregorian_month(opt_month_of_year: Option<MonthOfYear>) -> Value {
  if let Some(month_of_year) = opt_month_of_year {
    value_string!(month_of_year.0)
  } else {
    value_null!("[month of year] no month")
  }
}

/// Returns the month of the year according to the Gregorian calendar enumeration:
/// `January`, `February`, `March`, `April`, `May`, `June`, `July`,
/// `August`, `September`, `October`, `November`, `December`.
pub fn month_of_year(value: &Value) -> Value {
  match value {
    Value::Date(date) => gregorian_month(date.month_of_year()),
    Value::DateTime(date_time) => gregorian_month(date_time.month_of_year()),
    _ => invalid_argument_type!("month of year", "date, date and time", value),
  }
}

/// Logical negation.
pub fn not(negand: &Value) -> Value {
  if let Value::Boolean(v) = negand {
    Value::Boolean(!(*v))
  } else {
    invalid_argument_type!("not", "boolean", negand)
  }
}

/// Returns current date and time.
pub fn now() -> Value {
  Value::DateTime(FeelDateTime::now())
}

/// `true` when `s` is a FEEL numeric literal (grammar rule 35):
/// `[ "-" ] , ( digits , [ ".", digits ] | "." , digits ), [ ( "e" | "E" ) , [ "+" | "-" ] , digits ]`.
fn is_numeric_literal(s: &str) -> bool {
  let b = s.as_bytes();
  let mut i = 0;
  let digits = |i: &mut usize| {
    let start = *i;
    while *i < b.len() && b[*i].is_ascii_digit() {
      *i += 1;
    }
    *i > start
  };
  if i < b.len() && b[i] == b'-' {
    i += 1;
  }
  let int = digits(&mut i);
  if i < b.len() && b[i] == b'.' {
    i += 1;
    if !digits(&mut i) {
      return false;
    }
  } else if !int {
    return false;
  }
  if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
    i += 1;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
      i += 1;
    }
    if !digits(&mut i) {
      return false;
    }
  }
  i == b.len()
}

/// `number(from)` (DMN 1.6, DMN16-208 and DMN16-105): a string converts as
/// `number(from, null, null)`, a number is returned unchanged.
pub fn number_single(from: &Value) -> Value {
  match from {
    Value::Number(_) => from.clone(),
    Value::String(_) => number(from, &value_null!(), &value_null!()),
    Value::Null(_) => value_null!(),
    other => invalid_argument_type!("number", "string or number", other),
  }
}

/// Converts string to a number.
/// Grouping...
pub fn number(from: &Value, grouping_separator: &Value, decimal_separator: &Value) -> Value {
  // function for converting string to Value::Number; the text must be a FEEL numeric literal
  // (grammar rule 35; the 1.6 footnote's "rule 37" is a stale reference), so "+5" or "1." are null
  let convert = |value: &str| {
    if !is_numeric_literal(value) {
      return value_null!("[core::number] not a FEEL numeric literal: {}", arg(&value));
    }
    match value.parse::<FeelNumber>() {
      Ok(number) => Value::Number(number),
      Err(reason) => value_null!("[core::number] {}", reason),
    }
  };
  match from {
    Value::String(value) => {
      // prepare grouping separator from Value::String ot VALUE_NULL
      let grouping_sep = match grouping_separator {
        Value::String(s) => match s.as_str() {
          " " | "." | "," => Some(s.as_str()),
          _ => return value_null!("[core::number] grouping separator must be space, period, comma or null"),
        },
        Value::Null(_) => None,
        _ => return value_null!("[core::number] grouping separator must be space, period, comma or null"),
      };
      // prepare decimal separator from Value::String ot VALUE_NULL
      let decimal_sep = match decimal_separator {
        Value::String(s) => match s.as_str() {
          "." | "," => Some(s.as_str()),
          _ => return value_null!("[core::number] decimal separator must be period, comma or null"),
        },
        Value::Null(_) => None,
        _ => return value_null!("[core::number] decimal separator must be period, comma or null"),
      };
      // replace both separators and try to convert
      if let Some(grp_sep) = &grouping_sep
        && let Some(dec_sep) = &decimal_sep
      {
        return if *grp_sep != *dec_sep {
          convert(&value.replace(grp_sep, "").replace(dec_sep, "."))
        } else {
          value_null!("[core::number] decimal separator must be different from grouping separator")
        };
      }
      // replace grouping separator and try to convert
      if decimal_sep.is_none()
        && let Some(sep) = grouping_sep
      {
        return convert(&value.replace(sep, ""));
      }
      // replace decimal separator and try to convert
      if grouping_sep.is_none()
        && let Some(sep) = decimal_sep
      {
        return convert(&value.replace(sep, "."));
      }
      // try to convert an input parameter without replacing
      convert(value)
    }
    _ => invalid_argument_type!("number", "string", from),
  }
}

/// Returns **true** if number is odd, **false** if it is even.
pub fn odd(value: &Value) -> Value {
  if let Value::Number(v) = value {
    Value::Boolean(v.odd())
  } else {
    invalid_argument_type!("odd", "number", value)
  }
}

/// Returns `true` when two ranges overlap.
pub fn overlaps(value1: &Value, value2: &Value) -> Value {
  open_ranges::relation(Relation::Overlaps, value1, value2)
}

/// Returns `true` when first range overlaps the end of the second range.
pub fn overlaps_after(value1: &Value, value2: &Value) -> Value {
  open_ranges::relation(Relation::OverlapsAfter, value1, value2)
}

/// Returns `true` when first range overlaps the beginning of the second range.
pub fn overlaps_before(value1: &Value, value2: &Value) -> Value {
  open_ranges::relation(Relation::OverlapsBefore, value1, value2)
}

/// Returns the product of numbers.
pub fn product(values: &[Value]) -> Value {
  if values.is_empty() {
    return value_null!();
  }
  let mut list = vec![];
  for value in values {
    if let Value::Number(n) = value {
      list.push(*n);
    } else {
      return invalid_argument_type!("product", "number", value);
    }
  }
  Value::Number(list.iter().fold(FeelNumber::one(), |acc, n| acc * (*n)))
}

/// Returns already parsed range or null value.
pub fn range(value: &Value) -> Value {
  match value {
    v @ Value::Null(_) => v.clone(),
    Value::String(range_literal) => {
      let scope = FeelScope::default();
      if let Ok(range) = evaluate_range_literal(&scope, range_literal) {
        if range.is_valid_range() { range } else { value_null!("invalid range") }
      } else {
        value_null!("invalid range literal")
      }
    }
    other => invalid_argument_type!("range", "string", other),
  }
}

/// ???
pub fn remove(list: &Value, position_value: &Value) -> Value {
  if let Value::List(items) = list
    && let Value::Number(position_number) = position_value
  {
    let mut items = items.to_vec();
    // a fractional position is truncated toward zero first (DMN 1.6 §10.3.2.9.4, DMN16-82), so -0.5 is 0
    let position_number = &position_number.trunc();
    if position_number.is_positive()
      && let Ok(position) = u64::try_from(position_number)
      && let Some(index) = position.checked_sub(1)
      && index < items.len() as u64
    {
      items.remove(index as usize);
      return Value::List(items.into());
    }
    if position_number.is_negative()
      && let Ok(index) = u64::try_from(position_number.abs())
      && index <= items.len() as u64
    {
      items.remove(items.len() - index as usize);
      return Value::List(items.into());
    }
  }
  value_null!("probably index is out of range")
}

/// Replaces the matches of a pattern in a string (DMN 1.6, Table 75): null for an invalid pattern,
/// invalid flags, an invalid replacement string or a pattern that matches the zero-length string;
/// a null `flags_string_value` is no flags.
pub fn replace(input_string_value: &Value, pattern_string_value: &Value, replacement_string_value: &Value, flags_string_value: &Value) -> Value {
  let Value::String(input_string) = input_string_value else {
    return value_null!("replace: input must be a string");
  };
  let Value::String(pattern_string) = pattern_string_value else {
    return value_null!("replace: pattern must be a string");
  };
  let Value::String(replacement_string) = replacement_string_value else {
    return value_null!("replace: replacement must be a string");
  };
  let flags = match flags_string_value {
    Value::Null(_) => "",
    Value::String(flags_string) => flags_string.as_str(),
    _ => return value_null!("replace: flags must be a string"),
  };
  match diagnostics::kept(|patterns: &mut Patterns| rheon_feel_regex::replace_with(patterns, input_string, pattern_string, replacement_string, flags)) {
    Ok(result) => Value::String(result.into()),
    Err(error) => regex_failure("replace", pattern_string, error, |_| value_null!("replace: invalid pattern")),
  }
}

pub fn reverse(list: &Value) -> Value {
  if let Value::List(items) = list {
    let mut items = items.to_vec();
    items.reverse();
    Value::List(items.into())
  } else {
    invalid_argument_type!("reverse", "list", list)
  }
}

/// Returns `n` with the given scale and rounding mode round down (DMN 1.6 Table 77).
pub fn round_down(n: &Value, scale: &Value) -> Value {
  round_to_scale(n, scale, RoundingMode::Down, "round down", "round_down")
}

/// Returns `n` with the given scale and rounding mode round half down (DMN 1.6 Table 77).
pub fn round_half_down(n: &Value, scale: &Value) -> Value {
  round_to_scale(n, scale, RoundingMode::HalfDown, "round half down", "round_half_down")
}

/// Returns `n` with the given scale and rounding mode round half up (DMN 1.6 Table 77).
pub fn round_half_up(n: &Value, scale: &Value) -> Value {
  round_to_scale(n, scale, RoundingMode::HalfUp, "round half up", "round_half_up")
}

/// Returns `n` with the given scale and rounding mode round up (DMN 1.6 Table 77).
pub fn round_up(n: &Value, scale: &Value) -> Value {
  round_to_scale(n, scale, RoundingMode::Up, "round up", "round_up")
}

/// The rounding built-ins of DMN 1.6 Table 77 with a scale: `n` rounded to `scale` fraction
/// digits by `mode`. `name` is the built-in's name and `site` its name in the reasons of a null:
/// an argument that is not a number, a scale that is not an integer in the range of `i32`
/// (footnote 2: a decimal scale is converted to an integer first), or a scale outside
/// `-6111..=6176` (footnote 1).
fn round_to_scale(n: &Value, scale: &Value, mode: RoundingMode, name: &str, site: &str) -> Value {
  let Value::Number(n) = n else {
    return invalid_argument_type!(name, "number", n);
  };
  let Value::Number(scale) = scale else {
    return invalid_argument_type!(name, "number", scale);
  };
  let Ok(scale): Result<i32, DsntkError> = scale.try_into() else {
    return value_null!("[core::{}] invalid scale: {}", site, arg(&scale));
  };
  match n.round_to(scale, mode) {
    Ok(rounded) => Value::Number(rounded),
    Err(reason) => value_null!("[core::{}] {}", site, reason),
  }
}

pub fn sort(list: &Value, ordering_function: &Value) -> Value {
  if let Value::List(items) = list {
    if let Value::FunctionDefinition(function) = ordering_function {
      if function.parameters().len() == 2 {
        // an answer other than `true` (false, null, any other value) means "does not precede";
        // once the evaluation was stopped the ordering ends, and its null is never observed
        // one empty scope for every call: a call leaves the scope as it found it
        let scope = FeelScope::default();
        let order = super::sorting::order_by(items.len(), |a, b| {
          if diagnostics::stopped() {
            return None;
          }
          Some(matches!(
            call_with(&scope, function, [items[a].copied(), items[b].copied()], Dialect::Feel),
            Value::Boolean(true)
          ))
        });
        match order {
          Some(order) => Value::List(order.into_iter().map(|position| items[position].clone()).collect()),
          None => Value::Null(None),
        }
      } else {
        value_null!("sort: ordering function should take exactly two arguments")
      }
    } else {
      value_null!("sort: expected ordering function definition as a second argument")
    }
  } else {
    value_null!("sort: expected a list of values as a first argument")
  }
}

/// Splits a string at the matches of a delimiter pattern (DMN 1.6, Table 75): null for an invalid
/// pattern or a delimiter that matches the zero-length string.
pub fn split(input_string_value: &Value, delimiter_string_value: &Value) -> Value {
  let Value::String(input_string) = input_string_value else {
    return value_null!("split: input must be a string");
  };
  let Value::String(delimiter_string) = delimiter_string_value else {
    return value_null!("split: delimiter must be a string");
  };
  match diagnostics::kept(|patterns: &mut Patterns| rheon_feel_regex::split_with(patterns, input_string, delimiter_string)) {
    Ok(parts) => Value::List(parts.into_iter().map(|part| Value::String(part.into())).collect()),
    Err(error) => regex_failure("split", delimiter_string, error, |_| value_null!("split: invalid delimiter")),
  }
}

/// Returns the square root of the given [Value].
///
/// When the given number is negative, this function returns [Value::Null].
pub fn sqrt(value: &Value) -> Value {
  if let Value::Number(v) = value {
    if *v >= FeelNumber::zero() {
      if let Some(result) = v.sqrt() {
        Value::Number(result)
      } else {
        value_null!("sqrt: result is not a finite number")
      }
    } else {
      value_null!("sqrt: argument must be positive number or zero")
    }
  } else {
    value_null!("sqrt: argument must be a number")
  }
}

/// Returns `true` when range is _started_ by point or other range.
pub fn started_by(value1: &Value, value2: &Value) -> Value {
  open_ranges::relation(Relation::StartedBy, value1, value2)
}

pub fn starts(value1: &Value, value2: &Value) -> Value {
  open_ranges::relation(Relation::Starts, value1, value2)
}

/// Returns `true` when the input string starts with specified match string.
pub fn starts_with(input_string_value: &Value, match_string_value: &Value) -> Value {
  if let Value::String(input_string) = input_string_value {
    if let Value::String(match_string) = match_string_value {
      Value::Boolean(input_string.starts_with(match_string.as_str()))
    } else {
      invalid_argument_type!("starts with", "string", match_string_value)
    }
  } else {
    invalid_argument_type!("starts with", "string", input_string_value)
  }
}

/// Returns the sample standard deviation of the list of numbers. An item that is no number is an
/// error; fewer than two numbers give the null that DMN 1.6 §10.3.4.4 defines ("If the list is
/// empty or if the list contains only one element, the function returns null"), with a note.
pub fn stddev(values: &[Value]) -> Value {
  let mut sum = FeelNumber::zero();
  let mut numbers = Vec::with_capacity(values.len());
  for value in values {
    if let Value::Number(x) = *value {
      sum += x;
      numbers.push(x);
    } else {
      return value_null!("stddev: expected number, actual type is {} with value {}", arg(&TypeOf(value)), arg(&value));
    }
  }
  if numbers.len() < 2 {
    return value_null!(note: "stddev: minimum two input arguments expected");
  }
  let n: FeelNumber = numbers.len().into();
  let avg = sum / n;
  let mut sum2 = FeelNumber::zero();
  for number in numbers {
    if let Some(square) = (number - avg).square() {
      sum2 += square;
    } else {
      return value_null!("stddev: intermediate result is not a finite number");
    }
  }
  if let Some(stddev) = (sum2 / (n - FeelNumber::one())).sqrt() {
    Value::Number(stddev)
  } else {
    value_null!("stddev: result is not a finite number")
  }
}

/// Converts specified value to string.
pub fn string(value: &Value) -> Value {
  match value {
    Value::Null(_) => value_null!(),
    Value::String(s) => Value::String(s.clone()),
    other => Value::String(other.to_feel_string().into()),
  }
}

/// Returns the number of characters in string.
pub fn string_join(list: &Value, delimiter: &Value) -> Value {
  match list {
    Value::List(items) => {
      let delimiter = match delimiter {
        Value::String(delimiter) => delimiter.as_str(),
        Value::Null(_) => "",
        _ => return value_null!("string join: invalid delimiter, expected string, actual value type is {}", arg(&TypeOf(delimiter))),
      };
      // null items are ignored (DMN 1.7 Beta 1, 10.3.4.3), so a delimiter goes only between strings
      let mut joined_string = String::new();
      let mut first = true;
      for item in items.iter() {
        match item {
          Value::String(s) => {
            if !first {
              joined_string.push_str(delimiter);
            }
            first = false;
            joined_string.push_str(s);
          }
          Value::Null(_) => {}
          other => return invalid_argument_type!("string join", "string", other),
        }
      }
      Value::String(joined_string.into())
    }
    Value::String(string) => Value::String(string.clone()),
    other => value_null!("string join: expected list or string, actual value type is {}", arg(&TypeOf(other))),
  }
}

/// Returns the number of characters in string.
pub fn string_length(input_string_value: &Value) -> Value {
  if let Value::String(input_string) = input_string_value {
    Value::Number(input_string.chars().count().into())
  } else {
    value_null!("string length: expected string as an argument")
  }
}

/// Returns the sum of values in the collection of numbers.
pub fn sum(values: &[Value]) -> Value {
  if values.is_empty() {
    return value_null!();
  }
  if let Value::Number(n) = values[0] {
    let mut sum = n;
    for value in values.iter().skip(1) {
      if let Value::Number(v) = *value {
        sum += v;
      } else {
        return invalid_argument_type!("sum", "number", value);
      }
    }
    Value::Number(sum)
  } else {
    invalid_argument_type!("sum", "number", values[0])
  }
}

/// Returns list of all elements of list, starting with specified position, 1st position is 1, last position is -1.
pub fn sublist2(list: &Value, position_value: &Value) -> Value {
  // Positions are converted through 64-bit integers and checked with wide (i128) arithmetic,
  // so 32-bit targets (wasm32) give the same results and messages as 64-bit ones and no
  // computation can overflow.
  if let Value::List(items) = list {
    if let Value::Number(position_number) = position_value {
      let len = items.len() as i128;
      // a fractional position is truncated toward zero first (DMN 1.6 §10.3.2.9.4, DMN16-82), so -0.5 is 0
      let position_number = &position_number.trunc();
      if position_number.is_positive() {
        if let Ok(position) = u64::try_from(position_number) {
          let index = position as i128 - 1;
          if index >= 0 && index < len {
            Value::List(items[index as usize..].to_vec().into())
          } else {
            value_null!("sublist: position is out of range, len = {}, position = {}", items.len(), position)
          }
        } else {
          value_null!("sublist: invalid position value: {}", arg(position_number))
        }
      } else if position_number.is_negative() {
        if let Ok(position) = u64::try_from(position_number.abs()) {
          if (position as i128) <= len {
            Value::List(items[items.len() - position as usize..].to_vec().into())
          } else {
            value_null!("sublist: position is out of range, len = {}, position = -{}", items.len(), position)
          }
        } else {
          value_null!("sublist: invalid position value: {}", arg(position_number))
        }
      } else {
        value_null!("sublist: position must not be zero")
      }
    } else {
      value_null!("sublist: expected number, actual position value type is {}", arg(&TypeOf(position_value)))
    }
  } else {
    value_null!("sublist: expected list, actual value type is {}", arg(&TypeOf(list)))
  }
}

/// Returns list of specified length of elements of list, starting with specified position, 1st position is 1, last position is -1.
pub fn sublist3(list: &Value, position_value: &Value, length_value: &Value) -> Value {
  // See sublist2: 64-bit conversions and i128 arithmetic, no overflow on any target.
  if let Value::List(items) = list {
    if let Value::Number(length_number) = length_value {
      if let Ok(length) = u64::try_from(length_number) {
        if let Value::Number(position_number) = position_value {
          let len = items.len() as i128;
          // a fractional position is truncated toward zero first (DMN 1.6 §10.3.2.9.4, DMN16-82), so -0.5 is 0
          let position_number = &position_number.trunc();
          let first = if position_number.is_positive() {
            match u64::try_from(position_number) {
              Ok(position) => position as i128 - 1,
              Err(_) => return value_null!("sublist: invalid position value: {}", arg(position_number)),
            }
          } else if position_number.is_negative() {
            match u64::try_from(position_number.abs()) {
              Ok(position) => len - position as i128,
              Err(_) => return value_null!("sublist: invalid position value: {}", arg(position_number)),
            }
          } else {
            return value_null!("sublist: position must not be zero");
          };
          // the length is in [1..E] (DMN 1.7 Beta 1, 10.3.4.4, note 2), also after a decimal length
          // is converted to an integer (note 3): a length below 1 is outside the domain
          if length == 0 {
            return value_null!("sublist: length must be at least 1, actual length is {}", arg(length_number));
          }
          let last = first + length as i128;
          if first >= 0 && first < len && last <= len {
            Value::List(items[first as usize..last as usize].to_vec().into())
          } else {
            value_null!("sublist: invalid range, len = {}, start position = {}, end position = {}", items.len(), first + 1, last + 1)
          }
        } else {
          value_null!("sublist: expected number, actual position value type is {}", arg(&TypeOf(position_value)))
        }
      } else if length_number.trunc() < FeelNumber::one() {
        value_null!("sublist: length must be at least 1, actual length is {}", arg(length_number))
      } else {
        value_null!("sublist: invalid length value: {}", arg(length_number))
      }
    } else {
      value_null!("sublist: expected number, actual length value type is {}", arg(&TypeOf(length_value)))
    }
  } else {
    value_null!("sublist: expected list, actual value type is {}", arg(&TypeOf(list)))
  }
}

/// Returns `length` (or all) characters from string, starting at
/// `start_position`. First position is 1, last position is -1.
pub fn substring(input_string_value: &Value, start_position_value: &Value, length_value: &Value) -> Value {
  if let Value::String(input_string) = input_string_value {
    if let Value::Number(start_position_number) = start_position_value {
      // Positions are converted through 64-bit integers and checked with wide (i128) arithmetic,
      // so 32-bit targets (wasm32) behave like 64-bit ones and nothing can overflow.
      let Ok(position): Result<i64, DsntkError> = start_position_number.try_into() else {
        return value_null!("substring: invalid start position value: {}", arg(start_position_number));
      };
      let position = position as i128;
      let input_string_len = input_string.chars().count();
      let len = input_string_len as i128;
      match length_value {
        Value::Number(length_number) => {
          if *length_number < FeelNumber::one() {
            return value_null!("substring: length is less than 1");
          }
          let Ok(length): Result<u64, DsntkError> = length_number.trunc().try_into() else {
            return value_null!("substring: invalid length value: {}", arg(length_number));
          };
          let length = length as i128;
          match position.cmp(&0) {
            Ordering::Greater => {
              let first = position - 1;
              let last = first + length;
              if first < len && last <= len {
                Value::String(input_string.chars().skip(first as usize).take(length as usize).collect::<String>().into())
              } else {
                value_null!(
                  "substring: invalid range, len = {}, start position = {}, end position = {}",
                  input_string_len,
                  first + 1,
                  last + 1
                )
              }
            }
            Ordering::Less => {
              let first = len + position;
              let last = first + length;
              if first >= 0 && first < len && last <= len {
                Value::String(input_string.chars().skip(first as usize).take(length as usize).collect::<String>().into())
              } else {
                value_null!(
                  "substring: invalid range, len = {}, start position = {}, end position = {}",
                  input_string_len,
                  first + 1,
                  last + 1
                )
              }
            }
            Ordering::Equal => value_null!("substring: start position must not be zero"),
          }
        }
        Value::Null(_) => match position.cmp(&0) {
          Ordering::Greater => {
            let index = position - 1;
            if index < len {
              Value::String(input_string.chars().skip(index as usize).collect::<String>().into())
            } else {
              value_null!(
                "substring: position is out of range, len = {}, position = {}",
                input_string_len,
                arg(&start_position_number)
              )
            }
          }
          Ordering::Less => {
            let index = len + position;
            if index >= 0 {
              Value::String(input_string.chars().skip(index as usize).collect::<String>().into())
            } else {
              value_null!(
                "substring: position is out of range, len = {}, position = {}",
                input_string_len,
                arg(&start_position_number)
              )
            }
          }
          Ordering::Equal => value_null!("substring: start position must not be zero"),
        },
        other => {
          value_null!("substring: expected number, actual length type is {}", arg(&TypeOf(other)))
        }
      }
    } else {
      value_null!("substring: expected number, actual start position type is {}", arg(&TypeOf(start_position_value)))
    }
  } else {
    value_null!("substring: expected string, actual value type is {}", arg(&TypeOf(input_string_value)))
  }
}

/// Returns substring of `input_string_value` after the `match_input_string` in string.
pub fn substring_after(input_string_value: &Value, match_input_string: &Value) -> Value {
  if let Value::String(input_string) = input_string_value {
    if let Value::String(match_string) = match_input_string {
      if let Some(index) = input_string.find(match_string.as_str()) {
        Value::String(input_string[match_string.len() + index..].into())
      } else {
        Value::String("".into())
      }
    } else {
      value_null!("substring after: expected string, actual match type is: {}", arg(&TypeOf(match_input_string)))
    }
  } else {
    value_null!("substring after: expected string, actual input type is: {}", arg(&TypeOf(input_string_value)))
  }
}

/// Returns substring of `input_string_value` before the `match_input_string` in string.
pub fn substring_before(input_string_value: &Value, match_input_string: &Value) -> Value {
  if let Value::String(input_string) = input_string_value {
    if let Value::String(match_string) = match_input_string {
      if let Some(index) = input_string.find(match_string.as_str()) {
        Value::String(input_string[..index].into())
      } else {
        Value::String("".into())
      }
    } else {
      value_null!("substring before: expected string, actual match type is: {}", arg(&TypeOf(match_input_string)))
    }
  } else {
    value_null!("substring before: expected string, actual input type is: {}", arg(&TypeOf(input_string_value)))
  }
}

pub fn time_1(value: &Value) -> Value {
  match value {
    Value::String(text) => {
      if let Ok((time, dropped)) = FeelTime::parse_reporting_truncation(text) {
        warn_truncated_fraction(text.as_str(), dropped);
        Value::Time(time)
      } else {
        value_null!(
          "time: invalid time literal '{}', expected hh:mm:ss with an optional fraction and an optional Z, @zone or ±hh:mm offset within -14:00..+14:00",
          arg(&text)
        )
      }
    }
    Value::Date(_) => Value::Time(FeelTime::midnight(FeelZone::Utc)),
    Value::DateTime(date_time) => Value::Time(date_time.time()),
    Value::Time(time) => Value::Time(time.clone()),
    other => invalid_argument_type!("time", "string, date, date and time, time", other),
  }
}

/// The nanoseconds of the fraction of `second` (a number of seconds, 0 to 59), truncated, and the
/// digits beyond them when one of them is not zero: a time holds nanoseconds, so the seconds of
/// `time(h, m, s)` are truncated as the fraction of a time literal is, and warn the same way.
fn split_nanoseconds(second: &FeelNumber) -> (FeelNumber, Option<String>) {
  let nanoseconds = second.frac() * FeelNumber::billion();
  let beyond = nanoseconds.frac();
  let dropped = (!beyond.is_zero()).then(|| {
    // the plain text of a number between 0 and 1 is "0." and its digits
    let text = beyond.to_string();
    text.strip_prefix("0.").unwrap_or(&text).trim_end_matches('0').to_string()
  });
  (nanoseconds.trunc(), dropped)
}

pub fn time_3(hour_value: &Value, minute_value: &Value, second_value: &Value) -> Value {
  if let Value::Number(hour) = hour_value {
    if let Value::Number(minute) = minute_value {
      if let Value::Number(second) = second_value {
        if (0..24).contains(hour) {
          if (0..60).contains(minute) {
            if (0..60).contains(second) {
              // unwraps below are safe, value ranges are checked above
              let seconds = second.trunc();
              let (nanoseconds, dropped) = split_nanoseconds(second);
              let h = hour.try_into().unwrap();
              let m = minute.try_into().unwrap();
              let s = seconds.try_into().unwrap();
              let n = nanoseconds.try_into().unwrap();
              let feel_time = FeelTime::local_opt(h, m, s, n).unwrap();
              warn_truncated_fraction(second, dropped.as_deref());
              Value::Time(feel_time)
            } else {
              value_null!("second must be 0..59, current value is {}", arg(&second))
            }
          } else {
            value_null!("minute must be 0..59, current value is {}", arg(&minute))
          }
        } else {
          value_null!("hour must be 0..23, current value is {}", arg(&hour))
        }
      } else {
        value_null!("seconds must be a number, current type is {}", arg(&TypeOf(second_value)))
      }
    } else {
      value_null!("minutes must be a number, current type is {}", arg(&TypeOf(minute_value)))
    }
  } else {
    value_null!("hour must be a number, current type is {}", arg(&TypeOf(hour_value)))
  }
}

pub fn time_4(hour_value: &Value, minute_value: &Value, second_value: &Value, offset_value: &Value) -> Value {
  if let Value::Number(hour) = hour_value {
    if let Value::Number(minute) = minute_value {
      if let Value::Number(second) = second_value {
        if (0..24).contains(hour) {
          if (0..60).contains(minute) {
            if (0..60).contains(second) {
              let seconds = second.trunc();
              // unwraps below are safe, value ranges are checked above
              let h = hour.try_into().unwrap();
              let m = minute.try_into().unwrap();
              let s = seconds.try_into().unwrap();
              let (nanoseconds, dropped) = split_nanoseconds(second);
              let n = nanoseconds.try_into().unwrap();
              let time = match offset_value {
                // a time offset is whole seconds within ±14:00 (DMN 1.6 Table 67); any other
                // duration (duration("PT19H"), duration("PT0.5S")) gives null
                Value::DaysAndTimeDuration(offset) => match FeelOffset::from_duration(offset).and_then(|offset| FeelTime::zone_opt(h, m, s, n, FeelZone::from(offset))) {
                  Some(time) => time,
                  None => return value_null!("time: offset is out of range: {}", offset),
                },
                Value::Null(_) => match FeelTime::local_opt(h, m, s, n) {
                  Some(time) => time,
                  None => return value_null!("time: invalid local time"),
                },
                _ => return value_null!("expected days and time duration or null, current offset type is {}", arg(&TypeOf(offset_value))),
              };
              warn_truncated_fraction(second, dropped.as_deref());
              Value::Time(time)
            } else {
              value_null!("core", "time_4", "second must be 0..59, current value is: {}", arg(&second))
            }
          } else {
            value_null!("core", "time_4", "minute must be 0..59, current value is: {}", arg(&minute))
          }
        } else {
          value_null!("core", "time_4", "hour must be 0..23, current value is: {}", arg(&hour))
        }
      } else {
        value_null!("core", "time_4", "seconds must be a number, current type is: {}", arg(&TypeOf(second_value)))
      }
    } else {
      value_null!("core", "time_4", "minutes must be a number, current type is: {}", arg(&TypeOf(minute_value)))
    }
  } else {
    value_null!("core", "time_4", "hour must be a number, current type is: {}", arg(&TypeOf(hour_value)))
  }
}

/// Returns current date.
pub fn today() -> Value {
  Value::Date(FeelDate::today())
}

/// `intersection(list...)` (DMN 1.7 beta 1, DMN17-98): the elements common to all lists, duplicates
/// removed. The result keeps the order of the first list (the spec gives no ordering guarantee).
pub fn intersection(lists: &[Value]) -> Value {
  let mut inputs = vec![];
  for list in lists {
    match list {
      Value::List(items) => inputs.push(items),
      Value::Null(_) => return value_null!(),
      other => return invalid_argument_type!("intersection", "list", other),
    }
  }
  let mut others: Vec<Seen> = inputs[1..].iter().map(|other| Seen::of(other)).collect();
  let mut seen = Seen::default();
  let mut result: Vec<Value> = vec![];
  for item in inputs[0].iter() {
    if others.iter_mut().all(|other| other.contains(item)) && seen.insert(item) {
      result.push(item.clone());
    }
  }
  Value::List(result.into())
}

/// `trim(string)` (DMN 1.7 beta 1, DMN17-96): the string without leading and trailing white space,
/// where white space is grammar rule 61 (`vertical space`, U+0009, U+0020, U+0085, U+00A0, U+1680,
/// U+180E, U+2000..U+200B, U+2028, U+2029, U+202F, U+205F, U+3000, U+FEFF).
pub fn trim(value: &Value) -> Value {
  fn is_feel_white_space(ch: char) -> bool {
    matches!(ch,
      '\u{000A}'..='\u{000D}' | '\u{0009}' | '\u{0020}' | '\u{0085}' | '\u{00A0}' | '\u{1680}' | '\u{180E}'
      | '\u{2000}'..='\u{200B}' | '\u{2028}' | '\u{2029}' | '\u{202F}' | '\u{205F}' | '\u{3000}' | '\u{FEFF}')
  }
  match value {
    Value::String(s) => Value::String(s.trim_matches(is_feel_white_space).into()),
    Value::Null(_) => value_null!(),
    other => invalid_argument_type!("trim", "string", other),
  }
}

/// Returns new list containing concatenated list with duplicates removed.
pub fn union(lists: &[Value]) -> Value {
  let mut seen = Seen::default();
  let mut result = vec![];
  for list in lists {
    if let Value::List(items) = list {
      for item in items {
        if seen.insert(item) {
          result.push(item.clone())
        }
      }
    } else {
      return invalid_argument_type!("union", "list", list);
    }
  }
  Value::List(result.into())
}

/// Returns upper-cased string.
pub fn upper_case(input_string_value: &Value) -> Value {
  match input_string_value {
    Value::List(items) => {
      if items.len() == 1
        && let Value::String(input_string) = &items[0]
      {
        return Value::String(input_string.to_uppercase().into());
      }
    }
    Value::String(input_string) => return Value::String(input_string.to_uppercase().into()),
    _ => {}
  }
  invalid_argument_type!("upper case", "string", input_string_value)
}

/// Returns the ISO week number of the year.
pub fn week_of_year(value: &Value) -> Value {
  fn iso_week_of_year(opt_week_of_year: Option<WeekOfYear>) -> Value {
    if let Some(week_of_year) = opt_week_of_year {
      value_number!(week_of_year)
    } else {
      value_null!("[week of year] no week of year")
    }
  }
  match value {
    Value::Date(date) => iso_week_of_year(date.week_of_year()),
    Value::DateTime(date_time) => iso_week_of_year(date_time.week_of_year()),
    _ => invalid_argument_type!("week of year", "date, date and time", value),
  }
}

/// Returns years and months duration between `from` and `to`.
pub fn years_and_months_duration(from_value: &Value, to_value: &Value) -> Value {
  fn between(from: &FeelDate, to: &FeelDate) -> Value {
    (to - from).map_or_else(|| value_null!("years and months duration: result is out of range"), Value::YearsAndMonthsDuration)
  }
  if let Value::Date(from) = from_value {
    if let Value::DateTime(to) = to_value {
      return between(from, &to.date());
    }
    if let Value::Date(to) = to_value {
      return between(from, to);
    }
    return invalid_argument_type!("years and months duration", "date, date and time", to_value);
  }
  if let Value::DateTime(from) = from_value {
    if let Value::DateTime(to) = to_value {
      return between(&from.date(), &to.date());
    }
    if let Value::Date(to) = to_value {
      return between(&from.date(), to);
    }
    return invalid_argument_type!("years and months duration", "date, date and time", to_value);
  }
  invalid_argument_type!("years and months duration", "date, date and time", from_value)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_gregorian_month_of_year() {
    assert_eq!("null([month of year] no month)", gregorian_month(None).to_string());
  }
}
