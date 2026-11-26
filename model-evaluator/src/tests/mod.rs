use crate::model_evaluator::ModelEvaluator;
use rheon_feel::FeelScope;
use rheon_feel::context::FeelContext;
use rheon_feel::values::Value;
use rheon_feel_evaluator::Outcome;
use rheon_model::DmnElement;
use std::sync::{Arc, LazyLock};

mod build_safety;
mod compatibility;
mod exhaustive;
mod host_calls;
mod input_data;
mod item_definition_graph;
mod limits;
mod send_sync;
mod simulation;
mod various;

macro_rules! from_examples {
  ($model_name:tt) => {
    model_evaluator_from_examples!($model_name);
    model_namespace_from_examples!($model_name);
    model_name_from_examples!($model_name);
  };
}

macro_rules! model_evaluator_from_examples {
  ($model_name:tt) => {
    static MODEL_EVALUATOR: LazyLock<Arc<ModelEvaluator>> = LazyLock::new(|| build_model_evaluator(dsntk_examples::$model_name));
  };
}

macro_rules! model_namespace_from_examples {
  ($model_name:tt) => {
    static MODEL_NAMESPACE: LazyLock<String> = LazyLock::new(|| build_model_namespace(dsntk_examples::$model_name));
  };
}

macro_rules! model_name_from_examples {
  ($model_name:tt) => {
    static MODEL_NAME: LazyLock<String> = LazyLock::new(|| build_model_name(dsntk_examples::$model_name));
  };
}

macro_rules! model_evaluator {
  ($model_name:tt) => {
    static MODEL_EVALUATOR: LazyLock<Arc<ModelEvaluator>> = LazyLock::new(|| build_model_evaluator($model_name));
  };
}

macro_rules! static_context {
  ($name:tt, $content:tt) => {
    static $name: LazyLock<FeelContext> = LazyLock::new(|| context($content));
  };
}

use from_examples;
use model_evaluator;
use model_evaluator_from_examples;
use model_name_from_examples;
use model_namespace_from_examples;
use rheon_model::NamedElement;
use static_context;

/// Utility function that creates a `FEEL` context from specified input expression.
pub fn context(input: &str) -> FeelContext {
  let scope = FeelScope::default();
  match rheon_feel_parser::parse_context(&scope, input) {
    Ok(node) => {
      let evaluator = rheon_feel_evaluator::prepare(&node);
      match evaluator(&scope) {
        Value::Context(ctx) => std::sync::Arc::unwrap_or_clone(ctx),
        other => panic!("ERROR: expected context value, actual value is: {}", other),
      }
    }
    Err(reason) => panic!("ERROR: parsing context failed with reason: {}", reason),
  }
}

/// Utility function that builds a model evaluator from single XML model definitions.
fn build_model_evaluator(model_content: &str) -> Arc<ModelEvaluator> {
  build_model_evaluators(&[model_content])
}

/// Utility function that builds a model evaluator from multiple XML model definitions.
fn build_model_evaluators(model_content: &[&str]) -> Arc<ModelEvaluator> {
  let mut definitions = vec![];
  for content in model_content {
    definitions.push(rheon_model::parse(content).unwrap());
  }
  ModelEvaluator::new(&definitions).unwrap()
}

/// The text of the outcome of `invocable` with `input_data`.
fn evaluated(model_evaluator: &ModelEvaluator, model_namespace: &str, model_name: &str, invocable: &str, input_data: &FeelContext) -> String {
  outcome_text(model_evaluator.evaluate_invocable(model_namespace, model_name, invocable, input_data))
}

/// Utility function that returns a model namespace from a single DMN model.
fn build_model_namespace(model_content: &str) -> String {
  let definitions = rheon_model::parse(model_content).unwrap();
  definitions.namespace().to_string()
}

/// Utility function that returns a model names from a single DMN model.
fn build_model_name(model_content: &str) -> String {
  let definitions = rheon_model::parse(model_content).unwrap();
  definitions.name().to_string()
}

/// The text of an evaluation's outcome: its value, or `aborted(<reason>)` when it was stopped.
fn outcome_text(outcome: Outcome<Value>) -> String {
  match outcome.into_result() {
    Ok(value) => value.to_string(),
    Err(abort) => format!("aborted({abort})"),
  }
}

/// Utility function that evaluates a [Decision] specified by name and compares the result.
fn assert_decision(model_evaluator: &ModelEvaluator, model_namespace: &str, model_name: &str, invocable: &str, input_data: &FeelContext, expected: &str) {
  let actual = evaluated(model_evaluator, model_namespace, model_name, invocable, input_data);
  assert_eq!(
    expected, actual,
    "Assertion error, actual value of the decision does not match the expected value:\n  expected: {expected}\n    actual: {actual}\n"
  );
}

/// Utility function that evaluates a [BusinessKnowledgeModel] specified by name and compares the result.
fn assert_business_knowledge_model(model_evaluator: &ModelEvaluator, model_namespace: &str, model_name: &str, invocable_name: &str, input_data: &FeelContext, expected: &str) {
  let actual = evaluated(model_evaluator, model_namespace, model_name, invocable_name, input_data);
  assert_eq!(
    expected, actual,
    "Assertion error, actual value of the business knowledge model does not match the expected value:\n  expected: {expected}\n    actual: {actual}\n"
  );
}

/// Utility function that evaluates a [DecisionService] specified by name and compares the result with expected value.
fn assert_decision_service(model_evaluator: &ModelEvaluator, model_namespace: &str, model_name: &str, invocable_name: &str, input_data: &FeelContext, expected: &str) {
  let actual = evaluated(model_evaluator, model_namespace, model_name, invocable_name, input_data);
  assert_eq!(
    expected, actual,
    "Assertion error, actual value of the decision service does not match the expected value:\n  expected: {expected}\n    actual: {actual}\n"
  );
}
