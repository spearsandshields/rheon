//! A list of one item that is given for a parameter of a built-in function that is no list is
//! that item (DMN 1.6, 10.3.2.9.4, "from singleton list": a list of one item of the type `T`
//! where the type `T` is expected). The parameters and their types are those of the table of
//! signatures ([rheon_feel::signatures]).
//!
//! An argument stays the list when a signature that the invocation fits takes a list or every
//! value there, and when its item is not of the type of the parameter: the function then says
//! what it was given.

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

/// The arguments by position of `bif` with every list of one item that is its item replaced by
/// it; `None` when there is none, which is known without the table when no argument is a list
/// of one item.
pub(super) fn by_position(bif: &Bif, arguments: &[Value]) -> Option<Vec<Value>> {
  if !arguments.iter().any(|argument| one_item(argument).is_some()) {
    return None;
  }
  let built_in = signatures::of(bif)?;
  let item_at = |place: usize| {
    let parameters = built_in.signatures.iter().filter_map(|signature| at_place(signature, place, arguments.len()));
    one_item(&arguments[place]).filter(|item| is_its_item(item, parameters))
  };
  let first = (0..arguments.len()).find(|place| item_at(*place).is_some())?;
  let mut values = arguments.to_vec();
  for (place, value) in values.iter_mut().enumerate().skip(first) {
    if let Some(item) = item_at(place) {
      *value = item.clone();
    }
  }
  Some(values)
}

/// The arguments by name of `bif` with every list of one item that is its item replaced by it;
/// `None` when there is none. The signatures that the invocation fits are those that have
/// every name of it.
pub(super) fn by_name(bif: &Bif, arguments: &BTreeMap<Name, (Value, usize)>) -> Option<BTreeMap<Name, (Value, usize)>> {
  if !arguments.values().any(|(argument, _)| one_item(argument).is_some()) {
    return None;
  }
  let built_in = signatures::of(bif)?;
  let named = |signature: &&Signature, name: &Name| signature.parameters.iter().find(|parameter| parameter.name == name.as_str());
  let fitting = || {
    built_in
      .signatures
      .iter()
      .filter(|signature| signature.named && arguments.keys().all(|name| named(signature, name).is_some()))
  };
  let is_item_of = |name: &Name, item: &Value| is_its_item(item, fitting().filter_map(|signature| named(&signature, name)));
  let item_of = |name: &Name| one_item(&arguments.get(name)?.0).filter(|item| is_item_of(name, item));
  arguments.keys().find(|name| item_of(name).is_some())?;
  let mut values = arguments.clone();
  for (name, value) in &mut values {
    if let Some(item) = item_of(name) {
      value.0 = item.clone();
    }
  }
  Some(values)
}
