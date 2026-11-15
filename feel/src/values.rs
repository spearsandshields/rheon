//! # FEEL values

use crate::FunctionBody;
use crate::bif::Bif;
use crate::context::FeelContext;
use crate::diagnostics;
use crate::errors::*;
use crate::function::FunctionDefinition;
use crate::message::{MessageArg, PREVIEW_CHARS, Size, arg, compose};
use crate::names::Name;
use crate::ranges::IntervalType;
use crate::strings::{ToFeelString, write_separated};
use crate::types::{FeelType, write_context_type, write_function_type, write_list_type};
use dsntk_common::{Jsonify, Result};
use rheon_feel_number::FeelNumber;
use rheon_feel_temporal::{FeelDate, FeelDateTime, FeelDaysAndTimeDuration, FeelTime, FeelYearsAndMonthsDuration, FeelZone};
use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;

mod list;
mod string;
mod type_relations;

pub use list::{FeelList, ListItems};
pub use string::FeelString;

/// Creates `Value::Null` with optional tracing message: the reason of an error null, or with
/// `note:` the note on a null that the specification defines, which is not counted as an error
/// ([Severity](crate::diagnostics::Severity)).
///
/// The message is built by [compose](crate::message::compose), so it holds at most 1,000
/// characters whatever its arguments; values in it are quoted with [arg](crate::message::arg).
/// It is built out of line, in [null_with_reason](crate::values::null_with_reason), so the code
/// that returns the null keeps the small stack frame of its other paths.
///
/// # Examples
///
/// ```
/// use crate::rheon_feel::{value_null, values::Value};
///
/// let v = value_null!();
/// assert_eq!("null", v.to_string());
///
/// let v = value_null!("missing input parameter");
/// assert_eq!("null(missing input parameter)", v.to_string());
///
/// let v = value_null!("integer out of range {}..{}", 1, 100);
/// assert_eq!("null(integer out of range 1..100)", v.to_string());
///
/// // the note on a null that the specification defines, which is no error
/// let v = value_null!(note: "no entry {} in context", "b");
/// assert_eq!("null(no entry b in context)", v.to_string());
/// ```
#[macro_export]
macro_rules! value_null {
  (note: $format:literal, $($arguments:tt)*) => {
    $crate::value_null!(@severity Note, $format, $($arguments)*)
  };
  (note: $argument:expr) => {
    $crate::value_null!(@severity Note, "{}", $argument)
  };
  (@severity $severity:ident, $($format:tt)*) => {
    $crate::values::null_with_reason($crate::diagnostics::Severity::$severity, || $crate::message::compose(format_args!($($format)*)))
  };
  ($module:expr, $function:expr, $format:literal, $($arguments:tt)*) => {
    $crate::value_null!(@severity Error, "[{}::{}] {}", $module, $function, format_args!($format, $($arguments)*))
  };
  ($format:literal, $($arguments:tt)*) => {
    $crate::value_null!(@severity Error, $format, $($arguments)*)
  };
  ($argument:expr) => {
    $crate::value_null!(@severity Error, "{}", $argument)
  };
  () => {
    Value::Null(None)
  };
}

/// The null with the reason that `reason` builds, recorded in the running diagnostics collection
/// when it is an error (the body of `value_null!`). It is cold and never inlined: building a
/// message takes stack space for its arguments and formatting, and a null with a reason is the
/// rare path of the code that returns it, often an evaluator that recursion nests deeply.
#[cold]
#[inline(never)]
pub fn null_with_reason(severity: crate::diagnostics::Severity, reason: impl FnOnce() -> String) -> Value {
  let message = reason();
  crate::diagnostics::record(severity, &message);
  Value::Null(Some(message))
}

#[macro_export]
macro_rules! value_number {
  ($n:expr) => {{ Value::Number($n.into()) }};
  ($n:expr, $s:expr) => {
    Value::Number(FeelNumber::new($n, $s))
  };
}

#[macro_export]
macro_rules! value_string {
  ($s:expr) => {{ Value::String($s.into()) }};
}

/// Utility constant for value `true `of type `Boolean`.
pub const VALUE_TRUE: Value = Value::Boolean(true);

/// Utility constant for value `false` of type `Boolean`.
pub const VALUE_FALSE: Value = Value::Boolean(false);

/// `FEEL` value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
  /// Value representing `FEEL` boolean type.
  Boolean(bool),

  /// Value for storing built-in function definition.
  BuiltInFunction(Bif),

  /// Value representing a context.
  Context(FeelContext),

  /// Value representing a context entry.
  ContextEntry(Name, Box<Value>),

  /// Value representing a key of the context entry.
  ContextEntryKey(Name),

  /// Value representing the context type.
  ContextType(FeelType),

  /// Value representing a context entry in context type definition.
  ContextTypeEntry(Name, FeelType),

  /// Value representing a key of the context entry in context type definition.
  ContextTypeEntryKey(Name),

  /// Value for storing dates as [FeelDate].
  Date(FeelDate),

  /// Value for storing date and time as [FeelDateTime].
  DateTime(FeelDateTime),

  /// Value for days and time durations.
  DaysAndTimeDuration(FeelDaysAndTimeDuration),

  /// Value representing a collection of comma-separated list of expressions.
  ExpressionList(Values),

  /// Value representing a mapping to externally defined `Java` function.
  ExternalJavaFunction(
    /// Class name.
    String,
    /// Method signature.
    String,
  ),

  /// Value representing a mapping to externally defined `PMML` function.
  ExternalPmmlFunction(
    /// Document.
    String,
    /// Model name.
    String,
  ),

  /// Value representing the `FEEL` type of the value.
  FeelType(FeelType),

  /// Value representing function's formal parameter with name and type.
  FormalParameter(Name, FeelType),

  /// List of formal parameters.
  FormalParameters(Vec<(Name, FeelType)>),

  /// Definition of the function body.
  FunctionBody(
    /// Body of the function.
    FunctionBody,
    /// Flag indicating if the function's body is an external function, `true` == external.
    bool,
  ),

  /// Value representing the function definition.
  /// This value holds the list of function's formal parameters, the function's body, closure for lambdas and expected result type.
  FunctionDefinition(Arc<FunctionDefinition>),

  /// Value representing interval end.
  IntervalEnd(
    /// Value at the end of the interval.
    Box<Value>,
    /// Type of the interval end: opened or closed.
    IntervalType,
  ),

  /// Value representing interval start.
  IntervalStart(
    /// Value at the start of the interval.
    Box<Value>,
    /// Type of the interval start: opened or closed.
    IntervalType,
  ),

  /// Value representing `FEEL` `irrelevant` value.
  Irrelevant,

  /// Value representing a list of values.
  List(FeelList),

  /// Value representing a collection of named parameters.
  NamedParameters(BTreeMap<Name, (Value, usize)>),

  /// Value representing a collection of values representing a negated comma-separated list of expressions.
  NegatedCommaList(Values),

  /// Null value with optional tracing message.
  Null(Option<String>),

  /// Value representing `FEEL` number type.
  Number(FeelNumber),

  /// Name of the parameter.
  ParameterName(Name),

  /// Value representing a list of function's parameter types.
  ParameterTypes(Vec<Value>),

  /// Value representing a segment of a qualified name.
  QualifiedNameSegment(Name),

  /// Value representing a `range`.
  Range(
    /// Value at the start of the range.
    Box<Value>,
    /// Type of the interval at the start of the range: closed or opened.
    IntervalType,
    /// Value at the end of the range.
    Box<Value>,
    /// Type of the interval at the end of the range: closed or opened.
    IntervalType,
  ),

  /// Value representing `FEEL` string type.
  String(FeelString),

  /// Value for storing time as [FeelTime].
  Time(FeelTime),

  /// Value representing unary `>`.
  UnaryGreater(
    /// Value on the right side of the unary `>` operator.
    Box<Value>,
  ),

  /// Value representing unary `>=`.
  UnaryGreaterOrEqual(
    /// Value on the right side of the unary `>=` operator.
    Box<Value>,
  ),

  /// Value representing unary `<`.
  UnaryLess(
    /// Value on the right side of the unary `<` operator.
    Box<Value>,
  ),

  /// Value representing unary `<=`.
  UnaryLessOrEqual(
    /// Value on the right side of the unary `<=` operator.
    Box<Value>,
  ),

  /// Value representing unary `=`.
  UnaryEqual(
    /// Value on the right side of the unary `=` operator.
    Box<Value>,
  ),

  /// Value representing unary `!=`.
  UnaryNotEqual(
    /// Value on the right side of the unary `!=` operator.
    Box<Value>,
  ),

  /// Value for storing years and months duration.
  YearsAndMonthsDuration(FeelYearsAndMonthsDuration),
}

// Values are moved and copied everywhere in the evaluator, so their size is part of its speed.
const _: () = assert!(std::mem::size_of::<Value>() <= 64);

impl Display for Value {
  /// Implements [Display] for a [Value].
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Value::Boolean(value) => write!(f, "{value}"),
      Value::BuiltInFunction(_) => write!(f, "BuiltInFunction"),
      Value::Context(context) => write!(f, "{context}"),
      Value::ContextEntry(_, _) => write!(f, "ContextEntry"),
      Value::ContextEntryKey(name) => write!(f, "{name}"),
      Value::ContextType(_) => write!(f, "ContextType"),
      Value::ContextTypeEntry(name, feel_type) => write!(f, "{name}: {feel_type}"),
      Value::ContextTypeEntryKey(name) => write!(f, "{name}"),
      Value::Date(date) => write!(f, "{date}"),
      Value::DateTime(date_time) => write!(f, "{date_time}"),
      Value::DaysAndTimeDuration(dt_duration) => write!(f, "{dt_duration}"),
      Value::ExpressionList(items) => write_values(f, items),
      Value::ExternalJavaFunction(class_name, method_signature) => write!(f, "ExternalJavaFunction({class_name}, {method_signature})"),
      Value::ExternalPmmlFunction(iri, model_name) => write!(f, "ExternalPmmlFunction({iri}, {model_name})"),
      Value::FeelType(feel_type) => write!(f, "type({feel_type})"),
      Value::FormalParameter(_, _) => write!(f, "FormalParameter"),
      Value::FormalParameters(_) => write!(f, "FormalParameters"),
      Value::FunctionBody(_, external) => write!(f, "FunctionBody{}", if *external { " (external)" } else { "" }),
      Value::FunctionDefinition(function) => {
        let FunctionDefinition {
          external, closure, closure_ctx, ..
        } = function.as_ref();
        let (parameters, result_type) = (function.parameters(), function.result_type());
        write!(f, "FunctionDefinition({parameters:?},_,{external},{closure},{closure_ctx},{result_type})")
      }
      Value::IntervalEnd(_, _) => write!(f, "IntervalEnd"),
      Value::IntervalStart(_, _) => write!(f, "IntervalStart"),
      Value::Irrelevant => write!(f, "Irrelevant"),
      Value::List(items) => write_values(f, items),
      Value::NamedParameters(_) => write!(f, "NamedParameters"),
      Value::NegatedCommaList(_) => write!(f, "NegatedCommaList"),
      Value::Number(value) => write!(f, "{value}"),
      Value::Null(None) => f.write_str("null"),
      Value::Null(Some(reason)) => write!(f, "null({reason})"),
      Value::ParameterName(_) => write!(f, "ParameterName"),
      Value::ParameterTypes(_) => write!(f, "ParameterTypes"),
      Value::QualifiedNameSegment(_) => write!(f, "QualifiedNameSegment"),
      Value::Range(start, start_type, end, end_type) => {
        f.write_str(if start_type.closed() { "[" } else { "(" })?;
        if !start_type.undefined() {
          write!(f, "{start}")?;
        }
        f.write_str("..")?;
        if !end_type.undefined() {
          write!(f, "{end}")?;
        }
        f.write_str(if end_type.closed() { "]" } else { ")" })
      }
      Value::String(s) => write!(f, "\"{s}\""),
      Value::Time(time) => write!(f, "{time}"),
      Value::UnaryGreater(value) => write!(f, "UnaryGreater({value})"),
      Value::UnaryGreaterOrEqual(value) => write!(f, "UnaryGreaterOrEqual({value})"),
      Value::UnaryLess(value) => write!(f, "UnaryLess({value})"),
      Value::UnaryLessOrEqual(value) => write!(f, "UnaryLessOrEqual({value})"),
      Value::UnaryEqual(value) => write!(f, "UnaryEqual({value})"),
      Value::UnaryNotEqual(value) => write!(f, "UnaryNotEqual({value})"),
      Value::YearsAndMonthsDuration(ym_duration) => write!(f, "{ym_duration}"),
    }
  }
}

impl ToFeelString for Value {
  /// Converts [Value] into `FEEL` string.
  fn to_feel_string(&self) -> String {
    match self {
      Value::Context(context) => context.to_feel_string(),
      Value::List(items) => values_to_feel_string(items),
      Value::String(value) => format!("\"{}\"", value.replace('"', "\\\"")),
      other => other.to_string(),
    }
  }
}

impl Jsonify for Value {
  /// Converts a [Value] into `JSON`.
  fn jsonify(&self) -> String {
    match self {
      Value::Boolean(value) => format!("{value}"),
      Value::Number(value) => value.jsonify(),
      Value::String(s) => format!(r#""{s}""#),
      Value::Date(date) => format!(r#""{}""#, date),
      Value::Time(time) => format!(r#""{}""#, time),
      Value::DateTime(date_time) => format!(r#""{}""#, date_time),
      Value::DaysAndTimeDuration(dt_duration) => format!(r#""{}""#, dt_duration),
      Value::YearsAndMonthsDuration(ym_duration) => format!(r#""{}""#, ym_duration),
      Value::ExpressionList(items) => values_to_jsonify(items),
      Value::Context(ctx) => ctx.jsonify(),
      Value::ContextEntryKey(name) => name.to_string(),
      Value::List(items) => values_to_jsonify(items),
      range @ Value::Range(..) => format!(r#""{}""#, range),
      Value::Null(message) => {
        if let Some(details) = message {
          format!(r#""null({details})""#)
        } else {
          "null".to_string()
        }
      }
      _ => format!("jsonify trait not implemented for value: {self}"),
    }
  }
}

impl Value {
  /// Returns `true` when the value is of type [Value::Null].
  pub fn is_null(&self) -> bool {
    matches!(self, Value::Null(_))
  }

  /// Returns `true` when the value is of type [Value::Boolean] and is equal to `true`.
  pub fn is_true(&self) -> bool {
    matches!(self, Value::Boolean(true))
  }

  /// Returns `true` when the value is of type [Value::Boolean] and is equal to `false`.
  pub fn is_false(&self) -> bool {
    matches!(self, Value::Boolean(false))
  }

  /// Returns `true` when the value is of type [Value::Number].
  pub fn is_number(&self) -> bool {
    matches!(self, Value::Number(_))
  }

  /// Returns `true` when the value is of type [Value::String].
  pub fn is_string(&self) -> bool {
    matches!(self, Value::String(_))
  }

  /// Returns `true` when the value is of type [Value::List].
  pub fn is_list(&self) -> bool {
    matches!(self, Value::List(_))
  }

  /// The nesting depth of this value: 0 for a scalar; a list or a context is one level deeper
  /// than the deepest value it holds, so an empty one is 1 level deep (a list keeps its depth, and
  /// a context remembers it from its first read until its next write); a function is one level
  /// deeper than its closure context, so 2 when it captures nothing; a range, an interval end, a
  /// unary test and the other values that hold values are one level deeper than what they hold.
  #[inline]
  pub(crate) fn nesting_depth(&self) -> u32 {
    // lists, contexts and the values that hold none are read where they are, as they are the most common
    match self {
      Value::List(list) => list.nesting_depth(),
      Value::Context(context) => context.nesting_depth(),
      Value::Null(_) | Value::Number(_) | Value::Boolean(_) | Value::String(_) => 0,
      other => other.operand_depth(),
    }
  }

  /// The nesting depth of the values other than lists, contexts and the most common scalars.
  #[inline(never)]
  fn operand_depth(&self) -> u32 {
    fn deepest<'a>(values: impl Iterator<Item = &'a Value>) -> u32 {
      values.map(Value::nesting_depth).max().unwrap_or(0).saturating_add(1)
    }
    match self {
      Value::List(list) => list.nesting_depth(),
      Value::Context(context) => context.nesting_depth(),
      Value::FunctionDefinition(function) => function.closure_ctx.nesting_depth().saturating_add(1),
      Value::Range(start, _, end, _) => deepest([start.as_ref(), end.as_ref()].into_iter()),
      Value::ContextEntry(_, value)
      | Value::IntervalEnd(value, _)
      | Value::IntervalStart(value, _)
      | Value::UnaryGreater(value)
      | Value::UnaryGreaterOrEqual(value)
      | Value::UnaryLess(value)
      | Value::UnaryLessOrEqual(value)
      | Value::UnaryEqual(value)
      | Value::UnaryNotEqual(value) => value.nesting_depth().saturating_add(1),
      Value::ExpressionList(values) | Value::NegatedCommaList(values) | Value::ParameterTypes(values) => deepest(values.iter()),
      Value::NamedParameters(parameters) => deepest(parameters.values().map(|(value, _)| value)),
      Value::Boolean(_)
      | Value::BuiltInFunction(_)
      | Value::ContextEntryKey(_)
      | Value::ContextType(_)
      | Value::ContextTypeEntry(_, _)
      | Value::ContextTypeEntryKey(_)
      | Value::Date(_)
      | Value::DateTime(_)
      | Value::DaysAndTimeDuration(_)
      | Value::ExternalJavaFunction(_, _)
      | Value::ExternalPmmlFunction(_, _)
      | Value::FeelType(_)
      | Value::FormalParameter(_, _)
      | Value::FormalParameters(_)
      | Value::FunctionBody(_, _)
      | Value::Irrelevant
      | Value::Null(_)
      | Value::Number(_)
      | Value::ParameterName(_)
      | Value::QualifiedNameSegment(_)
      | Value::String(_)
      | Value::Time(_)
      | Value::YearsAndMonthsDuration(_) => 0,
    }
  }

  /// Returns `true` when the value is of type [Value::Range].
  pub fn is_range(&self) -> bool {
    matches!(self, Value::Range(_, _, _, _))
  }

  /// Returns `true` when the value is a valid range.
  pub fn is_valid_range(&self) -> bool {
    if let Value::Range(start, start_type, end, end_type) = self {
      if (start_type.undefined() && start_type.closed()) || (end_type.undefined() && end_type.closed()) {
        // Ranges with undefined interval values can not be closed.
        return false;
      }
      match start.borrow() {
        Value::String(start) => match end.borrow() {
          Value::String(end) => start <= end,
          Value::Null(_) => end_type.opened(),
          _ => false,
        },
        Value::Number(start) => match end.borrow() {
          Value::Number(end) => start <= end,
          Value::Null(_) => end_type.opened(),
          _ => false,
        },
        Value::Date(start) => match end.borrow() {
          Value::Date(end) => start <= end,
          Value::Null(_) => end_type.opened(),
          _ => false,
        },
        Value::DateTime(start) => match end.borrow() {
          Value::DateTime(end) => start <= end,
          Value::Null(_) => end_type.opened(),
          _ => false,
        },
        Value::Time(start) => match end.borrow() {
          Value::Time(end) => start <= end,
          Value::Null(_) => end_type.opened(),
          _ => false,
        },
        Value::YearsAndMonthsDuration(start) => match end.borrow() {
          Value::YearsAndMonthsDuration(end) => start <= end,
          Value::Null(_) => end_type.opened(),
          _ => false,
        },
        Value::DaysAndTimeDuration(start) => match end.borrow() {
          Value::DaysAndTimeDuration(end) => start <= end,
          Value::Null(_) => end_type.opened(),
          _ => false,
        },
        Value::Null(_) => match end.borrow() {
          Value::Null(_) => false,
          _ => start_type.opened(),
        },
        _ => false,
      }
    } else {
      false
    }
  }

  /// Returns the type of this [Value].
  pub fn type_of(&self) -> FeelType {
    // an unlimited budget is never used up
    self.type_of_within(&mut Unlimited).unwrap_or(FeelType::Any)
  }

  /// The type of this value, or `None` when it would hold more type nodes than `budget` has
  /// left: the body of [type_of](Self::type_of), which passes an unlimited budget.
  fn type_of_within(&self, budget: &mut impl Budget) -> Option<FeelType> {
    budget.take(1)?;
    Some(match self {
      Value::Boolean(_) => FeelType::Boolean,
      Value::BuiltInFunction(_) => FeelType::Any,
      Value::Context(context) => {
        let mut entries = BTreeMap::new();
        for (name, value) in context.iter() {
          entries.insert(name.clone(), value.type_of_within(budget)?);
        }
        FeelType::Context(entries)
      }
      Value::ContextEntry(_, _) => FeelType::Any,
      Value::ContextEntryKey(_) => FeelType::Any,
      Value::ContextType(feel_type) => budget.copy(feel_type)?,
      Value::ContextTypeEntry(_, feel_type) => budget.copy(feel_type)?,
      Value::ContextTypeEntryKey(_) => FeelType::Any,
      Value::Date(_) => FeelType::Date,
      Value::DateTime(_) => FeelType::DateTime,
      Value::DaysAndTimeDuration(_) => FeelType::DaysAndTimeDuration,
      Value::ExpressionList(_) => FeelType::Any,
      Value::ExternalJavaFunction(_, _) => FeelType::Any,
      Value::ExternalPmmlFunction(_, _) => FeelType::Any,
      Value::FeelType(feel_type) => budget.copy(feel_type)?,
      Value::FormalParameter(_, feel_type) => budget.copy(feel_type)?,
      Value::FormalParameters(_) => FeelType::Any,
      Value::FunctionBody(_, _) => FeelType::Any,
      Value::FunctionDefinition(function) => {
        let mut parameter_types = Vec::with_capacity(function.parameters().len());
        for (_, feel_type) in function.parameters() {
          parameter_types.push(budget.copy(feel_type)?);
        }
        FeelType::Function(parameter_types, Box::new(budget.copy(function.result_type())?))
      }
      Value::IntervalEnd(interval_end, _) => interval_end.type_of_within(budget)?,
      Value::IntervalStart(interval_start, _) => interval_start.type_of_within(budget)?,
      Value::Irrelevant => FeelType::Any,
      Value::List(values) => FeelType::List(Box::new(list_item_type(values, budget)?)),
      Value::NamedParameters(_) => FeelType::Any,
      Value::NegatedCommaList(_) => FeelType::Any,
      Value::Null(_) => FeelType::Null,
      Value::Number(_) => FeelType::Number,
      Value::ParameterName(_) => FeelType::Any,
      Value::ParameterTypes(_) => FeelType::Any,
      Value::QualifiedNameSegment(_) => FeelType::Any,
      Value::Range(range_start, _, range_end, _) => {
        let range_start_type = range_start.type_of_within(budget)?;
        let range_end_type = range_end.type_of_within(budget)?;
        if range_start_type == range_end_type {
          return Some(FeelType::Range(Box::new(range_start_type)));
        }
        FeelType::Range(Box::new(FeelType::Any))
      }
      Value::String(_) => FeelType::String,
      Value::Time(_) => FeelType::Time,
      Value::UnaryGreater(_) => FeelType::Boolean,
      Value::UnaryGreaterOrEqual(_) => FeelType::Boolean,
      Value::UnaryLess(_) => FeelType::Boolean,
      Value::UnaryLessOrEqual(_) => FeelType::Boolean,
      Value::UnaryEqual(_) => FeelType::Boolean,
      Value::UnaryNotEqual(_) => FeelType::Boolean,
      Value::YearsAndMonthsDuration(_) => FeelType::YearsAndMonthsDuration,
    })
  }

  /// Returns `self.type_of().is_conformant(target_type)` without building the type of a context,
  /// or `None` when a type it builds would hold more type nodes than `budget` has left.
  ///
  /// The type of a context is the context type of its entries' types: it conforms to `Any`, and
  /// to a context type when every entry of the target type has an entry here whose type conforms
  /// to the target entry's type (extra entries are allowed); to nothing else
  /// ([FeelType::is_conformant]). The type of `null` conforms to every type.
  fn type_is_conformant_within(&self, target_type: &FeelType, budget: &mut impl Budget) -> Option<bool> {
    match self {
      Value::Null(_) => Some(true),
      Value::Context(ctx) => match target_type {
        FeelType::Any => Some(true),
        FeelType::Context(target_entries) => {
          for (name, entry_type) in target_entries {
            match ctx.get_entry(name) {
              Some(value) if value.type_is_conformant_within(entry_type, budget)? => {}
              _ => return Some(false),
            }
          }
          Some(true)
        }
        _ => Some(false),
      },
      other => {
        // the type is dropped after the check, so its nodes are given back
        let mark = budget.mark();
        let conformant = other.type_of_within(budget)?.is_conformant(target_type);
        budget.restore(mark);
        Some(conformant)
      }
    }
  }

  /// Checks if a value is conformant with specified target type.
  pub fn is_conformant(&self, target_type: &FeelType) -> bool {
    if matches!(target_type, FeelType::Any) {
      return true;
    }
    match self {
      Value::Null(_) => true,
      Value::Boolean(_) => matches!(target_type, FeelType::Boolean),
      Value::Number(_) => matches!(target_type, FeelType::Number),
      Value::String(_) => matches!(target_type, FeelType::String),
      Value::Date(_) => matches!(target_type, FeelType::Date),
      Value::Time(_) => matches!(target_type, FeelType::Time),
      Value::DateTime(_) => matches!(target_type, FeelType::DateTime),
      Value::DaysAndTimeDuration(_) => matches!(target_type, FeelType::DaysAndTimeDuration),
      Value::YearsAndMonthsDuration(_) => matches!(target_type, FeelType::YearsAndMonthsDuration),
      Value::Range(r_start, _, r_end, _) => {
        if let FeelType::Range(range_type) = target_type {
          return r_start.is_conformant(range_type) && r_end.is_conformant(range_type);
        }
        false
      }
      Value::List(items) => {
        if let FeelType::List(list_type) = target_type {
          for item in items {
            if !item.is_conformant(list_type) {
              return false;
            }
          }
          return true;
        }
        false
      }
      Value::Context(context) => {
        if let FeelType::Context(type_context) = target_type {
          for (name, entry_type) in type_context.iter() {
            if let Some(entry_value) = context.get_entry(name) {
              if !entry_value.is_conformant(entry_type) {
                return false;
              }
            } else {
              return false;
            }
          }
          return true;
        }
        false
      }
      Value::FunctionDefinition(function) => {
        if let FeelType::Function(t_parameters, t_result) = target_type {
          if function.parameters().len() != t_parameters.len() {
            return false;
          }
          if !function.result_type().is_conformant(t_result) {
            return false;
          }
          for (i, (_, parameter_type)) in function.parameters().iter().enumerate() {
            if !parameter_type.is_conformant(&t_parameters[i]) {
              return false;
            }
          }
          return true;
        }
        false
      }
      _ => false,
    }
  }

  /// Returns value coerced to specified type.
  /// When a value appears in a certain context, it must be compatible
  /// with a type expected in that context, called the target type.
  /// After the type of the value is known, an implicit conversion
  /// from the type of the value to the target type can be performed.
  /// If an implicit conversion is mandatory but it cannot be performed,
  /// the result is `None`.
  ///
  /// There are several possible type conversions:
  ///
  /// - to singleton list:
  ///
  ///  When the type of the value is `T` and the target type is `List<T>`,
  ///  the simple value is converted to a singleton list.
  ///
  /// - from singleton list:
  ///
  ///  When the type of the value is `List<T>`, and the value is a singleton list
  ///  and the target type is T, the value is converted by unwrapping the first element.
  ///
  /// - conforms to:
  ///
  ///  When the type of the value is T1, the target type is T2, and T1 conforms to T2,
  ///  the value remains unchanged, otherwise the result is `None`.
  ///
  /// All these conversion rules are implemented in this function and in its owned form
  /// [Value::try_into_coerced].
  ///
  /// `None` when the value does not conform before or after the implicit conversions (§10.3.2.9.4);
  /// in an invocation context the result of the invocation is then null.
  pub fn try_coerced(&self, target_type: &FeelType) -> Option<Value> {
    if self.needs_no_coercion(target_type) {
      Some(self.clone())
    } else {
      self.converted(target_type)
    }
  }

  /// `true` when [Value::try_coerced] returns the value unchanged: a function definition
  /// (whatever the target type), a built-in function whose type conforms to the target type,
  /// or a value that conforms to it.
  fn needs_no_coercion(&self, target_type: &FeelType) -> bool {
    match self {
      Value::FunctionDefinition(_) => true,
      Value::BuiltInFunction(bif) if bif.feel_type().is_conformant(target_type) => true,
      _ => self.is_conformant(target_type),
    }
  }

  /// The implicit conversions of [Value::try_coerced] of a value that needs one: from date to
  /// date and time, from a singleton list, to a singleton list; `None` when none applies.
  fn converted(&self, target_type: &FeelType) -> Option<Value> {
    match self {
      // from date to date and time: UTC midnight (DMN 1.6 §10.3.2.9.4 and §10.3.2.3.5)
      Value::Date(date) if *target_type == FeelType::DateTime => {
        return Some(Value::DateTime(FeelDateTime::new(date.clone(), FeelTime::midnight(FeelZone::Utc))));
      }
      // from singleton list
      Value::List(items) => {
        if items.len() == 1 {
          let value = items[0].clone();
          if value.is_conformant(target_type) {
            return Some(value);
          }
          if let Value::Date(date) = &value
            && *target_type == FeelType::DateTime
          {
            return Some(Value::DateTime(FeelDateTime::new(date.clone(), FeelTime::midnight(FeelZone::Utc))));
          }
        }
      }
      // to singleton list
      value => {
        if let FeelType::List(list_type) = target_type
          && value.is_conformant(list_type)
        {
          return Some(Value::List(vec![value.clone()].into()));
        }
      }
    }
    None
  }

  /// Returns value coerced to specified type, or null when it can not be converted
  /// (the binding contexts of §10.3.2.9.4; see [Value::try_coerced]).
  pub fn coerced(&self, target_type: &FeelType) -> Value {
    self.try_coerced(target_type).unwrap_or_else(|| value_null!("after coercion"))
  }

  /// [Value::try_coerced] of an owned value: a value that needs no conversion is returned
  /// itself instead of a copy.
  pub fn try_into_coerced(self, target_type: &FeelType) -> Option<Value> {
    if self.needs_no_coercion(target_type) { Some(self) } else { self.converted(target_type) }
  }

  /// [Value::coerced] of an owned value (see [Value::try_into_coerced]).
  pub fn into_coerced(self, target_type: &FeelType) -> Value {
    self.try_into_coerced(target_type).unwrap_or_else(|| value_null!("after coercion"))
  }

  /// Tries to convert `xsd:integer` string into valid [Value] representing a number.
  pub fn try_from_xsd_integer(text: &str) -> Result<Self> {
    let value = text.parse::<FeelNumber>().map_err(|_| err_invalid_xsd_integer(text))?;
    Ok(Value::Number(value))
  }

  /// Tries to convert `xsd:decimal` string into valid [Value] representing a number.
  pub fn try_from_xsd_decimal(text: &str) -> Result<Self> {
    let value = text.parse::<FeelNumber>().map_err(|_| err_invalid_xsd_decimal(text))?;
    Ok(Value::Number(value))
  }

  /// Tries to convert `xsd:double` string into valid [Value] representing a number.
  pub fn try_from_xsd_double(text: &str) -> Result<Self> {
    let value = text.parse::<FeelNumber>().map_err(|_| err_invalid_xsd_double(text))?;
    Ok(Value::Number(value))
  }

  /// Tries to convert `xsd:boolean` string into valid [Value] representing a boolean.
  pub fn try_from_xsd_boolean(text: &str) -> Result<Self> {
    match text {
      "true" | "1" => Ok(Value::Boolean(true)),
      "false" | "0" => Ok(Value::Boolean(false)),
      _ => Err(err_invalid_xsd_boolean(text)),
    }
  }

  /// Tries to convert `xsd:date` string into valid [Value] representing a date.
  /// FEEL date format is fully conformant with `xsd:date`.
  pub fn try_from_xsd_date(text: &str) -> Result<Self> {
    if let Ok(feel_date) = FeelDate::from_str(text) {
      return Ok(Value::Date(feel_date));
    }
    Err(err_invalid_xsd_date(text))
  }

  /// Tries to convert `xsd:time` string into valid [Value] representing a time.
  /// FEEL time format is fully conformant with `xsd:time`. Fraction digits beyond nanoseconds
  /// are truncated with a warning ([warn_truncated_fraction]).
  pub fn try_from_xsd_time(text: &str) -> Result<Self> {
    if let Ok((feel_time, dropped)) = FeelTime::parse_reporting_truncation(text) {
      warn_truncated_fraction(text, dropped);
      return Ok(Value::Time(feel_time));
    }
    Err(err_invalid_xsd_time(text))
  }

  /// Tries to convert `xsd:dateTime` string into valid [Value] representing a date and time.
  /// FEEL date and time format is fully conformant with `xsd:dateTime`. Fraction digits beyond
  /// nanoseconds are truncated with a warning ([warn_truncated_fraction]).
  pub fn try_from_xsd_date_time(text: &str) -> Result<Self> {
    let (date_time, dropped) = FeelDateTime::parse_reporting_truncation(text).map_err(|_| err_invalid_xsd_date_time(text))?;
    warn_truncated_fraction(text, dropped);
    Ok(Value::DateTime(date_time))
  }

  /// Tries to convert `xsd:duration` string into valid [Value] representing a date and time.
  /// FEEL durations are conformant with `xsd:duration` but spit into two ranges. Fraction digits
  /// beyond nanoseconds are truncated with a warning ([warn_truncated_fraction]).
  pub fn try_from_xsd_duration(text: &str) -> Result<Self> {
    if let Ok(ym_duration) = FeelYearsAndMonthsDuration::try_from(text) {
      return Ok(Value::YearsAndMonthsDuration(ym_duration));
    }
    if let Ok((dt_duration, dropped)) = FeelDaysAndTimeDuration::parse_reporting_truncation(text) {
      warn_truncated_fraction(text, dropped);
      return Ok(Value::DaysAndTimeDuration(dt_duration));
    }
    Err(err_invalid_xsd_duration(text))
  }
}

/// The warning that the seconds of `source`, a temporal literal or text (or a number of seconds),
/// were truncated to nanoseconds and lost the fraction digits `dropped`. DMN 1.6 holds no time
/// finer than a nanosecond, so the digits are truncated and the evaluation says so.
pub fn truncated_fraction_warning(source: &(impl MessageArg + ?Sized), dropped: &str) -> String {
  compose(format_args!(
    "fractional seconds of '{}' truncated to nanoseconds; dropped digits: '{}'",
    arg(source),
    arg(dropped)
  ))
}

/// Records [truncated_fraction_warning] in the active collection when digits were `dropped`.
pub fn warn_truncated_fraction(source: &(impl MessageArg + ?Sized), dropped: Option<&str>) {
  if let Some(dropped) = dropped {
    diagnostics::warn(&truncated_fraction_warning(source, dropped));
  }
}

/// Type alias to a collection of values.
pub type Values = Vec<Value>;

/// The type of the items of a list, `Null` for an empty one: the type of the first item when
/// every other item conforms to it, `Any` otherwise. The first item's type is built once
/// (building it twice made the cost exponential in the nesting depth of lists); the other items
/// are only checked against it, without building their context types.
fn list_item_type(values: &[Value], budget: &mut impl Budget) -> Option<FeelType> {
  let Some((first, rest)) = values.split_first() else {
    budget.take(1)?;
    return Some(FeelType::Null);
  };
  let item_type = first.type_of_within(budget)?;
  for item in rest {
    if !item.type_is_conformant_within(&item_type, budget)? {
      budget.take(1)?;
      return Some(FeelType::Any);
    }
  }
  Some(item_type)
}

/// The number of type nodes a type may still build: [Unlimited] for [Value::type_of], [Nodes]
/// for a message.
trait Budget {
  /// Takes `nodes` type nodes; `None` when fewer are left.
  fn take(&mut self, nodes: usize) -> Option<()>;

  /// A copy of `feel_type` when its nodes are left.
  fn copy(&mut self, feel_type: &FeelType) -> Option<FeelType>;

  /// The nodes left, for [restore](Self::restore).
  fn mark(&self) -> usize;

  /// Gives back the nodes taken since `mark`, of a type that was dropped.
  fn restore(&mut self, mark: usize);
}

/// No limit: every check is a constant, so [Value::type_of] pays nothing for the budget.
struct Unlimited;

impl Budget for Unlimited {
  fn take(&mut self, _: usize) -> Option<()> {
    Some(())
  }

  fn copy(&mut self, feel_type: &FeelType) -> Option<FeelType> {
    Some(feel_type.clone())
  }

  fn mark(&self) -> usize {
    0
  }

  fn restore(&mut self, _: usize) {}
}

/// At most this many type nodes.
struct Nodes(usize);

impl Budget for Nodes {
  fn take(&mut self, nodes: usize) -> Option<()> {
    self.0 = self.0.checked_sub(nodes)?;
    Some(())
  }

  fn copy(&mut self, feel_type: &FeelType) -> Option<FeelType> {
    self.take(type_nodes(feel_type))?;
    Some(feel_type.clone())
  }

  fn mark(&self) -> usize {
    self.0
  }

  fn restore(&mut self, mark: usize) {
    self.0 = mark;
  }
}

/// The number of nodes of a type.
fn type_nodes(feel_type: &FeelType) -> usize {
  1 + match feel_type {
    FeelType::Context(entries) => entries.values().map(type_nodes).sum(),
    FeelType::Function(parameter_types, result_type) => parameter_types.iter().map(type_nodes).sum::<usize>() + type_nodes(result_type),
    FeelType::List(item_type) | FeelType::Range(item_type) => type_nodes(item_type),
    _ => 0,
  }
}

/// The type nodes a message may build for the type of one list item ([TypeOf]).
const TYPE_NODE_BUDGET: usize = 1_000;

/// The type of a value as a message argument: the text of [Value::type_of], written from the
/// value itself, so a message never builds the whole type of a value.
///
/// The type of a context is written entry by entry, so entries past a preview are never visited.
/// The type of a list builds the type of its first item, of at most 1,000 type nodes, and checks
/// the other items against it: `list<…>` when it would hold more. Declared types (of functions,
/// formal parameters and type values) are written as they are, without a copy.
pub struct TypeOf<'a>(pub &'a Value);

impl Display for TypeOf<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self.0 {
      Value::Context(context) => write_context_type(f, context.iter().map(|(name, value)| (name, TypeOf(value)))),
      Value::List(values) => match list_item_type(values, &mut Nodes(TYPE_NODE_BUDGET)) {
        Some(item_type) => write_list_type(f, item_type),
        None => write_list_type(f, "…"),
      },
      Value::FunctionDefinition(function) => write_function_type(f, function.parameters().iter().map(|(_, feel_type)| feel_type), function.result_type()),
      Value::ContextType(feel_type) | Value::ContextTypeEntry(_, feel_type) | Value::FeelType(feel_type) | Value::FormalParameter(_, feel_type) => fmt::Display::fmt(feel_type, f),
      Value::IntervalEnd(value, _) | Value::IntervalStart(value, _) => fmt::Display::fmt(&TypeOf(value), f),
      other => fmt::Display::fmt(&other.type_of(), f),
    }
  }
}

impl MessageArg for TypeOf<'_> {
  fn write_message(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Display::fmt(self, f)
  }

  /// The size of the type itself: the entries of a context type.
  fn size(&self) -> Option<Size> {
    match self.0 {
      Value::Context(context) => Some(Size::Entries(context.len())),
      Value::ContextType(feel_type) | Value::ContextTypeEntry(_, feel_type) | Value::FeelType(feel_type) | Value::FormalParameter(_, feel_type) => feel_type.size(),
      Value::IntervalEnd(value, _) | Value::IntervalStart(value, _) => TypeOf(value).size(),
      _ => None,
    }
  }

  /// The type of a value that holds no other values is a short name.
  fn is_short(&self) -> bool {
    matches!(self.0, Value::String(_)) || self.0.is_short_scalar()
  }
}

impl MessageArg for Value {
  fn write_message(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Value::Number(number) => number.write_message(f),
      other => fmt::Display::fmt(other, f),
    }
  }

  fn size(&self) -> Option<Size> {
    match self {
      Value::List(items) => Some(Size::Items(items.len())),
      Value::ExpressionList(items) => Some(Size::Items(items.len())),
      Value::Context(context) => context.size(),
      Value::String(text) => text.as_str().size(),
      _ => None,
    }
  }

  /// Numbers, booleans, temporal values, a null without a reason and a string of at most 118
  /// bytes (with its quotes) are short.
  fn is_short(&self) -> bool {
    match self {
      Value::String(text) => text.len() + 2 <= PREVIEW_CHARS,
      other => other.is_short_scalar(),
    }
  }
}

/// Characters of the plain text of a number quoted in a message at most; a longer one is
/// quoted in scientific notation.
const MAX_PLAIN_NUMBER_CHARS: usize = 40;

/// A text of at most [MAX_PLAIN_NUMBER_CHARS] bytes, written without allocating; a longer one is refused.
struct ShortText {
  bytes: [u8; MAX_PLAIN_NUMBER_CHARS],
  len: usize,
}

impl ShortText {
  fn as_str(&self) -> &str {
    // only whole `str`s are written into `bytes`
    std::str::from_utf8(&self.bytes[..self.len]).unwrap_or_default()
  }
}

impl fmt::Write for ShortText {
  fn write_str(&mut self, s: &str) -> fmt::Result {
    let end = self.len + s.len();
    if end > MAX_PLAIN_NUMBER_CHARS {
      return Err(fmt::Error);
    }
    self.bytes[self.len..end].copy_from_slice(s.as_bytes());
    self.len = end;
    Ok(())
  }
}

impl Value {
  /// Whether this is a number, a boolean, a temporal value or a null without a reason: a value
  /// whose text and type name are a few dozen characters at most.
  fn is_short_scalar(&self) -> bool {
    matches!(
      self,
      Value::Number(_)
        | Value::Boolean(_)
        | Value::Null(None)
        | Value::Date(_)
        | Value::Time(_)
        | Value::DateTime(_)
        | Value::DaysAndTimeDuration(_)
        | Value::YearsAndMonthsDuration(_)
    )
  }
}

impl MessageArg for FeelNumber {
  /// The plain text, or scientific notation when that is longer than 40 characters (`9e6144`
  /// prints as 6145 digits), so that a message stays readable.
  fn write_message(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut short = ShortText {
      bytes: [0; MAX_PLAIN_NUMBER_CHARS],
      len: 0,
    };
    if fmt::Write::write_fmt(&mut short, format_args!("{self}")).is_ok() {
      return f.write_str(short.as_str());
    }
    let plain = self.to_string();
    let (sign, unsigned) = match plain.strip_prefix('-') {
      Some(rest) => ("-", rest),
      None => ("", plain.as_str()),
    };
    let (int_part, frac_part) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let digits = format!("{int_part}{frac_part}");
    let Some(first) = digits.find(|c: char| c != '0') else {
      return f.write_str(&plain);
    };
    let exponent = int_part.len() as i64 - first as i64 - 1;
    let mantissa = digits[first..].trim_end_matches('0');
    let (head, tail) = mantissa.split_at(1);
    if tail.is_empty() {
      write!(f, "{sign}{head}E{exponent:+}")
    } else {
      write!(f, "{sign}{head}.{tail}E{exponent:+}")
    }
  }

  /// The text after the 40-character rule is at most a few dozen characters.
  fn is_short(&self) -> bool {
    true
  }
}

/// Converts a collection of values into string.
pub fn values_to_string(values: &[Value]) -> String {
  struct List<'a>(&'a [Value]);
  impl Display for List<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write_values(f, self.0)
    }
  }
  List(values).to_string()
}

/// Writes a list of values, `[a, b, c]`, as it goes.
fn write_values(f: &mut fmt::Formatter<'_>, values: &[Value]) -> fmt::Result {
  f.write_str("[")?;
  write_separated(f, values, |f, value| write!(f, "{value}"))?;
  f.write_str("]")
}

/// Converts a collection of values into `FEEL` string.
pub fn values_to_feel_string(values: &[Value]) -> String {
  format!("[{}]", values.iter().map(|value| value.to_feel_string()).collect::<Vec<String>>().join(", "))
}

/// Converts a collection of values into `JSON` string.
pub fn values_to_jsonify(values: &[Value]) -> String {
  format!("[{}]", values.iter().map(|value| value.jsonify()).collect::<Vec<String>>().join(", "))
}
