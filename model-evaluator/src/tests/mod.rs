use crate::model_evaluator::ModelEvaluator;
use rheon_feel::FeelScope;
use rheon_feel::context::FeelContext;
use rheon_feel::values::Value;
use rheon_feel_evaluator::Outcome;
use rheon_feel_evaluator::testing::{Backend, take_lowered, take_refusals, with_backend};
use rheon_model::DmnElement;
use std::sync::{Arc, LazyLock, Mutex, Weak};

mod compatibility;
mod exhaustive;
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
  match rheon_feel_parser::parse_context(&scope, input, false) {
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

/// Utility function that builds a model evaluator from multiple XML model definitions; with
/// [twins] it is built by the flat evaluator, and its twin by the evaluator of closures.
fn build_model_evaluators(model_content: &[&str]) -> Arc<ModelEvaluator> {
  let build = || {
    let mut definitions = vec![];
    for content in model_content {
      definitions.push(rheon_model::parse(content).unwrap());
    }
    ModelEvaluator::new(&definitions).unwrap()
  };
  if !twins() {
    return build();
  }
  let model_evaluator = with_backend(Backend::Flat, build);
  note_flat_builds();
  let twin = with_backend(Backend::Old, build);
  take_refusals();
  take_lowered();
  TWINS.lock().unwrap().push((Arc::downgrade(&model_evaluator), twin));
  model_evaluator
}

/// Whether the model tests compare every evaluation with that of a twin built by the evaluator of
/// closures (`RHEON_BACKEND_TWINS` set; the group tier sets it).
fn twins() -> bool {
  std::env::var_os("RHEON_BACKEND_TWINS").is_some()
}

/// With [twins], adds `lines` to the file that `RHEON_BACKEND_COVERAGE` names, when it is set: a
/// line for each expression the flat evaluator lowered or refused and for each evaluation, with or
/// without a twin. The group tier counts the lines of all model tests.
fn note_coverage(lines: &str) {
  if let Some(path) = std::env::var_os("RHEON_BACKEND_COVERAGE")
    && !lines.is_empty()
  {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&path).expect("RHEON_BACKEND_COVERAGE");
    file.write_all(lines.as_bytes()).expect("RHEON_BACKEND_COVERAGE");
  }
}

/// Notes the expressions that the flat evaluator lowered and refused since the last note.
fn note_flat_builds() {
  let mut lines = "lowered\n".repeat(take_lowered());
  for refusal in take_refusals() {
    lines.push_str(&format!("refused {} at {}\n", refusal.kind, refusal.node));
  }
  note_coverage(&lines);
}

/// The models built with [twins] and their twins. A model is held weakly, so that it is dropped as
/// without a twin, while its allocation stays and no other model is built at its address.
static TWINS: Mutex<Vec<(Weak<ModelEvaluator>, Arc<ModelEvaluator>)>> = Mutex::new(Vec::new());

/// With [twins], a model that is built has a twin, which [evaluated] finds.
#[test]
fn a_model_has_a_twin() {
  if !twins() {
    return;
  }
  let model_evaluator = build_model_evaluator(dsntk_examples::DMN_2_0001);
  let found = TWINS.lock().unwrap().iter().any(|(model, _)| std::ptr::eq(model.as_ptr(), Arc::as_ptr(&model_evaluator)));
  assert!(found);
}

/// The text of the outcome of `invocable` with `input_data`; with [twins], the twin must give the
/// same text and the same diagnostics.
fn evaluated(model_evaluator: &ModelEvaluator, model_namespace: &str, model_name: &str, invocable: &str, input_data: &FeelContext) -> String {
  let evaluate = |model_evaluator: &ModelEvaluator| outcome_text(model_evaluator.evaluate_invocable(model_namespace, model_name, invocable, input_data));
  if !twins() {
    return evaluate(model_evaluator);
  }
  let twin = TWINS
    .lock()
    .unwrap()
    .iter()
    .find(|(model, _)| std::ptr::eq(model.as_ptr(), model_evaluator))
    .map(|(_, twin)| Arc::clone(twin));
  let (actual, diagnostics) = with_backend(Backend::Flat, || rheon_feel::diagnostics::collect(|| evaluate(model_evaluator)));
  note_flat_builds();
  match twin {
    Some(twin) => {
      let (expected, twin_diagnostics) = with_backend(Backend::Old, || rheon_feel::diagnostics::collect(|| evaluate(&twin)));
      take_refusals();
      take_lowered();
      assert_eq!(expected, actual, "the twins of {invocable} differ");
      assert_eq!(twin_diagnostics, diagnostics, "the diagnostics of the twins of {invocable} differ");
      note_coverage("evaluation with a twin\n");
    }
    None => note_coverage("evaluation without a twin\n"),
  }
  actual
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
