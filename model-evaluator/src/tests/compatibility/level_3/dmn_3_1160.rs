use super::*;
use dsntk_examples::*;

// TCK 1160-multiple-imports (DMN 1.6 §6.3.10, DMN16-50): model A defines the input data `person`;
// 01-B imports A as "a", 01-C imports A as "a" and 01-B as "b"; 02-B and 02-C import with empty
// names. All five models are deployed together, so the two import names of A must not collide,
// and one binding of `person` serves every import path.
static MODEL_EVALUATOR: LazyLock<Arc<ModelEvaluator>> =
  LazyLock::new(|| build_model_evaluators(&[DMN_3_1160_01_A, DMN_3_1160_01_B, DMN_3_1160_01_C, DMN_3_1160_02_B, DMN_3_1160_02_C]));

const INPUT: &str = r#"{ person: { name: "Peter", age: 25 } }"#;

#[test]
fn _0001() {
  assert_decision(
    &MODEL_EVALUATOR,
    "https://example.org/1160-multiple-imports-01-B",
    "1160-multiple-imports-01-B",
    "DecisionB",
    &context(INPUT),
    r#""Peter""#,
  );
}

#[test]
fn _0002() {
  assert_decision(
    &MODEL_EVALUATOR,
    "https://example.org/1160-multiple-imports-01-C",
    "1160-multiple-imports-01-C",
    "DecisionC",
    &context(INPUT),
    r#""PeterPeter""#,
  );
}

#[test]
fn _0003() {
  assert_decision(
    &MODEL_EVALUATOR,
    "https://example.org/1160-multiple-imports-02-B",
    "1160-multiple-imports-02-B",
    "DecisionB",
    &context(INPUT),
    r#""Peter""#,
  );
}

#[test]
fn _0004() {
  assert_decision(
    &MODEL_EVALUATOR,
    "https://example.org/1160-multiple-imports-02-C",
    "1160-multiple-imports-02-C",
    "DecisionC",
    &context(INPUT),
    r#""PeterPeter""#,
  );
}
