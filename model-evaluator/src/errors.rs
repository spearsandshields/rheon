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

/// Errors of a decision table whose parts do not make a table (DMN 1.6 §8.2, §8.3).
#[derive(ToErrorMessage)]
struct DecisionTableError(String);

/// Raised when a decision table has no output clause.
pub fn err_table_without_output() -> DsntkError {
  DecisionTableError("decision table: expected one or more output clauses, found none".into()).into()
}

/// Raised when a decision table of several outputs has an output without a name.
pub fn err_table_unnamed_output(output: usize, outputs: usize) -> DsntkError {
  DecisionTableError(format!("decision table: expected a name for each of its {outputs} outputs, found none for output {output}")).into()
}

/// Raised when a rule has not an entry for each clause; `what` is `input` or `output`.
pub fn err_table_rule_entries(rule: usize, what: &str, expected: usize, found: usize) -> DsntkError {
  let entries = if expected == 1 { "entry" } else { "entries" };
  DecisionTableError(format!("decision table: expected {expected} {what} {entries} in rule {rule}, found {found}")).into()
}

/// Raised when the hit policy of a decision table is not one that its shape allows.
pub fn err_table_hit_policy(policy: &str, expected: &str, found: &str) -> DsntkError {
  DecisionTableError(format!("decision table: expected {expected} for the hit policy {policy}, found {found}")).into()
}

/// Errors of the item definitions of the models of a build, found before any of their types is built.
#[derive(ToErrorMessage)]
struct ItemDefinitionError(String);

/// Raised when item definitions refer to themselves; `cycle` names them, the first one repeated at the end.
pub fn err_item_definition_cycle(cycle: impl Display) -> DsntkError {
  ItemDefinitionError(compose(format_args!("item definitions form a cycle: {cycle}"))).into()
}

/// Raised when the type of an item definition has more nodes than a type may have.
pub fn err_item_definition_too_large(name: &str, max_nodes: u64) -> DsntkError {
  ItemDefinitionError(compose(format_args!("item definition '{}' expands to more than {max_nodes} type nodes", arg(name)))).into()
}

/// Raised when the type of an item definition nests deeper than a type may nest.
pub fn err_item_definition_too_deep(name: &str, max_depth: u64) -> DsntkError {
  ItemDefinitionError(compose(format_args!("item definition '{}' nests types more than {max_depth} levels deep", arg(name)))).into()
}
