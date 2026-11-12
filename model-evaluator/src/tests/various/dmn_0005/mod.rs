//! Null is null, whatever produced it: a missing input, an input whose value does not conform to its
//! type, or a decision that results in null (DMN 1.6, rule 49.a and Table 64).

use super::super::*;

const DMN_0005: &str = include_str!("_0005.dmn");

model_evaluator!(DMN_0005);

const MODEL_NAMESPACE: &str = "https://dsntk.io/null-inputs";
const MODEL_NAME: &str = "null inputs";

fn decision(name: &str, expected: &str) {
  assert_decision(&MODEL_EVALUATOR, MODEL_NAMESPACE, MODEL_NAME, name, &context(r#"{m: "x"}"#), expected);
}

#[test]
fn _0001() {
  // a missing input is null: it matches `!= 1`, `null` and `not(1)`, not `1`, `< 10` or `not(null)`
  decision("table on missing input", r#"["!= 1", "null", "not(1)"]"#);
}

#[test]
fn _0002() {
  // an input whose value does not conform to its type is null too, and matches the same entries
  decision("non-conformant input is null", "true");
  decision("table on non-conformant input", r#"["!= 1", "null", "not(1)"]"#);
}

#[test]
fn _0003() {
  // null conforms to every type, so a boxed invocation fed a null invokes the function with it
  decision("invocation fed a null", r#""null""#);
}

#[test]
fn _0004() {
  // a decision service invoked with named arguments binds an unsupplied input to null (§10.3.2.13.5)
  decision("service with a missing argument", "1");
  decision("service with all arguments", "3");
}
