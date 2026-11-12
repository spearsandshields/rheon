//! Boxed invocations and invocables invoked with input data are invoked like any function (DMN 1.6, Table 64).

use super::super::*;

const DMN_0004: &str = include_str!("_0004.dmn");

model_evaluator!(DMN_0004);

const MODEL_NAMESPACE: &str = "https://dsntk.io/invocation";
const MODEL_NAME: &str = "invocation";

fn decision(name: &str, expected: &str) {
  assert_decision(&MODEL_EVALUATOR, MODEL_NAMESPACE, MODEL_NAME, name, &context("{}"), expected);
}

fn bkm(name: &str, input_data: &str, expected: &str) {
  assert_business_knowledge_model(&MODEL_EVALUATOR, MODEL_NAMESPACE, MODEL_NAME, name, &context(input_data), expected);
}

#[test]
fn _0001() {
  // the binding's parameter type is not a conversion target, the function's parameter type is (§7.3.6)
  decision("binding typed date and time", "2020-01-01");
  decision("binding typed string", "4");
  // so an unknown binding type does not stop the model from building
  decision("binding typed unknown", "4");
}

#[test]
fn _0002() {
  // a binding that is not a parameter of the function: the function is not invoked (Table 64)
  decision("extra binding", "null(function has no parameter named 'y')");
  decision("extra typed binding", "null(function has no parameter named 'z')");
  decision("extra binding seen", "null(function has no parameter named 'y')");
}

#[test]
fn _0003() {
  // an unsupplied parameter is bound to null (§10.3.2.13.5), in boxed and in FEEL named invocations alike
  decision("missing binding", r#""none""#);
  decision("missing named argument", r#""none""#);
  decision("binding without formula", r#""none""#);
  decision("missing binding value", "null");
}

#[test]
fn _0004() {
  // a boxed invocation can invoke any function, built-in functions too (§10.2.1.3)
  decision("built in function", "1");
}

#[test]
fn _0005() {
  // recursion through boxed invocations only stops at the call depth limit, and the limit is per evaluation
  std::thread::Builder::new()
    .stack_size(512 * 1024 * 1024)
    .spawn(|| {
      decision("runaway recursion", "null(function call depth limit (200) exceeded)");
      decision("built in function", "1");
      decision("missing named argument", r#""none""#);
    })
    .unwrap()
    .join()
    .unwrap();
}

#[test]
fn _0006() {
  // a business knowledge model invoked with input data: its parameters are innermost, so a requirement
  // with the name of a parameter does not hide it (§10.3.2.13.2)
  bkm("shadowed", "{helper: 5}", "5");
}

#[test]
fn _0007() {
  // a business knowledge model invoked with input data: its arguments are converted and checked (Table 64)
  bkm("double", r#"{x: "x"}"#, "null(argument for parameter 'x' does not conform to type number)");
  bkm("double", "{x: [1]}", "2");
  bkm("double", "{x: 3, y: 4}", "6");
}

#[test]
fn _0008() {
  // a business knowledge model invoked with input data: a missing input is bound to null
  bkm("flag", "{a: 1}", r#""none""#);
  bkm("second", "{a: 1}", "null");
}
