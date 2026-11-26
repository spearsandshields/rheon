//! # FEEL evaluator errors

use dsntk_common::{DsntkError, ToErrorMessage};
use std::fmt::Display;

/// `FEEL` expressions evaluator errors.
#[derive(ToErrorMessage)]
struct FeelEvaluatorError(String);

pub fn err_not_a_context() -> DsntkError {
  FeelEvaluatorError("expected FEEL context on input".to_string()).into()
}

pub fn err_not_a_range() -> DsntkError {
  FeelEvaluatorError("expected FEEL range on input".to_string()).into()
}

pub fn err_invalid_interval_start(start_type: impl Display) -> DsntkError {
  FeelEvaluatorError(format!("invalid type of interval start: {start_type}")).into()
}

pub fn err_invalid_interval_end(expected: &str, end_type: impl Display) -> DsntkError {
  FeelEvaluatorError(format!("invalid type of interval end, expected {expected}, actual: {end_type}")).into()
}

pub fn err_range_iteration_context(range: impl Display) -> DsntkError {
  FeelEvaluatorError(format!("iteration context expected a list or an interval a..b, found the range {range}")).into()
}

pub fn err_interval_not_integers(start: impl Display, end: impl Display) -> DsntkError {
  FeelEvaluatorError(format!("interval a..b expected two integers or two dates, found {start} and {end}")).into()
}
