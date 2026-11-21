//! # Function invocation
//!
//! The one place where arguments are bound to the parameters of a function and the function is applied (DMN 1.6, §10.3.2.13, Table 64):
//! FEEL and boxed invocations, invocables invoked with input data, and the functions called by `sort` and `list replace`.

use crate::bfeel::{self, Dialect};
use crate::bifs;
use crate::evaluator_java::evaluate_external_java_function;
use crate::evaluator_pmml::evaluate_external_pmml_function;
use crate::kernels::finite;
use rheon_feel::bif::Bif;
use rheon_feel::context::FeelContext;
use rheon_feel::diagnostics;
use rheon_feel::message::arg;
use rheon_feel::values::Value;
use rheon_feel::{FeelScope, FeelType, FunctionBody, FunctionDefinition, Name, QualifiedName, value_null};
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::sync::Arc;

/// Maximum number of nested function invocations during one evaluation.
///
/// Every nested invocation (user-defined recursion such as `f(n - 1)`, or built-in calls nested
/// in each other) uses host stack. Without a limit, deep recursion overflows the stack, which on
/// wasm32 is a trap that kills the engine instance, at a depth that depends on the host (browser
/// worker, Node main thread, ...). With this limit the evaluation stops deterministically at the
/// same depth everywhere, with [Abort::CallDepth](rheon_feel::diagnostics::Abort::CallDepth).
pub const MAX_CALL_DEPTH: usize = 200;

/// Evaluates an invocation expression within [MAX_CALL_DEPTH]; the result is never a non-finite
/// number, and never a value nested more deeply than values may nest ([diagnostics::bound_depth]).
///
/// A call beyond the limit stops the evaluation, and no further call is made in it. A refused
/// call's null is never observed: the abort is the outcome of the evaluation.
pub(crate) fn guarded(evaluate: impl FnOnce() -> Value) -> Value {
  match diagnostics::enter_call(MAX_CALL_DEPTH) {
    None => Value::Null(None),
    Some(call) => call.finish(diagnostics::bound_depth(finite(evaluate()))),
  }
}

/// Arguments of an invocation (DMN 1.6, §10.3.2.13.5).
pub(crate) enum Arguments {
  /// All parameters are supplied, in their order.
  Positional(Vec<Value>),
  /// Parameters supplied by name, in the order written; unsupplied parameters are bound to null.
  Named(Vec<(Name, Value)>),
}

/// A counted call ([guarded]) of a function value: the invocation of a function by a built-in
/// that takes one (`sort`, `list replace`) or by a caller outside the evaluator. The invocation
/// nodes enter the guard themselves, before they evaluate the function and the arguments, and
/// call [invoke].
pub(crate) fn call(scope: &FeelScope, function: Value, arguments: Arguments, dialect: Dialect) -> Value {
  guarded(|| invoke(scope, function, arguments, dialect))
}

/// [call] of a function definition with the positional arguments `values`, for a built-in that
/// calls a function for the items of a list: the same binding, body and result, without a list
/// of the arguments and without a copy of the function value for each call.
pub(crate) fn call_with<const N: usize>(scope: &FeelScope, function: &Arc<FunctionDefinition>, mut values: [Value; N], dialect: Dialect) -> Value {
  let code = if function.external { None } else { crate::flat::code_of(&function.body) };
  let Some((program, proto)) = code else {
    return call(scope, Value::FunctionDefinition(Arc::clone(function)), Arguments::Positional(values.into()), dialect);
  };
  guarded(|| {
    let parameters = function.parameters();
    match arity(parameters, N).and_then(|()| coerce_arguments(parameters, &mut values)) {
      Ok(()) => crate::flat::run_function(scope, function, program, proto, values).into_coerced(function.result_type()),
      Err(reason) => value_null!(reason),
    }
  })
}

/// Applies a function to arguments (DMN 1.6, Table 64): null when `function` is not a function, or when
/// the arguments do not match or do not conform to its parameters; the body is then not evaluated.
pub(crate) fn invoke(scope: &FeelScope, function: Value, arguments: Arguments, dialect: Dialect) -> Value {
  match function {
    Value::BuiltInFunction(bif) => apply_built_in(bif, arguments, dialect),
    Value::FunctionDefinition(function) => {
      let values = match bind(function.parameters(), arguments) {
        Ok(values) => values,
        Err(reason) => return value_null!(reason),
      };
      let result = if function.external {
        apply_external(scope, &function.body, &values)
      } else if let Some((program, proto)) = crate::flat::code_of(&function.body) {
        crate::flat::run_function(scope, &function, program, proto, values)
      } else {
        apply_body(scope, &function, values)
      };
      result.into_coerced(function.result_type())
    }
    other => value_null!("expected built-in function name or function definition, actual is {}", arg(&other)),
  }
}

/// The argument values in the order of the parameters, each converted to its parameter's type
/// (§10.3.2.9.4, invocation context), or why the arguments do not match the parameters.
/// Not inlined: its frame is not on the stack while the body runs, so a nested call costs no
/// more stack than its caller's own frame.
#[inline(never)]
fn bind(parameters: &[(Name, FeelType)], arguments: Arguments) -> Result<Vec<Value>, String> {
  let mut values = match arguments {
    Arguments::Positional(values) => {
      arity(parameters, values.len())?;
      values
    }
    Arguments::Named(arguments) => {
      let mut values = vec![];
      in_order(parameters, arguments, &mut values)?;
      values
    }
  };
  coerce_arguments(parameters, &mut values)?;
  Ok(values)
}

/// Whether `count` positional arguments match `parameters`, or why they do not.
pub(crate) fn arity(parameters: &[(Name, FeelType)], count: usize) -> Result<(), String> {
  if count == parameters.len() {
    Ok(())
  } else {
    Err("invalid number of arguments".to_string())
  }
}

/// Why an invocation that gives the parameter `name` twice is refused.
fn given_twice(name: &Name) -> String {
  format!("expected each parameter name once, found the name '{}' twice", arg(name))
}

/// The values of the named `arguments` in `values`, in the order of `parameters` (null for a
/// parameter that no argument names; an argument of a name that two parameters have goes to the
/// first), or why they do not match the parameters: a name that no parameter has, or a name
/// that is given twice.
pub(crate) fn in_order<N: std::borrow::Borrow<Name>>(
  parameters: &[(Name, FeelType)],
  arguments: impl IntoIterator<Item = (N, Value)>,
  values: &mut Vec<Value>,
) -> Result<(), String> {
  values.clear();
  values.resize(parameters.len(), value_null!());
  // the parameters that an argument has named: a bit of a word each, and beyond its bits a list
  let mut few = 0_u64;
  let mut many = if parameters.len() > u64::BITS as usize { vec![false; parameters.len()] } else { vec![] };
  for (name, value) in arguments {
    let name = name.borrow();
    let Some(index) = parameters.iter().position(|(parameter_name, _)| parameter_name == name) else {
      return Err(format!("function has no parameter named '{}'", arg(name)));
    };
    let twice = match many.get_mut(index) {
      Some(given) => std::mem::replace(given, true),
      None => {
        let bit = 1_u64 << index;
        let given = few & bit != 0;
        few |= bit;
        given
      }
    };
    if twice {
      return Err(given_twice(name));
    }
    values[index] = value;
  }
  Ok(())
}

/// Converts the argument values in `values`, one per parameter and in the order of the
/// parameters, to the types of their parameters in place, up to the first that does not conform,
/// which gives why.
pub(crate) fn coerce_arguments(parameters: &[(Name, FeelType)], values: &mut [Value]) -> Result<(), String> {
  for (value, (name, parameter_type)) in values.iter_mut().zip(parameters) {
    // every value conforms to `Any`, the type of a parameter that declares none
    if matches!(parameter_type, FeelType::Any) {
      continue;
    }
    let argument = std::mem::replace(value, Value::Null(None));
    *value = argument
      .try_into_coerced(parameter_type)
      .ok_or_else(|| format!("argument for parameter '{}' does not conform to type {}", arg(name), arg(parameter_type)))?;
  }
  Ok(())
}

/// Evaluates the body of a FEEL function with its parameters bound to the argument values, in the
/// scope of the invocation extended with the function's closure.
fn apply_body(scope: &FeelScope, function: &FunctionDefinition, values: Vec<Value>) -> Value {
  let mut params_ctx = FeelContext::default();
  for ((name, _), value) in function.parameters().iter().zip(values) {
    params_ctx.set_entry(name, value);
  }
  // parameters innermost: a decision service body reads its inputs from the top (decision_service.rs)
  let depth = scope.depth();
  scope.push_shared(Arc::clone(&function.closure_ctx));
  scope.push(params_ctx);
  let result = closed_over(|name| scope.search_entry(name), function.body.evaluate(scope));
  scope.truncate(depth);
  result
}

/// A function returned by a body closes over the names it uses as they are where the body ends,
/// including entries the body added (kept from dsntk): `search` finds the value of a name there.
/// Not inlined, for the reason of [bind]: it runs after the body, whose nested calls would
/// otherwise carry its frame.
#[inline(never)]
pub(crate) fn closed_over(search: impl Fn(&QualifiedName) -> Option<Value>, result: Value) -> Value {
  match result {
    Value::FunctionDefinition(mut function) => {
      let closed: Vec<(QualifiedName, Value)> = function.closure.iter().filter_map(|name| search(name).map(|value| (name.clone(), value))).collect();
      if !closed.is_empty() {
        // a copy of the definition and of its closure context when they are shared (the function
        // value has value semantics)
        let closure_ctx = Arc::make_mut(&mut Arc::make_mut(&mut function).closure_ctx);
        for (name, value) in closed {
          closure_ctx.create_entry(&name, value);
        }
      }
      Value::FunctionDefinition(function)
    }
    other => other,
  }
}

/// Applies an external function: its Java, PMML or ONNX mapping, which rheon does not execute, so the
/// result is null with a reason.
fn apply_external(scope: &FeelScope, body: &FunctionBody, arguments: &[Value]) -> Value {
  match body.evaluate(scope) {
    Value::ExternalJavaFunction(class_name, method_signature) => evaluate_external_java_function(&class_name, &method_signature, arguments),
    Value::ExternalPmmlFunction(document, model_name) => evaluate_external_pmml_function(&document, &model_name, arguments),
    null @ Value::Null(_) => null,
    other => value_null!("expected a Java, ONNX or PMML mapping, actual value is {}", arg(&other)),
  }
}

/// Applies a built-in function in FEEL to positional arguments, which it reads where they are.
pub(crate) fn apply_feel_built_in(bif: Bif, values: &[Value]) -> Value {
  bifs::positional::evaluate_bif(bif, values)
}

/// Applies a built-in function; the dialect selects the FEEL or B-FEEL semantics.
fn apply_built_in(bif: Bif, arguments: Arguments, dialect: Dialect) -> Value {
  match arguments {
    Arguments::Positional(values) => match dialect {
      Dialect::BFeel => bfeel::evaluate_positional_bif(bif, values),
      Dialect::Feel => apply_feel_built_in(bif, &values),
    },
    Arguments::Named(arguments) => {
      // positions tell `before(point, range)` from `before(range, point)` (bifs/named.rs)
      let mut named = BTreeMap::new();
      for ((name, value), position) in arguments.into_iter().zip(1_usize..) {
        match named.entry(name) {
          Entry::Vacant(entry) => {
            entry.insert((value, position));
          }
          Entry::Occupied(entry) => {
            let twice = value_null!(given_twice(entry.key()));
            return match dialect {
              Dialect::BFeel => bfeel::default_if_null(&bif, twice),
              Dialect::Feel => twice,
            };
          }
        }
      }
      let named = Value::NamedParameters(named);
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

  #[test]
  fn test_unimplemented_external_function_kind() {
    let evaluator = Box::new(move |_: &FeelScope| Value::Boolean(false)) as Evaluator;
    let body = FunctionBody::External(Arc::new(evaluator));
    let function = FunctionDefinition::new(vec![], body, true, Closure::default(), FeelContext::default(), FeelType::Boolean).into();
    let result = invoke(&scope!(), function, Arguments::Positional(vec![]), Dialect::Feel);
    assert_eq!("null(expected a Java, ONNX or PMML mapping, actual value is false)", result.to_string())
  }
}
