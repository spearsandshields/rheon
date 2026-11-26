use super::*;

from_examples!(DMN_3_0031);

static_context!(CTX, r#"{inputA: 10, inputB: 5}"#);

#[test]
fn _0001() {
  assert_decision(
    &MODEL_EVALUATOR,
    &MODEL_NAMESPACE,
    &MODEL_NAME,
    "fn invocation positional parameters",
    &CTX,
    r#"{sumResult: 15, multiplicationResultPositional: 50, divisionResultPositional: 2}"#,
  );
}

#[test]
fn _0002() {
  assert_decision(
    &MODEL_EVALUATOR,
    &MODEL_NAMESPACE,
    &MODEL_NAME,
    "fn invocation named parameters",
    &CTX,
    r#"{subResult: 5, multiplicationResultNamed: 50, subResultMixed: -5, divisionResultNamed: 2}"#,
  );
}

#[test]
fn _0003() {
  assert_decision(
    &MODEL_EVALUATOR,
    &MODEL_NAMESPACE,
    &MODEL_NAME,
    "fn invocation complex parameters",
    &CTX,
    r#"{functionInvocationInParameter: 200, functionInvocationLiteralExpressionInParameter: 500, circumference: 94.24776}"#,
  );
}
