use super::*;

from_examples!(DMN_3_1157);

static_context!(CTX, "{}");

#[bench]
fn _0001(b: &mut Bencher) {
  let invocable = "To Singleton List";
  assert_decision(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, invocable, &CTX, r#"["abc"]"#);
  iter!(b, MODEL_EVALUATOR.evaluate_invocable(&MODEL_NAMESPACE, &MODEL_NAME, invocable, &CTX));
}

#[bench]
fn _0002(b: &mut Bencher) {
  let invocable = "From Singleton List";
  assert_decision(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, invocable, &CTX, r#""abc""#);
  iter!(b, MODEL_EVALUATOR.evaluate_invocable(&MODEL_NAMESPACE, &MODEL_NAME, invocable, &CTX));
}

#[bench]
fn _0003(b: &mut Bencher) {
  let invocable = "From Singleton List Error";
  assert_decision(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, invocable, &CTX, r#"null(after coercion)"#);
  iter!(b, MODEL_EVALUATOR.evaluate_invocable(&MODEL_NAMESPACE, &MODEL_NAME, invocable, &CTX));
}

#[bench]
fn _0004(b: &mut Bencher) {
  let invocable = "From Date To Date and Time";
  assert_decision(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, invocable, &CTX, r#"2000-01-02T00:00:00Z"#);
  iter!(b, MODEL_EVALUATOR.evaluate_invocable(&MODEL_NAMESPACE, &MODEL_NAME, invocable, &CTX));
}

#[bench]
fn _0005(b: &mut Bencher) {
  let invocable = "To Singleton List BKM";
  assert_business_knowledge_model(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, invocable, &CTX, r#"[1]"#);
  iter!(b, MODEL_EVALUATOR.evaluate_invocable(&MODEL_NAMESPACE, &MODEL_NAME, invocable, &CTX));
}

#[bench]
fn _0006(b: &mut Bencher) {
  let invocable = "From Singleton List BKM";
  assert_business_knowledge_model(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, invocable, &CTX, r#"1"#);
  iter!(b, MODEL_EVALUATOR.evaluate_invocable(&MODEL_NAMESPACE, &MODEL_NAME, invocable, &CTX));
}

#[bench]
fn _0007(b: &mut Bencher) {
  let invocable = "From Date To Date and Time BKM";
  assert_business_knowledge_model(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, invocable, &CTX, r#"2000-01-02T00:00:00Z"#);
  iter!(b, MODEL_EVALUATOR.evaluate_invocable(&MODEL_NAMESPACE, &MODEL_NAME, invocable, &CTX));
}

#[bench]
fn _0008(b: &mut Bencher) {
  let invocable = "To Singleton List DS";
  assert_decision_service(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, invocable, &CTX, r#"[2000-01-02]"#);
  iter!(b, MODEL_EVALUATOR.evaluate_invocable(&MODEL_NAMESPACE, &MODEL_NAME, invocable, &CTX));
}

#[bench]
fn _0009(b: &mut Bencher) {
  let invocable = "From Singleton List DS";
  assert_decision_service(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, invocable, &CTX, r#"2000-01-02"#);
  iter!(b, MODEL_EVALUATOR.evaluate_invocable(&MODEL_NAMESPACE, &MODEL_NAME, invocable, &CTX));
}

#[bench]
fn _0010(b: &mut Bencher) {
  let invocable = "From Date to Date and Time DS";
  assert_decision_service(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, invocable, &CTX, r#"2000-01-02T00:00:00Z"#);
  iter!(b, MODEL_EVALUATOR.evaluate_invocable(&MODEL_NAMESPACE, &MODEL_NAME, invocable, &CTX));
}

#[bench]
fn _0011(b: &mut Bencher) {
  let invocable = "Implicit Conversions CE";
  assert_decision(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, invocable, &CTX, r#"[[1], 2, 2000-12-01T00:00:00Z]"#);
  iter!(b, MODEL_EVALUATOR.evaluate_invocable(&MODEL_NAMESPACE, &MODEL_NAME, invocable, &CTX));
}

#[bench]
fn _0012(b: &mut Bencher) {
  let invocable = "Implicit Conversions FUNCT";
  assert_decision(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, invocable, &CTX, r#"[[1], 2, 2000-02-01T00:00:00Z]"#);
  iter!(b, MODEL_EVALUATOR.evaluate_invocable(&MODEL_NAMESPACE, &MODEL_NAME, invocable, &CTX));
}
