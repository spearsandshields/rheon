//! Values nest at most 500 levels within an evaluation, also those that a boxed expression builds
//! from the values of its parts: a boxed list, a boxed context, and an entry of a boxed context
//! whose collection type puts its value in a list.

use super::super::*;

const DMN_0009: &str = include_str!("_0009.dmn");

model_evaluator!(DMN_0009);

const MODEL_NAMESPACE: &str = "https://dsntk.io/value-depth";
const MODEL_NAME: &str = "value depth";

const VALUE_DEPTH: &str = "aborted(values are nested too deeply (more than 500 levels); evaluation stopped)";

fn decision(name: &str, expected: &str) {
  assert_decision(&MODEL_EVALUATOR, MODEL_NAMESPACE, MODEL_NAME, name, &context("{}"), expected);
}

/// The text of a list `levels` deep: lists around an empty one.
fn list_text(levels: usize) -> String {
  format!("{}{}", "[".repeat(levels), "]".repeat(levels))
}

#[test]
fn _0001() {
  decision("boxed list 500", &list_text(500));
  decision("boxed list 501", VALUE_DEPTH);
}

#[test]
fn _0002() {
  decision("boxed context 500", &format!("{{a: {}}}", list_text(499)));
  decision("boxed context 501", VALUE_DEPTH);
}

#[test]
fn _0003() {
  // the entry is put in a list, a level deeper than its value, before the entry after it reads it
  decision("boxed context entry 500", "1");
  decision("boxed context entry 501", VALUE_DEPTH);
}
