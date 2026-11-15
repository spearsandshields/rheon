//! # FEEL regex errors

use dsntk_common::{DsntkError, ToErrorMessage};
use dsntk_macros::ToErrorMessage;

/// `FEEL` expressions evaluator errors.
#[derive(ToErrorMessage)]
struct FeelRegexError(String);

pub fn err_invalid_flag(flag: char) -> DsntkError {
  FeelRegexError(format!("invalid regex flag: '{flag}'").to_string()).into()
}

pub fn err_lone_dollar() -> DsntkError {
  FeelRegexError("invalid replacement string: a '$' that is not escaped must be followed by a digit".to_string()).into()
}

pub fn err_lone_backslash() -> DsntkError {
  FeelRegexError(r"invalid replacement string: a '\' must be followed by '\' or '$'".to_string()).into()
}

pub fn err_invalid_pattern(pattern: &str) -> DsntkError {
  FeelRegexError(format!("invalid regex pattern: '{pattern}'").to_string()).into()
}

/// Why a regular expression gave no answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegexError {
  /// A pattern that is invalid (XQuery 1.0 and XPath 2.0 F&O §7.6, FORX0002): the FEEL result
  /// is null. The reason quotes the pattern as the caller wrote it.
  InvalidPattern(DsntkError),
  /// Flags (FORX0001) or a replacement string (FORX0004) that are invalid: the FEEL result is
  /// null. The reason names the rule.
  InvalidArgument(DsntkError),
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
