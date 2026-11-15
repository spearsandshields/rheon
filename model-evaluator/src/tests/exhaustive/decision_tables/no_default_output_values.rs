use super::*;

from_examples!(EXHAUSTIVE_DECISION_TABLES_NO_DEFAULT_OUTPUT_VALUES);

static_context!(CTX_1, r#"{ Customer type: "Business", Order size: 9 }"#);
static_context!(CTX_2, r#"{ Customer type: "Business", Order size: 10 }"#);
static_context!(CTX_3, r#"{ Customer type: "Private", Order size: 100 }"#);
static_context!(CTX_4, r#"{ Customer type: "Private", Order size: 101 }"#);

fn eq(invocable_name: &str, input: &FeelContext, expected: &str) {
  assert_decision(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, invocable_name, input, expected);
}

#[test]
fn _0001() {
  eq("decision_0001", &CTX_1, r#"{Discount: 0.1, Priority: "Normal"}"#);
}

#[test]
fn _0002() {
  eq("decision_0001", &CTX_2, r#"{Discount: 0.15, Priority: "High"}"#);
}

#[test]
fn _0003() {
  eq("decision_0001", &CTX_3, r#"{Discount: 0.05, Priority: "Low"}"#);
}

#[test]
fn _0004() {
  // No rule matches, so null value is returned.
  eq("decision_0001", &CTX_4, r#"null(no rules matched, no output value defined)"#);
}

/// DMN 1.6 §10.3.2.10: when no rule matches, "the result is given by the default output value,
/// or null if no default output value is specified": a null the specification defines, so no
/// error. The null keeps its reason as a note, and strict mode does not stop at it.
#[test]
fn no_matching_rule_is_a_null_with_a_note() {
  use rheon_feel::diagnostics::ErrorMode;
  for error_mode in [ErrorMode::Lenient, ErrorMode::Strict] {
    let (value, diagnostics) = MODEL_EVALUATOR
      .evaluate_invocable_with_mode(&MODEL_NAMESPACE, &MODEL_NAME, "decision_0001", &CTX_4, error_mode)
      .into_result()
      .unwrap();
    assert_eq!(value.to_string(), "null(no rules matched, no output value defined)", "{error_mode:?}");
    assert_eq!(diagnostics.count, 0, "{error_mode:?} {diagnostics:?}");
  }
}
