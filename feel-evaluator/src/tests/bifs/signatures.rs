//! The table of signatures ([rheon_feel::signatures]) against the evaluator: every signature is
//! invoked by position, and by name when the table says it can be, and the function takes it.

use rheon_feel::FeelScope;
use rheon_feel::signatures::{BUILT_INS, Signature};
use rheon_feel::values::Value;

/// What a built-in function answers to parameters it does not take.
const REFUSALS: [&str; 4] = [
  "not found",
  "no version with named parameters",
  "no implementation with named parameters",
  "actual number of parameters is",
];

/// The message of a function that does not take the parameters of the invocation.
fn refusal(text: &str) -> Option<String> {
  let scope = FeelScope::default();
  let node = rheon_feel_parser::parse_expression(&scope, text).unwrap_or_else(|reason| panic!("{text}: {reason}"));
  match super::super::eval(&scope, &node) {
    Value::Null(Some(message)) if REFUSALS.iter().any(|refusal| message.contains(refusal)) => Some(format!("{text}: {message}")),
    _ => None,
  }
}

fn invoke(text: &str) {
  if let Some(refusal) = refusal(text) {
    panic!("{refusal}");
  }
}

/// A value of a type of the table, as text.
fn sample(feel_type: &str) -> &'static str {
  match feel_type {
    "number" | "Any" | "number or duration" => "1",
    "string" => r#""a""#,
    "boolean" => "true",
    "list" | "list<number>" => "[1, 2]",
    "list<boolean>" => "[true]",
    "list<string>" => r#"["a"]"#,
    "list<context>" => r#"[{key: "a", value: 1}]"#,
    "context" => "{a: 1}",
    "range" => "[1..2]",
    "function" => "function(x, y) x < y",
    "date or date and time" => r#"date("2020-01-01")"#,
    "string or date and time" => r#""2020-01-01""#,
    "time" => r#"time("10:00:00")"#,
    "string or time or date and time" => r#""10:00:00""#,
    "days and time duration" => r#"duration("PT1H")"#,
    other => panic!("no sample value of the type {other}"),
  }
}

/// The invocations of a signature: with every parameter and without the optional ones, a rest
/// parameter once and three times; each by position, and by name when the signature is named.
fn invocations(name: &str, signature: &Signature) -> Vec<String> {
  let mut texts = vec![];
  let required = signature.parameters.iter().filter(|parameter| !parameter.optional).count();
  for count in [required, signature.parameters.len()] {
    let parameters = &signature.parameters[..count];
    let mut positional: Vec<&str> = parameters.iter().map(|parameter| sample(parameter.feel_type)).collect();
    texts.push(format!("{name}({})", positional.join(", ")));
    if parameters.last().is_some_and(|parameter| parameter.rest) {
      let more = sample(parameters[count - 1].feel_type);
      positional.extend([more, more]);
      texts.push(format!("{name}({})", positional.join(", ")));
    }
    if signature.named && count > 0 {
      let named: Vec<String> = parameters.iter().map(|parameter| format!("{}: {}", parameter.name, sample(parameter.feel_type))).collect();
      texts.push(format!("{name}({})", named.join(", ")));
    }
  }
  texts
}

#[test]
fn the_evaluator_takes_every_signature() {
  let (mut invoked, mut refused) = (0, vec![]);
  for built_in in &BUILT_INS {
    for signature in built_in.signatures {
      for text in invocations(built_in.name, signature) {
        refused.extend(refusal(&text));
        invoked += 1;
      }
    }
  }
  assert!(refused.is_empty(), "{}", refused.join("\n"));
  assert!(invoked > 300, "{invoked}");
}

/// The check can fail: parameters that a function does not take are refused with one of the
/// messages it looks for.
#[test]
#[should_panic(expected = "not found")]
fn a_wrong_name_is_refused() {
  invoke(r#"substring(text: "a", start position: 1)"#);
}

#[test]
#[should_panic(expected = "actual number of parameters is")]
fn a_wrong_count_is_refused() {
  invoke(r#"substring("a")"#);
}

/// A parameter that is given by its name is never reported as not found, whatever its value:
/// a value of another type has a message that names the type.
#[test]
fn a_given_parameter_is_never_reported_as_not_found() {
  let (mut invoked, mut refused) = (0, vec![]);
  for built_in in &BUILT_INS {
    for signature in built_in.signatures.iter().filter(|signature| signature.named) {
      for wrong in ["null", "true", "{a: 1}", "[1, 2]", "1", r#""a""#] {
        for index in 0..signature.parameters.len() {
          let named: Vec<String> = signature
            .parameters
            .iter()
            .enumerate()
            .map(|(position, parameter)| format!("{}: {}", parameter.name, if position == index { wrong } else { sample(parameter.feel_type) }))
            .collect();
          refused.extend(refusal(&format!("{}({})", built_in.name, named.join(", "))));
          invoked += 1;
        }
      }
    }
  }
  assert!(refused.is_empty(), "{}", refused.join("\n"));
  assert!(invoked > 600, "{invoked}");
}

#[test]
fn a_parameter_of_another_type_is_named_with_its_type() {
  let scope = FeelScope::default();
  let node = rheon_feel_parser::parse_expression(&scope, "sum(list: 1)").unwrap();
  assert_eq!(
    Value::Null(Some("[core::sum] invalid argument type, expected list, actual type is number".to_string())),
    super::super::eval(&scope, &node)
  );
}
