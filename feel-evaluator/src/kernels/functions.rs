//! Function values: the value of a function definition and of an external function's mapping
//! (DMN 1.6 §10.3.2.13).

use rheon_feel::closure::Closure;
use rheon_feel::context::FeelContext;
use rheon_feel::message::arg;
use rheon_feel::values::{TypeOf, Value};
use rheon_feel::{Evaluator, FeelScope, FeelType, FunctionBody, FunctionDefinition, QualifiedName, diagnostics, value_null};
use std::sync::Arc;

/// The function that a definition gives from its evaluated formal parameters and body. Its closure
/// context holds the value of each name of `closure` that `search` finds, in the order of `closure`;
/// the function is nested no deeper than values may nest.
pub(crate) fn function_definition(parameters: Value, body: Value, closure: &Closure, search: impl Fn(&QualifiedName) -> Option<Value>) -> Value {
  match parameters {
    Value::FormalParameters(parameters) => {
      if let Value::FunctionBody(body, external) = body {
        // evaluate closure context
        let mut closure_ctx = FeelContext::default();
        for closure_name in closure.iter() {
          if let Some(closure_value) = search(closure_name) {
            closure_ctx.create_entry(closure_name, closure_value);
          }
        }
        //TODO Check if `FeelType::Any` is always ok for function result type in function definition.
        diagnostics::bound_depth(FunctionDefinition::new(parameters, body, external, closure.clone(), closure_ctx, FeelType::Any).into())
      } else {
        value_null!("invalid body in function definition")
      }
    }
    null @ Value::Null(_) => null,
    other => value_null!("expected formal parameters, actual value type is {}", arg(&TypeOf(&other))),
  }
}

/// The body of an external function from its evaluated mapping: a context with a `java` entry
/// (`class` and `method signature`) or a `pmml` entry (`document` and `model`).
pub(crate) fn external_body(mapping_value: Value) -> Value {
  match mapping_value {
    Value::Context(mapping_information) => {
      if let Some(Value::Context(java_mapping)) = mapping_information.get_entry(&"java".into()) {
        return if let Some(Value::String(class_name)) = java_mapping.get_entry(&"class".into()) {
          if let Some(Value::String(method_signature)) = java_mapping.get_entry(&"method signature".into()) {
            let java_class_name = class_name.as_str().to_owned();
            let java_method_signature = method_signature.as_str().to_owned();
            let java_evaluator = Box::new(move |_: &FeelScope| Value::ExternalJavaFunction(java_class_name.clone(), java_method_signature.clone())) as Evaluator;
            let lhe = Arc::new(java_evaluator);
            Value::FunctionBody(FunctionBody::External(lhe), true)
          } else {
            value_null!("invalid Java function mapping, no method signature entry in context {}", arg(&java_mapping))
          }
        } else {
          value_null!("invalid Java function mapping, no class name entry in context {}", arg(&java_mapping))
        };
      }
      if let Some(Value::Context(pmml_mapping)) = mapping_information.get_entry(&"pmml".into()) {
        return if let Some(Value::String(document)) = pmml_mapping.get_entry(&"document".into()) {
          if let Some(Value::String(model_name)) = pmml_mapping.get_entry(&"model".into()) {
            let pmml_document = document.as_str().to_owned();
            let pmml_model_name = model_name.as_str().to_owned();
            let pmml_evaluator = Box::new(move |_: &FeelScope| Value::ExternalPmmlFunction(pmml_document.clone(), pmml_model_name.clone())) as Evaluator;
            let lhe = Arc::new(pmml_evaluator);
            Value::FunctionBody(FunctionBody::External(lhe), true)
          } else {
            value_null!("invalid PMML function mapping, no model name entry in context {}", arg(&pmml_mapping))
          }
        } else {
          value_null!("invalid PMML function mapping, no document entry in context {}", arg(&pmml_mapping))
        };
      }
      value_null!(
        "invalid external function mapping, expected 'java' or 'pmml' entry in context {}",
        arg(&mapping_information)
      )
    }
    other => value_null!("invalid external function mapping, expected context, actual value is {}", arg(&other)),
  }
}
