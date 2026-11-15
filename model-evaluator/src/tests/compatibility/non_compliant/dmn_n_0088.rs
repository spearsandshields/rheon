use crate::model_evaluator::ModelEvaluator;
use dsntk_examples::DMN_N_0088;

#[test]
fn _0001() {
  let definitions = rheon_model::parse(DMN_N_0088).unwrap();
  assert_eq!(
    "<ItemDefinitionError> item definitions form a cycle: iTreeNode -> iTreeNode",
    ModelEvaluator::new(&[definitions]).err().unwrap().to_string()
  );
}
