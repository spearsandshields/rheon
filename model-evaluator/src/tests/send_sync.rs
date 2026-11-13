//! Compile-time check that the model evaluator, which a host builds once and shares between
//! threads (`Arc<ModelEvaluator>`), is `Send + Sync`.

use crate::ModelEvaluator;

fn send_sync<T: Send + Sync>() {}

#[test]
fn model_evaluator_is_send_and_sync() {
  send_sync::<ModelEvaluator>();
}
