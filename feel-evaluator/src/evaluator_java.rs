//! # External Java functions
//!
//! rheon runs no JVM and makes no network call during an evaluation, on any target: an external
//! Java function evaluates to null with a reason (DMN 1.7 Beta 1, 2.1: support for the Java kind is
//! optional). Earlier native builds posted the call to a Java service on `127.0.0.1:22023`.

use rheon_feel::value_null;
use rheon_feel::values::Value;

/// Evaluates an external Java function: always null with a reason.
pub fn evaluate_external_java_function(_class_name: &str, _method_signature: &str, _arguments: &[Value]) -> Value {
  value_null!("external Java functions are not supported")
}
