use super::*;

// TCK 1157-implicit-conversions (DMN 1.6 §10.3.2.9.4): conversions of decision, BKM, decision
// service, context entry and invocation binding values to their declared types.
from_examples!(DMN_3_1157);

static_context!(CTX, "{}");

#[test]
fn _0001() {
  assert_decision(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, "To Singleton List", &CTX, r#"["abc"]"#);
}

#[test]
fn _0002() {
  assert_decision(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, "From Singleton List", &CTX, r#""abc""#);
}

#[test]
fn _0003() {
  assert_decision(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, "From Singleton List Error", &CTX, r#"null(after coercion)"#);
}

#[test]
fn _0004() {
  assert_decision(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, "From Date To Date and Time", &CTX, r#"2000-01-02T00:00:00Z"#);
}

#[test]
fn _0005() {
  assert_business_knowledge_model(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, "To Singleton List BKM", &CTX, r#"[1]"#);
}

#[test]
fn _0006() {
  assert_business_knowledge_model(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, "From Singleton List BKM", &CTX, r#"1"#);
}

#[test]
fn _0007() {
  assert_business_knowledge_model(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, "From Date To Date and Time BKM", &CTX, r#"2000-01-02T00:00:00Z"#);
}

#[test]
fn _0008() {
  assert_decision_service(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, "To Singleton List DS", &CTX, r#"[2000-01-02]"#);
}

#[test]
fn _0009() {
  assert_decision_service(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, "From Singleton List DS", &CTX, r#"2000-01-02"#);
}

#[test]
fn _0010() {
  assert_decision_service(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, "From Date to Date and Time DS", &CTX, r#"2000-01-02T00:00:00Z"#);
}

#[test]
fn _0011() {
  assert_decision(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, "Implicit Conversions CE", &CTX, r#"[[1], 2, 2000-12-01T00:00:00Z]"#);
}

#[test]
fn _0012() {
  assert_decision(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, "Implicit Conversions FUNCT", &CTX, r#"[[1], 2, 2000-02-01T00:00:00Z]"#);
}
