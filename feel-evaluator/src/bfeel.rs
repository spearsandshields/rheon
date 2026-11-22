//! # B-FEEL (DMN 1.6, chapter 11)
//!
//! B-FEEL is a dialect of FEEL with the same grammar and friendlier semantics: null only means
//! missing data, never an error. Where FEEL returns null for an error, B-FEEL returns a value:
//!
//! - boolean operators and built-in functions return `false` (`!=` returns `true`), §11.2;
//! - built-in functions return the default of their result type: `0`, `""`, the epoch date, time
//!   or date and time, a zero duration, `[]`, or the empty range `(0..0)`, §11.3–§11.8;
//! - `mean`, `median`, `product`, `stddev`, `sum` and `mode` ignore non-numeric items, §11.3, §11.7;
//! - addition, subtraction, multiplication, division and exponentiation convert mismatched
//!   operands with the B-FEEL `string`, `number` and `duration` functions, §11.9–§11.11.
//!
//! The evaluator builder applies these rules when it builds an expression in [Dialect::BFeel].

use crate::bifs;
use crate::kernels::Arithmetic;
use rheon_feel::bif::Bif;
use rheon_feel::values::Value;
use rheon_feel::{FeelNumber, diagnostics};
use rheon_feel_temporal::{FeelDate, FeelDateTime, FeelOffset, FeelTime, FeelYearsAndMonthsDuration, FeelZone};

/// The expression language of an expression: FEEL (the default) or B-FEEL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dialect {
  #[default]
  Feel,
  BFeel,
}

impl Dialect {
  /// The dialect selected by an `expressionLanguage` URI: B-FEEL for the DMN 1.6 B-FEEL URI (and
  /// the version-less form of DMN 1.7 beta), FEEL for anything else.
  pub fn from_expression_language(uri: &str) -> Self {
    match uri.trim().trim_end_matches('/') {
      "https://www.omg.org/spec/DMN/20240513/B-FEEL" | "https://www.omg.org/spec/DMN/B-FEEL" => Dialect::BFeel,
      _ => Dialect::Feel,
    }
  }
}

/// The B-FEEL result of a built-in function whose FEEL result is null (§11.2–§11.8), or `None`
/// when B-FEEL defines no default for it (functions returning any type, contexts).
pub fn bif_default(bif: &Bif) -> Option<Value> {
  use Bif::*;
  Some(match bif {
    After | All | Any | Before | Coincides | Contains | During | EndsWith | Even | FinishedBy | Finishes | Includes | Is | ListContains | Matches | Meets | MetBy | Not | Odd
    | Overlaps | OverlapsAfter | OverlapsBefore | StartedBy | Starts | StartsWith => Value::Boolean(false),
    Abs | Ceiling | Count | DayOfYear | Decimal | Exp | Floor | Log | Mean | Median | Modulo | Number | Product | RoundDown | RoundHalfDown | RoundHalfUp | RoundUp | Sqrt
    | Stddev | StringLength | Sum | WeekOfYear => zero(),
    DayOfWeek | LoweCase | MonthOfYear | Replace | String | StringJoin | Substring | SubstringAfter | SubstringBefore | Trim | UpperCase => Value::String(Default::default()),
    Date | Today => epoch_date(),
    DateAndTime | Now => epoch_date_time(),
    Time => epoch_time(),
    Duration | YearsAndMonthsDuration => zero_ym_duration(),
    DaysAndTimeDuration => Value::DaysAndTimeDuration(rheon_feel_temporal::FeelDaysAndTimeDuration::default()),
    Append | Concatenate | DistinctValues | Flatten | GetEntries | IndexOf | InsertAfter | InsertBefore | Intersection | ListReplace | Mode | Remove | Reverse | Sort | Split
    | Sublist | Union => Value::List(vec![].into()),
    Range => empty_range(),
    Context | ContextMerge | ContextPut | GetValue | Max | Min => return None,
  })
}

/// Whether B-FEEL's `bif` ignores the items that are no numbers (11.3, 11.7).
fn takes_numbers_only(bif: &Bif) -> bool {
  matches!(bif, Bif::Mean | Bif::Median | Bif::Product | Bif::Stddev | Bif::Sum | Bif::Mode)
}

/// Arguments of a positional built-in call as B-FEEL passes them: the list-number functions drop
/// non-numeric items (from a single list argument, or from the arguments themselves).
pub fn positional_arguments(bif: &Bif, args: Vec<Value>) -> Vec<Value> {
  if !takes_numbers_only(bif) {
    return args;
  }
  let numbers = |items: &[Value]| items.iter().filter(|v| matches!(v, Value::Number(_))).cloned().collect::<Vec<Value>>();
  match args.as_slice() {
    [Value::List(items)] => vec![Value::List(numbers(items).into())],
    _ => vec![Value::List(numbers(&args).into())],
  }
}

/// Evaluates a positional built-in call in B-FEEL.
pub fn evaluate_positional_bif(bif: Bif, args: Vec<Value>) -> Value {
  let args = positional_arguments(&bif, args);
  defaulted(|| bifs::positional::evaluate_bif(bif.clone(), &args), |null| default_if_null(&bif, null))
}

/// Evaluates a named built-in call in B-FEEL.
pub fn evaluate_named_bif(bif: Bif, args: &Value) -> Value {
  let filtered = named_arguments(&bif, args);
  let args = filtered.as_ref().unwrap_or(args);
  defaulted(|| bifs::named::evaluate_bif(bif.clone(), args), |null| default_if_null(&bif, null))
}

/// The arguments of a named built-in call as B-FEEL passes them: the list-number functions drop
/// the items of their `list` that are no numbers, as by position (11.3, 11.7: `sum(list: [1,
/// null, 3])` is 4); `None` when nothing is dropped.
fn named_arguments(bif: &Bif, args: &Value) -> Option<Value> {
  if !takes_numbers_only(bif) {
    return None;
  }
  let Value::NamedParameters(given) = args else {
    return None;
  };
  let list = rheon_feel::Name::from("list");
  let (Value::List(items), position) = given.get(&list)? else {
    return None;
  };
  if items.iter().all(|item| matches!(item, Value::Number(_))) {
    return None;
  }
  let numbers: Vec<Value> = items.iter().filter(|item| matches!(item, Value::Number(_))).cloned().collect();
  let mut given = given.clone();
  given.insert(list, (Value::List(numbers.into()), *position));
  Some(Value::NamedParameters(given))
}

/// The value of the FEEL operation `feel`, or, when it is null, what `b_feel` makes of that null.
/// When B-FEEL gives a value there, the errors that the operation recorded become warnings: B-FEEL
/// defines that result as normal ("null only means missing data, never an error", DMN 1.7 Beta 1,
/// 11.1), so a strict evaluation accepts it, and the messages stay to be read.
pub(crate) fn defaulted(feel: impl FnOnce() -> Value, b_feel: impl FnOnce(Value) -> Value) -> Value {
  let mark = diagnostics::tentative_mark();
  let value = feel();
  if !value.is_null() {
    drop(mark);
    return value;
  }
  let value = b_feel(value);
  if value.is_null() {
    drop(mark);
  } else {
    mark.demote();
  }
  value
}

pub(crate) fn default_if_null(bif: &Bif, result: Value) -> Value {
  if result.is_null()
    && let Some(default) = bif_default(bif)
  {
    return default;
  }
  result
}

/// A boolean operator's B-FEEL result (§11.2): null is `false`, or `true` for `!=`.
pub fn boolean_result(value: Value, not_equal: bool) -> Value {
  match value {
    Value::Null(_) => Value::Boolean(not_equal),
    other => other,
  }
}

/// The B-FEEL `string` function: `string(null)` is `""`.
pub fn to_string(value: &Value) -> Value {
  default_if_null(&Bif::String, bifs::core::string(value))
}

/// The B-FEEL `number` function: a number unchanged, a numeric string converted, anything else 0.
pub fn to_number(value: &Value) -> Value {
  match value {
    Value::Number(_) => value.clone(),
    Value::String(_) => default_if_null(&Bif::Number, bifs::core::number_single(value)),
    _ => zero(),
  }
}

/// The B-FEEL `duration` function: a duration unchanged, a duration string converted, anything
/// else the zero years and months duration.
pub fn to_duration(value: &Value) -> Value {
  match value {
    Value::YearsAndMonthsDuration(_) | Value::DaysAndTimeDuration(_) => value.clone(),
    Value::String(_) => default_if_null(&Bif::Duration, bifs::core::duration(value)),
    _ => zero_ym_duration(),
  }
}

/// B-FEEL arithmetic (§11.9–§11.11): the FEEL result when it is not null; otherwise the first
/// matching rule, in the spec's order of precedence, converts the mismatched operand and FEEL
/// semantics apply to the converted values ([Arithmetic::apply] each time).
pub(crate) fn arithmetic(operator: Arithmetic, lhs: Value, rhs: Value) -> Value {
  let (l, r) = (lhs.clone(), rhs.clone());
  defaulted(move || operator.apply(l, r), move |feel| converted_arithmetic(operator, lhs, rhs, feel))
}

/// [arithmetic] once the FEEL result `feel` is null: the conversion of the first matching rule.
fn converted_arithmetic(operator: Arithmetic, lhs: Value, rhs: Value, feel: Value) -> Value {
  let either = |f: fn(&Value) -> bool| f(&lhs) || f(&rhs);
  // convert the operand that does not satisfy `is`, keep the other one
  let convert = |is: fn(&Value) -> bool, to: fn(&Value) -> Value| {
    let l = if is(&lhs) { lhs.clone() } else { to(&lhs) };
    let r = if is(&rhs) { rhs.clone() } else { to(&rhs) };
    operator.apply(l, r)
  };
  match operator {
    Arithmetic::Exponentiation => operator.apply(to_number(&lhs), to_number(&rhs)),
    Arithmetic::Addition | Arithmetic::Subtraction => {
      if either(is_string) {
        if operator == Arithmetic::Subtraction {
          return Value::String(Default::default());
        }
        return convert(is_string, to_string);
      }
      if either(is_number) {
        return convert(is_number, to_number);
      }
      for is in [is_date_time as fn(&Value) -> bool, is_date, is_time, is_ym_duration, is_dt_duration] {
        if either(is) {
          return convert(is, to_duration);
        }
      }
      feel
    }
    Arithmetic::Multiplication | Arithmetic::Division => {
      for is in [is_number as fn(&Value) -> bool, is_ym_duration, is_dt_duration] {
        if either(is) {
          return convert(is, to_number);
        }
      }
      feel
    }
    // negation is no B-FEEL conversion: its FEEL result stands
    Arithmetic::Negation => feel,
  }
}

fn is_string(v: &Value) -> bool {
  matches!(v, Value::String(_))
}
fn is_number(v: &Value) -> bool {
  matches!(v, Value::Number(_))
}
fn is_date_time(v: &Value) -> bool {
  matches!(v, Value::DateTime(_))
}
fn is_date(v: &Value) -> bool {
  matches!(v, Value::Date(_))
}
fn is_time(v: &Value) -> bool {
  matches!(v, Value::Time(_))
}
fn is_ym_duration(v: &Value) -> bool {
  matches!(v, Value::YearsAndMonthsDuration(_))
}
fn is_dt_duration(v: &Value) -> bool {
  matches!(v, Value::DaysAndTimeDuration(_))
}

fn zero() -> Value {
  Value::Number(FeelNumber::zero())
}
fn zero_ym_duration() -> Value {
  Value::YearsAndMonthsDuration(FeelYearsAndMonthsDuration::default())
}
fn epoch_date() -> Value {
  FeelDate::new(1970, 1, 1).map(Value::Date).unwrap_or(Value::Null(None))
}
fn epoch_time() -> Value {
  Value::Time(FeelTime::midnight(FeelZone::Offset(FeelOffset::ZERO)))
}
fn epoch_date_time() -> Value {
  match (FeelDate::new(1970, 1, 1), FeelTime::midnight(FeelZone::Offset(FeelOffset::ZERO))) {
    (Some(date), time) => Value::DateTime(FeelDateTime::new(date, time)),
    _ => Value::Null(None),
  }
}
fn empty_range() -> Value {
  bifs::core::range(&Value::String("(0..0)".into()))
}
