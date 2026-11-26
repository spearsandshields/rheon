use super::*;

from_examples!(DMN_2_0117);

#[test]
fn _0001() {
  let ctx = context(r#"{Age: 19,RiskCategory: "Low",isAffordable: true}"#);
  assert_decision(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, "Approval", &ctx, r#"{Status: "Approved", Rate: "Best"}"#);
}

#[test]
fn _0002() {
  let ctx = context(r#"{Age: 17,RiskCategory: "High",isAffordable: true}"#);
  assert_decision(
    &MODEL_EVALUATOR,
    &MODEL_NAMESPACE,
    &MODEL_NAME,
    "Approval",
    &ctx,
    r#"{Status: "Declined", Rate: "Standard"}"#,
  );
}

#[test]
fn _0003() {
  let ctx = context(r#"{Age: 19,RiskCategory: "Medium",isAffordable: true}"#);
  assert_decision(
    &MODEL_EVALUATOR,
    &MODEL_NAMESPACE,
    &MODEL_NAME,
    "Approval",
    &ctx,
    r#"{Status: "Approved", Rate: "Standard"}"#,
  );
}
