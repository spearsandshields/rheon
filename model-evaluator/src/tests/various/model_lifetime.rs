//! # The lifetime of a model evaluator

use super::super::*;
use crate::model_definitions::DefKey;
use rheon_feel_evaluator::{Dialect, evaluate_invocation};
use std::sync::Arc;

#[test]
fn a_model_with_decision_services_is_freed_when_dropped() {
  let model_evaluator = build_model_evaluator(dsntk_examples::DMN_3_0085);
  let weak = Arc::downgrade(&model_evaluator);
  drop(model_evaluator);
  assert!(weak.upgrade().is_none());
}

#[test]
fn a_decision_service_value_outliving_its_model_is_null() {
  let model_evaluator = build_model_evaluator(dsntk_examples::DMN_3_0085);
  let mut output = FeelContext::default();
  let def_key = DefKey::new("https://dsntk.io", "_decisionService_001");
  let name = model_evaluator
    .decision_service_evaluator()
    .evaluate_fd(&def_key, &FeelContext::default(), &mut output)
    .unwrap();
  let function = output.get_entry(&name).unwrap().clone();
  let scope = FeelScope::default();
  assert_eq!(r#""foo""#, evaluate_invocation(&scope, function.clone(), vec![], Dialect::default()).to_string());
  drop(model_evaluator);
  assert_eq!(
    "null(decision service decisionService_001 cannot be called after its model was unloaded)",
    evaluate_invocation(&scope, function, vec![], Dialect::default()).to_string()
  );
}
