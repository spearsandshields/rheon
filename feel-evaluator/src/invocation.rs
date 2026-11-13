//! # Function invocation
//!
//! The one place where arguments are bound to the parameters of a function and the function is applied (DMN 1.6, §10.3.2.13, Table 64):
//! FEEL and boxed invocations, invocables invoked with input data, and the functions called by `sort` and `list replace`.

use crate::bfeel::{self, Dialect};
use crate::bifs;
use crate::builders::finite;
use crate::evaluator_java::evaluate_external_java_function;
use crate::evaluator_pmml::evaluate_external_pmml_function;
use rheon_feel::bif::Bif;
use rheon_feel::context::FeelContext;
use rheon_feel::values::Value;
use rheon_feel::{FeelScope, FeelType, FunctionBody, FunctionDefinition, Name, QualifiedName, value_null};
use std::sync::Arc;

/// Maximum number of nested function invocations during one evaluation.
///
/// Every nested invocation (user-defined recursion such as `f(n - 1)`, or built-in calls nested
/// in each other) uses host stack. Without a limit, deep recursion overflows the stack, which on
/// wasm32 is a trap that kills the engine instance, at a depth that depends on the host (browser
/// worker, Node main thread, ...). With this limit the evaluation stops deterministically at the
/// same depth everywhere.
pub const MAX_CALL_DEPTH: usize = 200;

thread_local! {
  /// Current depth of nested function invocations.
  static CALL_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
  /// Set when [MAX_CALL_DEPTH] was exceeded; all further invocations return null immediately
  /// so the evaluation unwinds quickly. Cleared by [reset_call_depth].
  static CALL_DEPTH_EXCEEDED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Resets the call depth tracking; call before every top-level evaluation.
pub fn reset_call_depth() {
  CALL_DEPTH.with(|depth| depth.set(0));
  CALL_DEPTH_EXCEEDED.with(|exceeded| exceeded.set(false));
}

/// Returns `true` when the last evaluation exceeded [MAX_CALL_DEPTH]; its result is then
/// meaningless (it was cut short) and should be reported as an error.
pub fn call_depth_exceeded() -> bool {
  CALL_DEPTH_EXCEEDED.with(|exceeded| exceeded.get())
}

/// Evaluates an invocation expression within [MAX_CALL_DEPTH]; the result is never a non-finite number.
pub(crate) fn guarded(evaluate: impl FnOnce() -> Value) -> Value {
  if CALL_DEPTH_EXCEEDED.with(|exceeded| exceeded.get()) {
    return value_null!("function call depth limit ({}) exceeded", MAX_CALL_DEPTH);
  }
  let depth = CALL_DEPTH.with(|depth| depth.get());
  if depth >= MAX_CALL_DEPTH {
    CALL_DEPTH_EXCEEDED.with(|exceeded| exceeded.set(true));
    return value_null!("function call depth limit ({}) exceeded", MAX_CALL_DEPTH);
  }
  CALL_DEPTH.with(|d| d.set(depth + 1));
  let value = finite(evaluate());
  CALL_DEPTH.with(|d| d.set(depth));
  value
}

/// Arguments of an invocation (DMN 1.6, §10.3.2.13.5).
pub(crate) enum Arguments {
  /// All parameters are supplied, in their order.
  Positional(Vec<Value>),
  /// Parameters supplied by name, in the order written; unsupplied parameters are bound to null.
  Named(Vec<(Name, Value)>),
}

/// Applies a function to arguments (DMN 1.6, Table 64): null when `function` is not a function, or when
/// the arguments do not match or do not conform to its parameters; the body is then not evaluated.
pub(crate) fn invoke(scope: &FeelScope, function: Value, arguments: Arguments, dialect: Dialect) -> Value {
  match function {
    Value::BuiltInFunction(bif) => apply_built_in(bif, arguments, dialect),
    Value::FunctionDefinition(function) => {
      let values = match bind(&function.parameters, arguments) {
        Ok(values) => values,
        Err(reason) => return value_null!(reason),
      };
      let result = if function.external {
        apply_external(scope, &function.body, &values)
      } else {
        apply_body(scope, &function, values)
      };
      result.into_coerced(&function.result_type)
    }
    other => value_null!("expected built-in function name or function definition, actual is {}", other),
  }
}

/// The argument values in the order of the parameters, each converted to its parameter's type
/// (§10.3.2.9.4, invocation context), or why the arguments do not match the parameters.
fn bind(parameters: &[(Name, FeelType)], arguments: Arguments) -> Result<Vec<Value>, String> {
  let values = match arguments {
    Arguments::Positional(values) => {
      if values.len() != parameters.len() {
        return Err("invalid number of arguments".to_string());
      }
      values
    }
    Arguments::Named(arguments) => {
      let mut values = vec![value_null!(); parameters.len()];
      for (name, value) in arguments {
        let Some(index) = parameters.iter().position(|(parameter_name, _)| *parameter_name == name) else {
          return Err(format!("function has no parameter named '{name}'"));
        };
        values[index] = value;
      }
      values
    }
  };
  values
    .into_iter()
    .zip(parameters)
    .map(|(value, (name, parameter_type))| {
      value
        .try_into_coerced(parameter_type)
        .ok_or_else(|| format!("argument for parameter '{name}' does not conform to type {parameter_type}"))
    })
    .collect()
}

/// Evaluates the body of a FEEL function with its parameters bound to the argument values, in the
/// scope of the invocation extended with the function's closure.
fn apply_body(scope: &FeelScope, function: &FunctionDefinition, values: Vec<Value>) -> Value {
  let mut params_ctx = FeelContext::default();
  for ((name, _), value) in function.parameters.iter().zip(values) {
    params_ctx.set_entry(name, value);
  }
  // parameters innermost: a decision service body reads its inputs from the top (decision_service.rs)
  let depth = scope.depth();
  scope.push(function.closure_ctx.clone());
  scope.push(params_ctx);
  let result = closed_over(scope, function.body.evaluate(scope));
  scope.truncate(depth);
  result
}

/// A function returned by a body closes over the names it uses as they are where the body ends,
/// including entries the body added (kept from dsntk).
fn closed_over(scope: &FeelScope, result: Value) -> Value {
  match result {
    Value::FunctionDefinition(mut function) => {
      let closed: Vec<(QualifiedName, Value)> = function
        .closure
        .iter()
        .filter_map(|name| scope.search_entry(name).map(|value| (name.clone(), value)))
        .collect();
      if !closed.is_empty() {
        // a copy of the definition when it is shared (the function value has value semantics)
        let function = Arc::make_mut(&mut function);
        for (name, value) in closed {
          function.closure_ctx.create_entry(&name, value);
        }
      }
      Value::FunctionDefinition(function)
    }
    other => other,
  }
}

/// Calls the Java method or the PMML model an external function is mapped to.
fn apply_external(scope: &FeelScope, body: &FunctionBody, arguments: &[Value]) -> Value {
  match &body.evaluate(scope) {
    Value::ExternalJavaFunction(class_name, method_signature) => evaluate_external_java_function(class_name, method_signature, arguments),
    Value::ExternalPmmlFunction(document, model_name) => evaluate_external_pmml_function(document, model_name, arguments),
    other => value_null!("expected JAVA or PMML mapping, actual value is {}", other),
  }
}

/// Applies a built-in function; the dialect selects the FEEL or B-FEEL semantics.
fn apply_built_in(bif: Bif, arguments: Arguments, dialect: Dialect) -> Value {
  match arguments {
    Arguments::Positional(values) => match dialect {
      Dialect::BFeel => bfeel::evaluate_positional_bif(bif, values),
      Dialect::Feel => bifs::positional::evaluate_bif(bif, &values),
    },
    Arguments::Named(arguments) => {
      // positions tell `before(point, range)` from `before(range, point)` (bifs/named.rs)
      let named = Value::NamedParameters(arguments.into_iter().zip(1_usize..).map(|((name, value), position)| (name, (value, position))).collect());
      match dialect {
        Dialect::BFeel => bfeel::evaluate_named_bif(bif, &named),
        Dialect::Feel => bifs::named::evaluate_bif(bif, &named),
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use rheon_feel::closure::Closure;
  use rheon_feel::{Evaluator, scope};
  use std::sync::Arc;

  #[test]
  fn test_unimplemented_external_function_kind() {
    let evaluator = Box::new(move |_: &FeelScope| Value::Boolean(false)) as Evaluator;
    let body = FunctionBody::External(Arc::new(evaluator));
    let function = FunctionDefinition::new(vec![], body, true, Closure::default(), FeelContext::default(), FeelType::Boolean).into();
    let result = invoke(&scope!(), function, Arguments::Positional(vec![]), Dialect::Feel);
    assert_eq!("null(expected JAVA or PMML mapping, actual value is false)", result.to_string())
  }
}
