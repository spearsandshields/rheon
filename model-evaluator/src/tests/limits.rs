//! # Models at the bound of the model parser
//!
//! These tests run on the default test thread of the debug build, the smallest native stack a
//! host gives an evaluation, so a model within the bounds is shown to build and evaluate there.

use super::*;

const NAMESPACE: &str = "https://dsntk.io/limits";
const NAME: &str = "limits";

/// A model with one decision whose boxed contexts are nested in each other, so that its deepest
/// element (the text of the innermost literal expression) is at `level`; the root element is level 1.
fn nested_boxed_contexts(level: usize) -> String {
  assert!(level >= 6 && level.is_multiple_of(2));
  let contexts = (level - 4) / 2;
  let mut xml = format!(
    r#"<definitions namespace="{NAMESPACE}" name="{NAME}" xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/"><decision name="Nested" id="_nested"><variable name="Nested"/>"#
  );
  xml.push_str(&"<context><contextEntry>".repeat(contexts));
  xml.push_str("<literalExpression><text>1 + 1</text></literalExpression>");
  xml.push_str(&"</contextEntry></context>".repeat(contexts));
  xml.push_str("</decision></definitions>");
  xml
}

/// A model with one decision whose boxed lists are nested in each other, one element level each,
/// so that its deepest element (the text of the innermost literal expression) is at `level`.
fn nested_boxed_lists(level: usize) -> String {
  let lists = level - 4;
  let mut xml = format!(
    r#"<definitions namespace="{NAMESPACE}" name="{NAME}" xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/"><decision name="Nested" id="_nested"><variable name="Nested"/>"#
  );
  xml.push_str(&"<list>".repeat(lists));
  xml.push_str("<literalExpression><text>1 + 1</text></literalExpression>");
  xml.push_str(&"</list>".repeat(lists));
  xml.push_str("</decision></definitions>");
  xml
}

#[test]
fn boxed_lists_nested_to_the_element_bound_build_and_evaluate() {
  let model_evaluator = build_model_evaluator(&nested_boxed_lists(100));
  let expected = format!("{}2{}", "[".repeat(96), "]".repeat(96));
  assert_decision(&model_evaluator, NAMESPACE, NAME, "Nested", &FeelContext::default(), &expected);
}

#[test]
fn boxed_contexts_nested_to_the_element_bound_build_and_evaluate() {
  let model_evaluator = build_model_evaluator(&nested_boxed_contexts(100));
  assert_decision(&model_evaluator, NAMESPACE, NAME, "Nested", &FeelContext::default(), "2");
}

#[test]
fn boxed_contexts_nested_far_past_the_element_bound_are_rejected() {
  assert_eq!(
    "<ModelParserError> elements are nested more than 100 levels deep",
    rheon_model::parse(&nested_boxed_contexts(100_000)).unwrap_err().to_string()
  );
}
