//! # External PMML functions
//!
//! rheon does not score PMML models (DMN 1.7 Beta 1, 2.1: support for the PMML kind is optional):
//! an external PMML function evaluates to null with a reason, never to a made-up value.

use rheon_feel::value_null;
use rheon_feel::values::Value;

/// Evaluates an external PMML function: null with a reason.
pub fn evaluate_external_pmml_function(document: &str, _model_name: &str, _arguments: &[Value]) -> Value {
  if document.is_empty() {
    value_null!("PMML document not specified")
  } else {
    value_null!("external PMML functions are not supported")
  }
}
