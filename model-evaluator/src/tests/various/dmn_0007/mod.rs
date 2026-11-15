//! The input decision of a decision service that is invoked as a whole is read from the input
//! context, converted by its item definition as an input variable is.

use super::super::*;
use rheon_feel::diagnostics::ErrorMode;

const DMN_0007: &str = include_str!("_0007.dmn");

model_evaluator!(DMN_0007);

const MODEL_NAMESPACE: &str = "https://dsntk.io/decision-service-inputs";
const MODEL_NAME: &str = "decision service inputs";

fn evaluate(input: &str, error_mode: ErrorMode) -> (Value, rheon_feel::diagnostics::Diagnostics) {
  MODEL_EVALUATOR
    .evaluate_invocable_with_mode(MODEL_NAMESPACE, MODEL_NAME, "Doubler", &context(input), error_mode)
    .into_result()
    .unwrap()
}

#[test]
fn _0001() {
  for error_mode in [ErrorMode::Lenient, ErrorMode::Strict] {
    let (value, diagnostics) = evaluate("{Score: 3}", error_mode);
    assert_eq!(value.to_string(), "6", "{error_mode:?}");
    assert_eq!(diagnostics.count, 0, "{error_mode:?} {diagnostics:?}");
  }
}

/// An input decision that the input context does not supply is missing data, as a missing
/// input variable is: a null with a note, which is no error, also when its type is an item
/// definition. The decision multiplies it, and the null operand of the multiplication is its one
/// error.
#[test]
fn a_missing_input_decision_of_an_item_definition_type_is_a_null_with_a_note() {
  let error = "[multiplication] incompatible types: null(no name Score in context {}) * 2";
  for error_mode in [ErrorMode::Lenient, ErrorMode::Strict] {
    let (_, diagnostics) = evaluate("{}", error_mode);
    assert_eq!(diagnostics.messages, [error], "{error_mode:?}");
    assert_eq!(diagnostics.count, 1, "{error_mode:?} {diagnostics:?}");
  }
  let (value, _) = evaluate("{}", ErrorMode::Lenient);
  assert_eq!(value.to_string(), format!("null({error})"));
}
