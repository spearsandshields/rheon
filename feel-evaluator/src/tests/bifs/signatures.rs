//! The table of signatures ([rheon_feel::signatures]) against the evaluator: every signature is
//! invoked by position, and by name when the table says it can be, and the function takes it.

use rheon_feel::FeelScope;
use rheon_feel::signatures::{BUILT_INS, Signature};
use rheon_feel::values::Value;

/// What a built-in function answers to parameters it does not take.
const REFUSALS: [&str; 5] = [
  "not found",
  "expected the parameters of",
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
#[should_panic(expected = "found the parameter name 'text'")]
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

/// The answer to `text`, as it is shown.
fn shown(text: &str) -> String {
  let scope = FeelScope::default();
  let node = rheon_feel_parser::parse_expression(&scope, text).unwrap_or_else(|reason| panic!("{text}: {reason}"));
  super::super::eval(&scope, &node).to_string()
}

/// A name that no signature of the function has is no parameter of it: the invocation is null,
/// whatever the other parameters are, and the function is not applied.
#[test]
fn a_name_that_is_no_parameter_is_refused() {
  assert_eq!(
    "null(expected the parameters of abs(n): number or duration, found the parameter name 'x')",
    shown("abs(n: -1, x: 2)")
  );
  assert_eq!(
    "null(expected the parameters of abs(n): number or duration, found the parameter name 'x')",
    shown("abs(x: 2)")
  );
  assert_eq!(
    "null(expected the parameters of substring(string, start position, length?): string, found the parameter name 'size')",
    shown(r#"substring(string: "abc", start position: 2, size: 1)"#)
  );
  let mut invoked = 0;
  for built_in in &BUILT_INS {
    for signature in built_in.signatures.iter().filter(|signature| signature.named && !signature.parameters.is_empty()) {
      let mut named: Vec<String> = signature
        .parameters
        .iter()
        .map(|parameter| format!("{}: {}", parameter.name, sample(parameter.feel_type)))
        .collect();
      named.push("no such parameter: 1".to_string());
      for _ in 0..named.len() {
        // the name at every place among the others
        named.rotate_left(1);
        let text = format!("{}({})", built_in.name, named.join(", "));
        let answer = shown(&text);
        assert!(
          answer.starts_with("null(expected the parameters of ") && answer.ends_with(", found the parameter name 'no such parameter')"),
          "{text}: {answer}"
        );
        invoked += 1;
      }
    }
  }
  assert!(invoked > 250, "{invoked}");
}

/// The names of an invocation are those of one signature: names of two signatures of a function
/// are refused together.
#[test]
fn names_of_two_signatures_are_refused_together() {
  assert_eq!(
    "null(expected the parameters of one of before(point1, point2): boolean, before(point, range): boolean, before(range, point): boolean, before(range1, range2): boolean, found the parameter names 'point', 'range', 'point1')",
    shown("before(point: 1, range: [2..3], point1: 5)")
  );
  assert_eq!(
    "null(expected the parameters of one of date(from): date, date(year, month, day): date, found the parameter names 'from', 'year')",
    shown(r#"date(from: "2020-01-01", year: 1)"#)
  );
  // each of them alone is taken
  assert_eq!("true", shown("before(point: 1, range: [2..3])"));
  assert_eq!("true", shown("before(range: [2..3], point: 5)"));
  assert_eq!("2020-01-01", shown(r#"date(from: "2020-01-01")"#));
}

/// A name is given once: of a built-in function and of a function definition, wherever the two
/// stand among the other names.
#[test]
fn a_name_given_twice_is_refused() {
  let twice = "null(expected each parameter name once, found the name 'n' twice)";
  assert_eq!(twice, shown("abs(n: -1, n: 2)"));
  assert_eq!(twice, shown("abs(n: 1, n: 1)"));
  assert_eq!(twice, shown("{f: function(n) n, r: f(n: 1, n: 2)}.r"));
  assert_eq!(twice, shown("{f: function(m, n) [m, n], r: f(n: 1, m: 0, n: 2)}.r"));
  assert_eq!(
    "null(expected each parameter name once, found the name 'start position' twice)",
    shown(r#"substring(start position: 1, string: "abc", start position: 2)"#)
  );
  // a function of more parameters than a machine word has bits
  let names: Vec<String> = (0..200).map(|index| format!("p{index}")).collect();
  let arguments: Vec<String> = names.iter().map(|name| format!("{name}: 1")).collect();
  let call = |arguments: &[String]| format!("{{f: function({}) p199 + p0, r: f({})}}.r", names.join(", "), arguments.join(", "));
  assert_eq!("2", shown(&call(&arguments)));
  let mut with_one_twice = arguments.clone();
  with_one_twice.push("p150: 2".to_string());
  assert_eq!("null(expected each parameter name once, found the name 'p150' twice)", shown(&call(&with_one_twice)));
  // a parameter that no argument names is null
  assert_eq!("[null, 1]", shown("{f: function(m, n) [m, n], r: f(n: 1)}.r"));
}

/// By position `is` takes two values; by name, a value that is not given is null (as in the
/// test `named_params_002` of the TCK model 0103).
#[test]
fn is_takes_two_values() {
  assert_eq!("null(expected 2 parameters, actual number of parameters is 1)", shown("is(1)"));
  assert_eq!("null(expected 2 parameters, actual number of parameters is 0)", shown("is()"));
  assert_eq!("false", shown("is(value1: 1)"));
  assert_eq!("false", shown("is(value2: 1)"));
  assert_eq!("true", shown("is(value1: 1, value2: 1)"));
}
