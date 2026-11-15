//! A regular expression that exceeds a limit of the regex engine stops the evaluation of the
//! decision (DMN 1.6, §7.3.8): the outcome is the abort, not a null or a `false`.

use super::super::*;

const DMN_0006: &str = include_str!("_0006.dmn");

model_evaluator!(DMN_0006);

const MODEL_NAMESPACE: &str = "https://dsntk.io/regex-limits";
const MODEL_NAME: &str = "regex limits";

fn decision(name: &str, input: &str, expected: &str) {
  assert_decision(&MODEL_EVALUATOR, MODEL_NAMESPACE, MODEL_NAME, name, &context(input), expected);
}

#[test]
fn _0001() {
  decision("match", r#"{text: "aa"}"#, "true");
  decision(
    "match",
    &format!(r#"{{text: "{}!"}}"#, "a".repeat(40)),
    r"aborted(matches: the pattern '^(a|a)*\1$' exceeds the regex engine's backtracking limit; evaluation stopped)",
  );
  // the next evaluation on the same thread starts clean
  decision("back reference", "{}", "true");
}
