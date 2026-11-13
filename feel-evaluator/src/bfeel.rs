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
use rheon_feel::bif::Bif;
use rheon_feel::context::FeelContext;
use rheon_feel::values::Value;
use rheon_feel::{Evaluator, FeelNumber, FeelScope, Name};
use rheon_feel_parser::AstNode;
use rheon_feel_temporal::{FeelDate, FeelDateTime, FeelTime, FeelYearsAndMonthsDuration};
use std::sync::LazyLock;

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
    DayOfWeek | LoweCase | MonthOfYear | Replace | String | StringJoin | Substring | SubstringAfter | SubstringBefore | Trim | UpperCase => {
      Value::String(std::string::String::new())
    }
    Date | Today => epoch_date(),
    DateAndTime | Now => epoch_date_time(),
    Time => epoch_time(),
    Duration | YearsAndMonthsDuration => zero_ym_duration(),
    Append | Concatenate | DistinctValues | Flatten | GetEntries | IndexOf | InsertAfter | InsertBefore | Intersection | ListReplace | Mode | Remove | Reverse | Sort | Split
    | Sublist | Union => Value::List(vec![]),
    Range => empty_range(),
    Context | ContextMerge | ContextPut | GetValue | Max | Min => return None,
  })
}

/// Arguments of a positional built-in call as B-FEEL passes them: the list-number functions drop
/// non-numeric items (from a single list argument, or from the arguments themselves).
pub fn positional_arguments(bif: &Bif, args: Vec<Value>) -> Vec<Value> {
  if !matches!(bif, Bif::Mean | Bif::Median | Bif::Product | Bif::Stddev | Bif::Sum | Bif::Mode) {
    return args;
  }
  let numbers = |items: &[Value]| items.iter().filter(|v| matches!(v, Value::Number(_))).cloned().collect::<Vec<Value>>();
  match args.as_slice() {
    [Value::List(items)] => vec![Value::List(numbers(items))],
    _ => vec![Value::List(numbers(&args))],
  }
}

/// Evaluates a positional built-in call in B-FEEL.
pub fn evaluate_positional_bif(bif: Bif, args: Vec<Value>) -> Value {
  let args = positional_arguments(&bif, args);
  let result = bifs::positional::evaluate_bif(bif.clone(), &args);
  default_if_null(&bif, result)
}

/// Evaluates a named built-in call in B-FEEL.
pub fn evaluate_named_bif(bif: Bif, args: &Value) -> Value {
  let result = bifs::named::evaluate_bif(bif.clone(), args);
  default_if_null(&bif, result)
}

fn default_if_null(bif: &Bif, result: Value) -> Value {
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

/// Arithmetic operators of B-FEEL.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Operator {
  Add,
  Sub,
  Mul,
  Div,
  Exp,
}

static LHS: LazyLock<Name> = LazyLock::new(|| Name::from("⟦b-feel lhs⟧"));
static RHS: LazyLock<Name> = LazyLock::new(|| Name::from("⟦b-feel rhs⟧"));
static OPERAND_NODES: LazyLock<[AstNode; 5]> = LazyLock::new(|| {
  let l = || Box::new(AstNode::Name(LHS.clone()));
  let r = || Box::new(AstNode::Name(RHS.clone()));
  [
    AstNode::Add(l(), r()),
    AstNode::Sub(l(), r()),
    AstNode::Mul(l(), r()),
    AstNode::Div(l(), r()),
    AstNode::Exp(l(), r()),
  ]
});

/// The FEEL evaluator of `⟦lhs⟧ <op> ⟦rhs⟧`, used on already evaluated operands.
pub fn feel_operator_evaluator(operator: Operator) -> Evaluator {
  let index = match operator {
    Operator::Add => 0,
    Operator::Sub => 1,
    Operator::Mul => 2,
    Operator::Div => 3,
    Operator::Exp => 4,
  };
  crate::builders::build_evaluator(&OPERAND_NODES[index])
}

/// Applies a FEEL arithmetic evaluator built by [feel_operator_evaluator] to two values.
fn apply(feel_operator: &Evaluator, scope: &FeelScope, lhs: &Value, rhs: &Value) -> Value {
  let mut operands = FeelContext::default();
  operands.set_entry(&LHS, lhs.clone());
  operands.set_entry(&RHS, rhs.clone());
  let depth = scope.depth();
  scope.push(operands);
  let value = feel_operator(scope);
  scope.truncate(depth);
  value
}

/// B-FEEL arithmetic (§11.9–§11.11): the FEEL result when it is not null; otherwise the first
/// matching rule, in the spec's order of precedence, converts the mismatched operand and FEEL
/// semantics apply to the converted values.
pub fn arithmetic(operator: Operator, feel_operator: &Evaluator, scope: &FeelScope, lhs: Value, rhs: Value) -> Value {
  let feel = apply(feel_operator, scope, &lhs, &rhs);
  if !feel.is_null() {
    return feel;
  }
  let either = |f: fn(&Value) -> bool| f(&lhs) || f(&rhs);
  // convert the operand that does not satisfy `is`, keep the other one
  let convert = |is: fn(&Value) -> bool, to: fn(&Value) -> Value| {
    let l = if is(&lhs) { lhs.clone() } else { to(&lhs) };
    let r = if is(&rhs) { rhs.clone() } else { to(&rhs) };
    apply(feel_operator, scope, &l, &r)
  };
  match operator {
    Operator::Exp => apply(feel_operator, scope, &to_number(&lhs), &to_number(&rhs)),
    Operator::Add | Operator::Sub => {
      if either(is_string) {
        if operator == Operator::Sub {
          return Value::String(String::new());
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
    Operator::Mul | Operator::Div => {
      for is in [is_number as fn(&Value) -> bool, is_ym_duration, is_dt_duration] {
        if either(is) {
          return convert(is, to_number);
        }
      }
      feel
    }
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
  Value::YearsAndMonthsDuration(FeelYearsAndMonthsDuration::from_m(0))
}
fn epoch_date() -> Value {
  FeelDate::new(1970, 1, 1).map(Value::Date).unwrap_or(Value::Null(None))
}
fn epoch_time() -> Value {
  Value::Time(FeelTime::offset(0, 0, 0, 0, 0))
}
fn epoch_date_time() -> Value {
  match (FeelDate::new(1970, 1, 1), FeelTime::offset(0, 0, 0, 0, 0)) {
    (Some(date), time) => Value::DateTime(FeelDateTime::new(date, time)),
    _ => Value::Null(None),
  }
}
fn empty_range() -> Value {
  bifs::core::range(&Value::String("(0..0)".to_string()))
}
