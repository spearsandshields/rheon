mod constructs;
mod host;
mod machine;
mod names;

use crate::Dialect;
use crate::flat::lower::lower;
use crate::flat::resolve::resolve;
use rheon_feel::diagnostics::{self, Diagnostics};
use rheon_feel::{Evaluator, FeelScope};
use rheon_feel_parser::AstNode;

/// The text of the value and the diagnostics of one evaluation of `evaluator`, run as `prepare`
/// runs it.
fn outcome(evaluator: Evaluator, scope: &FeelScope) -> (String, Diagnostics) {
  let evaluator = diagnostics::one_evaluation(evaluator);
  let (value, diagnostics) = diagnostics::collect(|| evaluator(scope));
  (format!("{value:?}"), diagnostics)
}

/// Evaluates `text` in `dialect` with the input context `input` (a FEEL context, or empty) by the
/// evaluator of closures and by the flat evaluator, which must lower it, and asserts that the two
/// give the same value and the same diagnostics.
#[track_caller]
pub(super) fn agree_in(dialect: Dialect, input: &str, text: &str) {
  let scope = scope(input);
  let node = rheon_feel_parser::parse_expression(&scope, text, false).unwrap_or_else(|e| panic!("{text}: {e}"));
  agree_node(dialect, &scope, &node);
}

/// The scope of the input context `input` (a FEEL context, or empty).
pub(super) fn scope(input: &str) -> FeelScope {
  if input.is_empty() { FeelScope::default() } else { crate::tests::te_scope(input) }
}

/// [agree_in] for the expression `node`.
#[track_caller]
pub(super) fn agree_node(dialect: Dialect, scope: &FeelScope, node: &AstNode) {
  let old = crate::builders::build_evaluator_with_dialect(node, dialect);
  let resolved = resolve(node);
  let program = lower(resolved, dialect).unwrap_or_else(|refusal| panic!("{node:?}: refused at {:?}", refusal.at));
  let flat: Evaluator = Box::new(move |scope: &FeelScope| program.run(scope));
  assert_eq!(outcome(old, scope), outcome(flat, scope), "{node:?}");
}

/// [agree_in] FEEL without an input.
#[track_caller]
pub(super) fn agree(text: &str) {
  agree_in(Dialect::Feel, "", text);
}
