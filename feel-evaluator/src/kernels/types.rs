//! Types: `instance of` and the types that type expressions build from their evaluated parts
//! (DMN 1.6 §10.3.2.9.1, grammar rules 54 and 55).

use rheon_feel::message::arg;
use rheon_feel::value_null;
use rheon_feel::values::{TypeOf, VALUE_FALSE, VALUE_TRUE, Value};
use rheon_feel::{FeelType, Name};
use std::collections::BTreeMap;

/// FEEL `lhv instance of rhv`: with a type on the right the conformance of the value; otherwise
/// null, as the right operand maps to no node of the type lattice (DMN 1.7 Beta 1, Table 62).
pub(crate) fn instance_of(lhv: Value, rhv: Value) -> Value {
  if let Value::FeelType(feel_type) = rhv {
    match lhv {
      // a value of a simple type is an instance of its type and of the abstract types above it
      // (DMN 1.7 Beta 1, Figure 10-26: comparable, temporal, duration)
      Value::Number { .. } => Value::Boolean(FeelType::Number.is_conformant(&feel_type)),
      Value::String { .. } => Value::Boolean(FeelType::String.is_conformant(&feel_type)),
      Value::Boolean { .. } => Value::Boolean(FeelType::Boolean.is_conformant(&feel_type)),
      Value::Date { .. } => Value::Boolean(FeelType::Date.is_conformant(&feel_type)),
      Value::DateTime { .. } => Value::Boolean(FeelType::DateTime.is_conformant(&feel_type)),
      Value::Time { .. } => Value::Boolean(FeelType::Time.is_conformant(&feel_type)),
      Value::YearsAndMonthsDuration { .. } => Value::Boolean(FeelType::YearsAndMonthsDuration.is_conformant(&feel_type)),
      Value::DaysAndTimeDuration { .. } => Value::Boolean(FeelType::DaysAndTimeDuration.is_conformant(&feel_type)),
      Value::Null { .. } => match feel_type {
        FeelType::Null => VALUE_TRUE,
        _ => VALUE_FALSE,
      },
      value @ (Value::Range { .. }
      | Value::UnaryGreater(_)
      | Value::UnaryGreaterOrEqual(_)
      | Value::UnaryLess(_)
      | Value::UnaryLessOrEqual(_)
      | Value::UnaryEqual(_)
      | Value::UnaryNotEqual(_)) => match feel_type {
        FeelType::Any => VALUE_TRUE,
        expected => Value::Boolean(value.has_type(&expected)),
      },
      // a list or a context is an instance of the types it conforms to (Table 62): each item of a
      // list conforms to the item type, each entry of the target type is an entry that conforms
      value @ (Value::List { .. } | Value::Context { .. }) => Value::Boolean(value.is_conformant(&feel_type)),
      // a function is an instance of the function types its type conforms to (10.3.2.9.2): its
      // declared result type counts, Any when it is not declared (the result type of a function
      // literal is not inferred), as a binding does not hold it against the target
      value @ Value::FunctionDefinition { .. } => Value::Boolean(value.type_of().is_conformant(&feel_type)),
      Value::BuiltInFunction(bif) => match feel_type {
        FeelType::Any => VALUE_TRUE,
        FeelType::Function(parameters, result) => Value::Boolean(rheon_feel::signatures::conforms(&bif, &parameters, &result, false)),
        _ => VALUE_FALSE,
      },
      // the parameters of an external function are not known here: it is a function, of type Any
      Value::ExternalJavaFunction(..) | Value::ExternalPmmlFunction(..) => Value::Boolean(feel_type == FeelType::Any),
      other => value_null!("invalid value in 'instance of' operator: {}", arg(&other)),
    }
  } else if let Value::Null(Some(_)) = rhv {
    // the reason why the right operand is no type
    rhv
  } else {
    // a right operand that maps to no node of the type lattice: the test is null (Table 62)
    value_null!("instance of expected a type on the right, found {}", arg(&TypeOf(&rhv)))
  }
}

/// FEEL `lhv as rhv` (DMN 1.7 Beta 1, Table 62): the value itself when it is an instance of the
/// type, otherwise null: "If e1 instance of e2 is false or null, the result is null". A value that
/// is of another type gives a null that explains it and is no error, as a test that answers no;
/// a right operand that is no type gives the null of `instance of`.
pub(crate) fn as_type(lhv: Value, rhv: Value) -> Value {
  // null is null, of whatever type
  if lhv.is_null() && matches!(rhv, Value::FeelType(_)) {
    return lhv;
  }
  match instance_of(lhv.clone(), rhv.clone()) {
    Value::Boolean(true) => lhv,
    Value::Boolean(false) => match rhv {
      Value::FeelType(feel_type) => value_null!(note: "a value of type {} is no {}", arg(&TypeOf(&lhv)), feel_type),
      _ => value_null!(note: "a value of type {} is no value of the type", arg(&TypeOf(&lhv))),
    },
    null => null,
  }
}

/// The type `list<t>` of the evaluated element type `element`.
pub(crate) fn list_type(element: Value) -> Value {
  match element {
    Value::FeelType(feel_type) => Value::FeelType(FeelType::List(Box::new(feel_type))),
    other => value_null!("type 'list<T>' expected a type for T, found {}", arg(&TypeOf(&other))),
  }
}

/// The type `range<t>` of the evaluated element type `element`.
pub(crate) fn range_type(element: Value) -> Value {
  match element {
    Value::FeelType(feel_type) => Value::FeelType(FeelType::Range(Box::new(feel_type))),
    other => value_null!("type 'range<T>' expected a type for T, found {}", arg(&TypeOf(&other))),
  }
}

/// An entry `name: type` of a context type, from its evaluated key and type.
pub(crate) fn context_type_entry(lhv: Value, rhv: Value) -> Value {
  if let Value::ContextTypeEntryKey(name) = lhv {
    if let Value::FeelType(feel_type) = rhv {
      Value::ContextTypeEntry(name, feel_type)
    } else {
      value_null!("context type entry '{}' expected a type, found {}", arg(&name), arg(&TypeOf(&rhv)))
    }
  } else {
    value_null!("expected a name in context type entry")
  }
}

/// Adds the evaluated entry `entry` of a context type to `entries`, or gives the null that ends the
/// context type: the entry's own null, or an error for a value that is no entry.
pub(crate) fn add_context_type_entry(entries: &mut BTreeMap<Name, FeelType>, entry: Value) -> Result<(), Value> {
  match entry {
    Value::ContextTypeEntry(name, feel_type) => {
      entries.insert(name, feel_type);
      Ok(())
    }
    null @ Value::Null(_) => Err(null),
    other => Err(value_null!("expected context type entry, actual value type is {}", arg(&TypeOf(&other)))),
  }
}

/// The type `function<t1, …> -> r` from its evaluated parameter types and result type; a
/// parameter type that is no type is left out.
pub(crate) fn function_type(lhv: Value, rhv: Value) -> Value {
  if let Value::ParameterTypes(types) = lhv {
    if let Value::FeelType(result_type) = rhv {
      let parameter_types = types
        .iter()
        .filter_map(|value| if let Value::FeelType(feel_type) = value { Some(feel_type.clone()) } else { None })
        .collect();
      Value::FeelType(FeelType::Function(parameter_types, Box::new(result_type)))
    } else {
      value_null!("expected function's result type")
    }
  } else {
    value_null!("expected function's parameter types")
  }
}

/// A formal parameter `name: type` of a function definition, from its evaluated name and type.
pub(crate) fn formal_parameter(lhv: Value, rhv: Value) -> Value {
  if let Value::ParameterName(parameter_name) = lhv {
    if let Value::FeelType(parameter_type) = rhv {
      Value::FormalParameter(parameter_name, parameter_type)
    } else {
      value_null!("expected type of the formal parameter")
    }
  } else {
    value_null!("expected name of the formal parameter")
  }
}

/// Adds the evaluated formal parameter `parameter` to `parameters`, or gives the null that ends the
/// parameter list: the parameter's own null, or an error for a value that is no parameter.
pub(crate) fn add_formal_parameter(parameters: &mut Vec<(Name, FeelType)>, parameter: Value) -> Result<(), Value> {
  match parameter {
    Value::FormalParameter(parameter_name, parameter_type) => {
      parameters.push((parameter_name, parameter_type));
      Ok(())
    }
    null @ Value::Null(_) => Err(null),
    other => Err(value_null!("expected formal parameter, actual value type is: {}", arg(&TypeOf(&other)))),
  }
}
