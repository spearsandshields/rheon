//! Types: `instance of` and the types that type expressions build from their evaluated parts
//! (DMN 1.6 §10.3.2.9.1, grammar rules 54 and 55).

use rheon_feel::message::arg;
use rheon_feel::value_null;
use rheon_feel::values::{TypeOf, VALUE_FALSE, VALUE_TRUE, Value};
use rheon_feel::{FeelType, Name};
use std::collections::BTreeMap;

/// FEEL `lhv instance of rhv`: with a type on the right the conformance of the value; otherwise
/// whether the two values have the same type.
pub(crate) fn instance_of(lhv: Value, rhv: Value) -> Value {
  if let Value::FeelType(feel_type) = rhv {
    match lhv {
      Value::Number { .. } => match feel_type {
        FeelType::Any => VALUE_TRUE,
        FeelType::Number => VALUE_TRUE,
        _ => VALUE_FALSE,
      },
      Value::String { .. } => match feel_type {
        FeelType::Any => VALUE_TRUE,
        FeelType::String => VALUE_TRUE,
        _ => VALUE_FALSE,
      },
      Value::Boolean { .. } => match feel_type {
        FeelType::Any => VALUE_TRUE,
        FeelType::Boolean => VALUE_TRUE,
        _ => VALUE_FALSE,
      },
      Value::Date { .. } => match feel_type {
        FeelType::Any => VALUE_TRUE,
        FeelType::Date => VALUE_TRUE,
        _ => VALUE_FALSE,
      },
      Value::DateTime { .. } => match feel_type {
        FeelType::Any => VALUE_TRUE,
        FeelType::DateTime => VALUE_TRUE,
        _ => VALUE_FALSE,
      },
      Value::Time { .. } => match feel_type {
        FeelType::Any => VALUE_TRUE,
        FeelType::Time => VALUE_TRUE,
        _ => VALUE_FALSE,
      },
      Value::YearsAndMonthsDuration { .. } => match feel_type {
        FeelType::Any => VALUE_TRUE,
        FeelType::YearsAndMonthsDuration => VALUE_TRUE,
        _ => VALUE_FALSE,
      },
      Value::DaysAndTimeDuration { .. } => match feel_type {
        FeelType::Any => VALUE_TRUE,
        FeelType::DaysAndTimeDuration => VALUE_TRUE,
        _ => VALUE_FALSE,
      },
      Value::Null { .. } => match feel_type {
        FeelType::Null => VALUE_TRUE,
        _ => VALUE_FALSE,
      },
      value @ Value::Range { .. } => match feel_type {
        FeelType::Any => VALUE_TRUE,
        expected => Value::Boolean(value.has_type(&expected)),
      },
      value @ Value::List { .. } => match feel_type {
        FeelType::Any => VALUE_TRUE,
        expected => Value::Boolean(value.type_instance_of(&expected)),
      },
      value @ Value::Context { .. } => match feel_type {
        FeelType::Any => VALUE_TRUE,
        expected => Value::Boolean(value.type_instance_of(&expected)),
      },
      value @ Value::FunctionDefinition { .. } => match feel_type {
        FeelType::Any => VALUE_TRUE,
        expected => Value::Boolean(value.has_type(&expected)),
      },
      other => value_null!("invalid value in 'instance of' operator: {}", arg(&other)),
    }
  } else {
    Value::Boolean(lhv.has_type_of(&rhv))
  }
}

/// The type `list<t>` of the evaluated element type `element`.
pub(crate) fn list_type(element: Value) -> Value {
  if let Value::FeelType(feel_type) = element {
    Value::FeelType(FeelType::List(Box::new(feel_type)))
  } else {
    value_null!("expected a feel type")
  }
}

/// The type `range<t>` of the evaluated element type `element`.
pub(crate) fn range_type(element: Value) -> Value {
  if let Value::FeelType(feel_type) = element {
    Value::FeelType(FeelType::Range(Box::new(feel_type)))
  } else {
    value_null!("expected a feel type")
  }
}

/// An entry `name: type` of a context type, from its evaluated key and type.
pub(crate) fn context_type_entry(lhv: Value, rhv: Value) -> Value {
  if let Value::ContextTypeEntryKey(name) = lhv {
    if let Value::FeelType(feel_type) = rhv {
      Value::ContextTypeEntry(name, feel_type)
    } else {
      value_null!("expected a type in context type entry")
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
