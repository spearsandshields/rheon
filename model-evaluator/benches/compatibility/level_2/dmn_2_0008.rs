use super::*;

from_examples!(DMN_2_0008);

#[bench]
fn _0001(b: &mut Bencher) {
  let ctx = context(r#"{loan: {principal: 600000,rate: 0.0375,termMonths: 360}}"#);
  let invocable_name = "payment";
  assert_decision(
    &MODEL_EVALUATOR,
    &MODEL_NAMESPACE,
    &MODEL_NAME,
    invocable_name,
    &ctx,
    r#"2778.693549432766768088520383236299"#,
  );
  iter!(b, MODEL_EVALUATOR.evaluate_invocable(&MODEL_NAMESPACE, &MODEL_NAME, invocable_name, &ctx));
}

#[bench]
fn _0002(b: &mut Bencher) {
  let ctx = context(r#"{loan: {principal: 30000,rate: 0.0475,termMonths: 60}}"#);
  let invocable_name = "payment";
  assert_decision(
    &MODEL_EVALUATOR,
    &MODEL_NAMESPACE,
    &MODEL_NAME,
    invocable_name,
    &ctx,
    r#"562.7073593732659271562143285576524"#,
  );
  iter!(b, MODEL_EVALUATOR.evaluate_invocable(&MODEL_NAMESPACE, &MODEL_NAME, invocable_name, &ctx));
}

#[bench]
fn _0003(b: &mut Bencher) {
  let ctx = context(r#"{loan: {principal: 600000,rate: 0.0399,termMonths: 360}}"#);
  let invocable_name = "payment";
  assert_decision(
    &MODEL_EVALUATOR,
    &MODEL_NAMESPACE,
    &MODEL_NAME,
    invocable_name,
    &ctx,
    r#"2861.033777003901636716262779605767"#,
  );
  iter!(b, MODEL_EVALUATOR.evaluate_invocable(&MODEL_NAMESPACE, &MODEL_NAME, invocable_name, &ctx));
}
