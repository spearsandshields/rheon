use crate::errors::*;
use crate::{Dialect, Outcome, Session};
use dsntk_common::Result;
use rheon_feel::context::FeelContext;
use rheon_feel::diagnostics;
use rheon_feel::values::Value;
use rheon_feel::{Evaluator, FeelScope, Name};
use rheon_feel_parser::AstNode;
use std::sync::Arc;

/// Evaluates a [Value] from given [AstNode] in its own [Session].
pub fn evaluate(scope: &FeelScope, node: &AstNode) -> Outcome<Value> {
  Session::new().run(|| crate::builders::build_evaluator(node)(scope))
}

/// Prepares an evaluator for given [AstNode].
///
/// Each run of the evaluator is one evaluation: inside a [Session] it is part of that session,
/// outside one it is a session of its own. When the evaluation is stopped (see [Outcome]), its
/// result is null with the reason of the abort, never a value computed after it.
pub fn prepare(node: &AstNode) -> Evaluator {
  diagnostics::one_evaluation(crate::builders::build_evaluator(node))
}

/// Builds the evaluator of an expression in the given [Dialect](crate::Dialect) (FEEL or B-FEEL),
/// run as one evaluation like the evaluators of [prepare].
pub fn prepare_with_dialect(node: &AstNode, dialect: crate::Dialect) -> Evaluator {
  diagnostics::one_evaluation(crate::builders::build_evaluator_with_dialect(node, dialect))
}

/// Evaluates the sum of specified values.
pub fn evaluate_sum(values: Vec<Value>) -> Value {
  crate::bifs::core::sum(&values)
}

/// Evaluates the minimum value from specified values.
pub fn evaluate_min(values: Vec<Value>) -> Value {
  crate::bifs::core::min(&values)
}

/// Evaluates the maximum value from specified values.
pub fn evaluate_max(values: Vec<Value>) -> Value {
  crate::bifs::core::max(&values)
}

/// Compares two values and returns `true` when the two `FEEL` values are equal.
pub fn evaluate_equals(left: &Value, right: &Value) -> bool {
  crate::equality::eval_ternary_equality(left, right).unwrap_or(false)
}

/// Evaluates the invocation of a function with named arguments, `function(n1: v1, ...)` (DMN 1.6, Table 64):
/// the meaning of a boxed invocation (§10.2.1.3) and of an invocable invoked with input data (§10.4).
/// The dialect selects the semantics of built-in functions.
///
/// The invocation is part of the running [Session]. Outside one, it is an evaluation of its own,
/// and when that is stopped its result is null with the reason of the abort.
pub fn evaluate_invocation(scope: &FeelScope, function: Value, arguments: Vec<(Name, Value)>, dialect: Dialect) -> Value {
  crate::invocation::call(scope, function, crate::invocation::Arguments::Named(arguments), dialect)
}

/// Evaluates a context from text containing `FEEL` expression.
pub fn evaluate_context(scope: &FeelScope, input: &str) -> Result<FeelContext> {
  let node = &rheon_feel_parser::parse_context(scope, input, false)?;
  evaluate_context_node(scope, node)
}

/// Evaluates a context from AST node in its own [Session]; a stopped evaluation is an error.
pub fn evaluate_context_node(scope: &FeelScope, node: &AstNode) -> Result<FeelContext> {
  if let Value::Context(context) = evaluate(scope, node).into_result()? {
    Ok(Arc::unwrap_or_clone(context))
  } else {
    Err(err_not_a_context())
  }
}

/// Evaluates a range value from range literal expression.
pub fn evaluate_range_literal(scope: &FeelScope, input: &str) -> Result<Value> {
  let node = &rheon_feel_parser::parse_range_literal(scope, input, false)?;
  evaluate_range_literal_node(scope, node)
}

/// Evaluates a range value from AST node in its own [Session]; a stopped evaluation is an error.
pub fn evaluate_range_literal_node(scope: &FeelScope, node: &AstNode) -> Result<Value> {
  if let value @ Value::Range(_, _, _, _) = evaluate(scope, node).into_result()? {
    Ok(value)
  } else {
    Err(err_not_a_range())
  }
}
