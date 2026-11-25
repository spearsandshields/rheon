//! The type of an expression, found from its syntax tree without evaluating it: for editors,
//! which complete the entries of a value after a dot and show the types of what they offer.
//!
//! The answer is the type the value has when the expression gives one: an expression can
//! always give null instead (DMN 1.6 clause 10.3.2.1). Where the tree does not say the type
//! (the result of a function of the host, a name without a value, a list of items of several
//! types) the answer is `Any`. Nothing here is used by the evaluation.
//!
//! The walk of the tree is the analysis ([crate::analysis]); here are its entry points for the
//! types of FEEL as values ([FeelType]) and the rules that read them.

use rheon_feel::values::Value;
use rheon_feel::{FeelScope, FeelType, Name};
use rheon_feel_parser::AstNode;
use std::collections::BTreeMap;

/// The deepest value of the scope whose type is read: below it the type is `Any`.
pub(crate) const MAX_VALUE_DEPTH: usize = 16;

/// Returns the type of the value of `node` in `scope`, where the names of `bound` (the
/// variables and parameters of the constructs around the expression, the innermost last) have
/// the types given with them.
pub fn infer_type(node: &AstNode, scope: &FeelScope, bound: &[(Name, FeelType)]) -> FeelType {
  infer_type_with(node, scope, bound, &[])
}

/// [infer_type] where names of `bound` are bound to function literals of the text: `literals`
/// gives the index of such a name in `bound` and the literal (an editor that types a piece of a
/// context literal gives the functions of the entries before it). An invocation of the name
/// types the body of the literal with the types of the arguments.
pub fn infer_type_with<'n>(node: &'n AstNode, scope: &FeelScope, bound: &[(Name, FeelType)], literals: &'n [(usize, AstNode)]) -> FeelType {
  crate::analysis::infer(node, scope, bound, literals)
}

/// Returns the type of a value of the scope as an editor shows it: of a context the types of
/// its entries, of a list the type of its first item (the items of a list of a host are of one
/// kind; reading them all would cost what the list is long).
pub fn type_of_value(value: &Value) -> FeelType {
  value_type(value, 0)
}

fn value_type(value: &Value, depth: usize) -> FeelType {
  if depth >= MAX_VALUE_DEPTH {
    return FeelType::Any;
  }
  match value {
    Value::Context(context) => FeelType::Context(context.iter().map(|(name, value)| (name.clone(), value_type(value, depth + 1))).collect()),
    Value::List(items) => FeelType::List(Box::new(items.first().map_or(FeelType::Any, |item| value_type(item, depth + 1)))),
    Value::Null(_) => FeelType::Any,
    // a type in the place of a value says the type of the name (an outline, a DMN declaration)
    Value::FeelType(feel_type) => feel_type.clone(),
    other => other.type_of(),
  }
}

/// Returns the type that `text` writes as FEEL writes a type (`number`, `list<string>`,
/// `range<date>`, `context<a: number>`, `function<number>->string`), or None when it is no
/// type of FEEL.
pub fn type_of_text(text: &str) -> Option<FeelType> {
  let scope = FeelScope::default();
  let node = rheon_feel_parser::parse_expression(&scope, &format!("null instance of {text}")).ok()?;
  match &node {
    AstNode::InstanceOf(value, written) if matches!(value.as_ref(), AstNode::Null) => type_written(written),
    _ => None,
  }
}

/// The type that a type node of the parser writes, None for a name that is no type of FEEL.
pub(crate) fn type_written(node: &AstNode) -> Option<FeelType> {
  Some(match node {
    AstNode::FeelType(feel_type) => feel_type.clone(),
    AstNode::ListType(item) => FeelType::List(Box::new(type_written(item)?)),
    AstNode::RangeType(endpoint) => FeelType::Range(Box::new(type_written(endpoint)?)),
    AstNode::ContextType(entries) => {
      let mut types = BTreeMap::new();
      for entry in entries {
        let AstNode::ContextTypeEntry(key, entry_type) = entry else {
          return None;
        };
        let AstNode::ContextTypeEntryKey(name) = key.as_ref() else {
          return None;
        };
        types.insert(name.clone(), type_written(entry_type)?);
      }
      FeelType::Context(types)
    }
    AstNode::FunctionType(parameters, result) => {
      let AstNode::ParameterTypes(items) = parameters.as_ref() else {
        return None;
      };
      FeelType::Function(items.iter().map(type_written).collect::<Option<Vec<_>>>()?, Box::new(type_written(result)?))
    }
    _ => return None,
  })
}

/// Returns the members that a value of the type `feel_type` has after a dot, with their types:
/// the entries of a context, the properties of a date, a time, a duration and a range, and for
/// a list the members of its items, each as the list of its values.
pub fn members_of(feel_type: &FeelType) -> Vec<(String, FeelType)> {
  let properties = |names: &[&str]| names.iter().map(|name| (name.to_string(), FeelType::Number)).collect::<Vec<_>>();
  match feel_type {
    FeelType::Context(entries) => entries.iter().map(|(name, entry_type)| (name.to_string(), entry_type.clone())).collect(),
    FeelType::List(item_type) => members_of(item_type)
      .into_iter()
      .map(|(name, member_type)| (name, FeelType::List(Box::new(member_type))))
      .collect(),
    FeelType::Date => properties(&["year", "month", "day", "weekday", "value"]),
    FeelType::DateTime => {
      let mut members = properties(&["year", "month", "day", "weekday", "hour", "minute", "second", "value"]);
      members.push(("time offset".to_string(), FeelType::DaysAndTimeDuration));
      members.push(("timezone".to_string(), FeelType::String));
      members
    }
    FeelType::Time => {
      let mut members = properties(&["hour", "minute", "second", "value"]);
      members.push(("time offset".to_string(), FeelType::DaysAndTimeDuration));
      members.push(("timezone".to_string(), FeelType::String));
      members
    }
    FeelType::YearsAndMonthsDuration => properties(&["years", "months", "value"]),
    FeelType::DaysAndTimeDuration => properties(&["days", "hours", "minutes", "seconds", "value"]),
    FeelType::Range(endpoint) => vec![
      ("start".to_string(), endpoint.as_ref().clone()),
      ("end".to_string(), endpoint.as_ref().clone()),
      ("start included".to_string(), FeelType::Boolean),
      ("end included".to_string(), FeelType::Boolean),
    ],
    _ => vec![],
  }
}

/// The type that a type written as text in the table of signatures names, `Any` for a text
/// that names several types.
pub(crate) fn signature_type(text: &str) -> FeelType {
  match text {
    "boolean" => FeelType::Boolean,
    "number" => FeelType::Number,
    "string" => FeelType::String,
    "date" => FeelType::Date,
    "time" => FeelType::Time,
    "date and time" => FeelType::DateTime,
    "days and time duration" => FeelType::DaysAndTimeDuration,
    "years and months duration" => FeelType::YearsAndMonthsDuration,
    "list" => FeelType::List(Box::new(FeelType::Any)),
    "list<number>" => FeelType::List(Box::new(FeelType::Number)),
    "list<string>" => FeelType::List(Box::new(FeelType::String)),
    "list<context>" => FeelType::List(Box::new(FeelType::Context(BTreeMap::new()))),
    "context" => FeelType::Context(BTreeMap::new()),
    "range" => FeelType::Range(Box::new(FeelType::Any)),
    _ => FeelType::Any,
  }
}

/// The type of an `@` literal, by the shape of its text.
pub(crate) fn at_literal(text: &str) -> FeelType {
  let text = text.trim();
  if text.starts_with('P') || text.starts_with("-P") {
    return if text.contains('T') || text.contains('D') {
      FeelType::DaysAndTimeDuration
    } else {
      FeelType::YearsAndMonthsDuration
    };
  }
  let date = text
    .as_bytes()
    .iter()
    .skip(1)
    .position(|&byte| byte == b'-')
    .is_some_and(|dash| text[..dash + 1].trim_start_matches('-').chars().all(|ch| ch.is_ascii_digit()));
  match (date, text.contains('T')) {
    (true, true) => FeelType::DateTime,
    (true, false) => FeelType::Date,
    (false, _) if text.contains(':') => FeelType::Time,
    _ => FeelType::Any,
  }
}

/// The type that the type written after a parameter is.
pub(crate) fn written_type(node: &AstNode) -> FeelType {
  match node {
    AstNode::FeelType(feel_type) => feel_type.clone(),
    AstNode::ListType(item) => FeelType::List(Box::new(written_type(item))),
    AstNode::RangeType(endpoint) => FeelType::Range(Box::new(written_type(endpoint))),
    AstNode::ContextType(entries) => FeelType::Context(
      entries
        .iter()
        .filter_map(|entry| match entry {
          AstNode::ContextTypeEntry(key, entry_type) => match key.as_ref() {
            AstNode::ContextTypeEntryKey(name) => Some((name.clone(), written_type(entry_type))),
            _ => None,
          },
          _ => None,
        })
        .collect(),
    ),
    AstNode::FunctionType(parameters, result) => FeelType::Function(
      match parameters.as_ref() {
        AstNode::ParameterTypes(items) => items.iter().map(written_type).collect(),
        _ => vec![],
      },
      Box::new(written_type(result)),
    ),
    _ => FeelType::Any,
  }
}

/// The type of `a + b`. An operand of a type not known (`Any`) beside a number or a string
/// gives a number or a string: FEEL adds a number only to a number and a string only to a
/// string (Tables 57 and 58), so a sum that is not null is of that type.
pub(crate) fn addition(a: &FeelType, b: &FeelType) -> FeelType {
  use FeelType::*;
  match (a, b) {
    (Number, Number | Any) | (Any, Number) => Number,
    (String, String | Any) | (Any, String) => String,
    (DaysAndTimeDuration, DaysAndTimeDuration) => DaysAndTimeDuration,
    (YearsAndMonthsDuration, YearsAndMonthsDuration) => YearsAndMonthsDuration,
    (temporal @ (Date | DateTime), DaysAndTimeDuration | YearsAndMonthsDuration) | (DaysAndTimeDuration | YearsAndMonthsDuration, temporal @ (Date | DateTime)) => temporal.clone(),
    (Time, DaysAndTimeDuration) | (DaysAndTimeDuration, Time) => Time,
    _ => Any,
  }
}

/// The type of `a - b`; a number and an operand of a type not known give a number, as for
/// [addition].
pub(crate) fn subtraction(a: &FeelType, b: &FeelType) -> FeelType {
  use FeelType::*;
  match (a, b) {
    (Number, Number | Any) | (Any, Number) => Number,
    (DaysAndTimeDuration, DaysAndTimeDuration) => DaysAndTimeDuration,
    (YearsAndMonthsDuration, YearsAndMonthsDuration) => YearsAndMonthsDuration,
    (Date | DateTime, Date | DateTime) | (Time, Time) => DaysAndTimeDuration,
    (temporal @ (Date | DateTime), DaysAndTimeDuration | YearsAndMonthsDuration) => temporal.clone(),
    (Time, DaysAndTimeDuration) => Time,
    _ => Any,
  }
}
