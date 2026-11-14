use dsntk_common::{DsntkError, ToErrorMessage};
use rheon_feel::FeelType;
use rheon_feel::message::{arg, compose};
use std::fmt::Display;

/// Errors related to model evaluation.
#[derive(ToErrorMessage)]
struct ModelEvaluatorError(String);

pub fn err_business_knowledge_model_with_reference_not_found(namespace: &str, id: &str) -> DsntkError {
  ModelEvaluatorError(format!("no business knowledge model with reference: '{}#{}'", arg(namespace), arg(id))).into()
}

pub fn err_empty_literal_expression() -> DsntkError {
  ModelEvaluatorError("empty literal expression".into()).into()
}

pub fn err_empty_encapsulated_logic() -> DsntkError {
  ModelEvaluatorError("empty encapsulated logic in business knowledge model".into()).into()
}

pub fn err_invalid_item_definition_type(s: &str) -> DsntkError {
  ModelEvaluatorError(format!("invalid item definition type for '{}'", arg(s))).into()
}

pub fn err_unsupported_feel_type(feel_type: FeelType, s: &str) -> DsntkError {
  ModelEvaluatorError(format!("unsupported FEEL type: {} in {s}", arg(&feel_type))).into()
}

pub fn err_empty_feel_type() -> DsntkError {
  ModelEvaluatorError("empty FEEL type".into()).into()
}

pub fn err_empty_function_body() -> DsntkError {
  ModelEvaluatorError("empty function definition body".into()).into()
}

/// Errors of the item definitions of the models of a build, found before any of their types is built.
#[derive(ToErrorMessage)]
struct ItemDefinitionError(String);

/// Raised when item definitions refer to themselves; `cycle` names them, the first one repeated at the end.
pub fn err_item_definition_cycle(cycle: impl Display) -> DsntkError {
  ItemDefinitionError(compose(format_args!("item definitions form a cycle: {cycle}"))).into()
}
