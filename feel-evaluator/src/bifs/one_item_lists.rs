//! The conversions of the invocation context for the arguments of a built-in function (DMN 1.7
//! Beta 1, 10.3.2.9.4). A list of one item that is given for a parameter that is no list is that
//! item ("from singleton list": a list of one item of the type `T` where the type `T` is expected),
//! and a value that is no list given for a parameter that takes a list is the list of that one
//! value ("to singleton list": `reverse(5)` is `[5]`). The parameters and their types are those of
//! the table of signatures ([rheon_feel::signatures]).
//!
//! An argument stays the list when a signature that the invocation fits takes a list or every
//! value there, and when its item is not of the type of the parameter: the function then says
//! what it was given. A value becomes a list only where every signature that the invocation fits
//! takes a list there, and null never does: null is no value of the item type (`count(null)` is
//! null, 10.3.4).

use rheon_feel::Name;
use rheon_feel::bif::Bif;
use rheon_feel::signatures::{self, Parameter, Signature};
use rheon_feel::values::Value;
use std::collections::BTreeMap;

/// The item of a list of one item.
fn one_item(value: &Value) -> Option<&Value> {
  match value {
    Value::List(items) if items.len() == 1 => items.first(),
    _ => None,
  }
}

/// Whether a parameter of this type of the table takes a list as it is.
fn takes_a_list(parameter: &Parameter) -> bool {
  parameter.feel_type == "Any" || parameter.feel_type.starts_with("list")
}

/// Whether `value` is of the type of `parameter`, a type of the table or several joined by `or`.
/// Null is of every type.
fn is_of_the_type(value: &Value, parameter: &Parameter) -> bool {
  parameter.feel_type.split(" or ").any(|feel_type| match feel_type {
    "boolean" => matches!(value, Value::Boolean(_)),
    "context" => matches!(value, Value::Context(_)),
    "date" => matches!(value, Value::Date(_)),
    "date and time" => matches!(value, Value::DateTime(_)),
    "days and time duration" => matches!(value, Value::DaysAndTimeDuration(_)),
    "duration" => matches!(value, Value::DaysAndTimeDuration(_) | Value::YearsAndMonthsDuration(_)),
    "function" => matches!(value, Value::FunctionDefinition(_) | Value::BuiltInFunction(_)),
    "number" => matches!(value, Value::Number(_)),
    "range" => matches!(value, Value::Range(..)),
    "string" => matches!(value, Value::String(_)),
    "time" => matches!(value, Value::Time(_)),
    _ => false,
  }) || matches!(value, Value::Null(_))
}

/// Whether a signature of `built_in` has a parameter that takes a list: without one, and without an
/// argument that is a list, no argument is converted (most calls, decided without the table).
fn has_list_parameters(built_in: &signatures::BuiltIn) -> bool {
  built_in
    .signatures
    .iter()
    .any(|signature| signature.parameters.iter().any(|parameter| parameter.feel_type.starts_with("list")))
}

/// Whether every one of the parameters `parameters` takes only a list, and there is one.
fn takes_only_lists<'s>(mut parameters: impl Iterator<Item = &'s Parameter> + Clone) -> bool {
  parameters.clone().next().is_some() && parameters.all(|parameter| parameter.feel_type.starts_with("list"))
}

/// How an argument is converted: to its one item, to the list of it, or not at all.
fn converted<'s>(argument: &Value, parameters: impl Iterator<Item = &'s Parameter> + Clone) -> Option<Value> {
  match argument {
    Value::List(_) => one_item(argument).filter(|item| is_its_item(item, parameters)).cloned(),
    Value::Null(_) => None,
    value if takes_only_lists(parameters) => Some(Value::List(vec![value.clone()].into())),
    _ => None,
  }
}

/// Whether the list of the one item `item` is that item for the parameters `parameters`, those
/// of one place or one name in the signatures that the invocation fits.
fn is_its_item<'s>(item: &Value, mut parameters: impl Iterator<Item = &'s Parameter> + Clone) -> bool {
  !parameters.clone().any(takes_a_list) && parameters.any(|parameter| is_of_the_type(item, parameter))
}

/// The parameter of `signature` for the argument at `place` of `count` arguments by position;
/// `None` when the signature does not take that many.
fn at_place(signature: &Signature, place: usize, count: usize) -> Option<&Parameter> {
  let parameters = signature.parameters;
  let required = parameters.iter().filter(|parameter| !parameter.optional).count();
  let rest = parameters.last().filter(|parameter| parameter.rest);
  if count < required || (rest.is_none() && count > parameters.len()) {
    return None;
  }
  parameters.get(place).or(rest)
}

/// The arguments by position of `bif` with every argument converted that the invocation context
/// converts ([converted]); `None` when none is.
pub(super) fn by_position(bif: &Bif, arguments: &[Value]) -> Option<Vec<Value>> {
  let built_in = signatures::of(bif)?;
  if !arguments.iter().any(|argument| matches!(argument, Value::List(_))) && !has_list_parameters(built_in) {
    return None;
  }
  let convert = |place: usize| {
    let parameters = built_in.signatures.iter().filter_map(|signature| at_place(signature, place, arguments.len()));
    converted(&arguments[place], parameters)
  };
  let first = (0..arguments.len()).find_map(|place| convert(place).map(|value| (place, value)))?;
  let mut values = arguments.to_vec();
  values[first.0] = first.1;
  for (place, slot) in values.iter_mut().enumerate().skip(first.0 + 1) {
    if let Some(value) = convert(place) {
      *slot = value;
    }
  }
  Some(values)
}

/// The arguments by name of `bif` with every argument converted that the invocation context
/// converts ([converted]); `None` when none is. The signatures that the invocation fits are those
/// that have every name of it.
pub(super) fn by_name(bif: &Bif, arguments: &BTreeMap<Name, (Value, usize)>) -> Option<BTreeMap<Name, (Value, usize)>> {
  let built_in = signatures::of(bif)?;
  if !arguments.values().any(|(argument, _)| matches!(argument, Value::List(_))) && !has_list_parameters(built_in) {
    return None;
  }
  let named = |signature: &&Signature, name: &Name| signature.parameters.iter().find(|parameter| parameter.name == name.as_str());
  let fitting = || {
    built_in
      .signatures
      .iter()
      .filter(|signature| signature.named && arguments.keys().all(|name| named(signature, name).is_some()))
  };
  let convert = |name: &Name| converted(&arguments.get(name)?.0, fitting().filter_map(|signature| named(&signature, name)));
  arguments.keys().find(|name| convert(name).is_some())?;
  let mut values = arguments.clone();
  for (name, value) in &mut values {
    if let Some(converted) = convert(name) {
      value.0 = converted;
    }
  }
  Some(values)
}
