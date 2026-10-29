use flowstone_feel::value_null;
use flowstone_feel::values::Value;

pub fn evaluate_external_java_function(_class_name: &str, _method_signature: &str, _arguments: &[Value]) -> Value {
  value_null!("external Java functions are not supported on wasm32")
}
