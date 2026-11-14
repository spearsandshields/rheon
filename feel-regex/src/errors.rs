//! # FEEL regex errors

use dsntk_common::{DsntkError, ToErrorMessage};
use dsntk_macros::ToErrorMessage;

/// `FEEL` expressions evaluator errors.
#[derive(ToErrorMessage)]
struct FeelRegexError(String);

pub fn err_empty_flags() -> DsntkError {
  FeelRegexError("empty regex flags".to_string()).into()
}

pub fn err_invalid_flag(flag: char) -> DsntkError {
  FeelRegexError(format!("invalid regex flag: '{flag}'").to_string()).into()
}

pub fn err_invalid_pattern(pattern: &str) -> DsntkError {
  FeelRegexError(format!("invalid regex pattern: '{pattern}'").to_string()).into()
}

/// Why a regular expression gave no answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegexError {
  /// An invalid pattern or flags (XQuery F&O 3.1, FORX0001-FORX0004): the FEEL result is null.
  Invalid(DsntkError),
  /// The engine gave up on a pattern F&O accepts: an implementation limit, never a FEEL value.
  LimitExceeded(Limit),
}

/// A limit of the regex engines.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Limit {
  /// The number of backtracking steps of one search.
  Backtracking,
  /// The stack of the backtracking search.
  BacktrackingStack,
  /// The size of the compiled pattern.
  CompiledSize,
  /// The nesting depth of the pattern.
  Nesting,
}

impl std::fmt::Display for Limit {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(match self {
      Limit::Backtracking => "backtracking limit",
      Limit::BacktrackingStack => "backtracking stack limit",
      Limit::CompiledSize => "compiled size limit",
      Limit::Nesting => "nesting limit",
    })
  }
}
