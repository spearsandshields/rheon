//! The table of signatures ([rheon_feel::signatures]) against the evaluator: every signature is
//! invoked by position, and by name when the table says it can be, and the function takes it.

use rheon_feel::FeelScope;
use rheon_feel::signatures::{BUILT_INS, Signature};
use rheon_feel::value_number;
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
  // a value that is no list is the list of it where a list is taken (10.3.2.9.4); an item of
  // another type is named with its type
  let node = rheon_feel_parser::parse_expression(&scope, "sum(list: 1)").unwrap();
  assert_eq!(value_number!(1), super::super::eval(&scope, &node));
  let node = rheon_feel_parser::parse_expression(&scope, r#"sum(list: "a")"#).unwrap();
  assert_eq!(
    Value::Null(Some("[core::sum] invalid argument type, expected number, actual type is string".to_string())),
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

/// A list of one item given for a parameter that is no list is that item (DMN 1.6, 10.3.2.9.4,
/// from singleton list), by position and by name.
#[test]
fn a_list_of_one_item_is_its_item_for_a_parameter_that_is_no_list() {
  for (text, expected) in [
    (r#"contains(["foobar"], "of")"#, "false"),
    (r#"contains(["foobar"], ["ob"])"#, "true"),
    (r#"string length(["abc"])"#, "3"),
    (r#"string length(string: ["abc"])"#, "3"),
    ("abs([-1])", "1"),
    ("abs(n: [-1])", "1"),
    (r#"substring(["abc"], [2])"#, r#""bc""#),
    (r#"substring(string: ["abc"], start position: [2], length: [1])"#, r#""b""#),
    (r#"date(["2020-01-02"])"#, "2020-01-02"),
    ("date([2020], 1, [2])", "2020-01-02"),
    (r#"string join(["a", "b"], ["-"])"#, r#""a-b""#),
    (r#"get value([{a: 1}], ["a"])"#, "1"),
    ("not([true])", "false"),
    (r#"upper case(["abc"])"#, r#""ABC""#),
    ("decimal([1.555], [2])", "1.56"),
    ("sort([[2, 1]], [function(a, b) a < b])", "[[2, 1]]"),
    ("sort([2, 1], [function(a, b) a < b])", "[1, 2]"),
    ("abs([null])", "null([core::abs] invalid argument type, expected number, actual type is Null)"),
    // a parameter that is a list or takes every value keeps the list
    ("count([1])", "1"),
    ("string([1])", r#""[1]""#),
    ("sum([2])", "2"),
    ("max([1])", "1"),
    ("list contains([1], 1)", "true"),
    ("list contains([[1]], [1])", "true"),
    ("append([1], [2])", "[1, [2]]"),
    ("flatten([[1]])", "[1]"),
    ("index of([[1]], [1])", "[1]"),
    ("get entries({a: [1]})[1].value", "[1]"),
    // a list of another length, and an item that is not of the type of the parameter
    ("abs([])", "null([core::abs] invalid argument type, expected number, actual type is list<Null>)"),
    ("abs([1, 2])", "null([core::abs] invalid argument type, expected number, actual type is list<number>)"),
    (r#"abs(["a"])"#, "null([core::abs] invalid argument type, expected number, actual type is list<string>)"),
    ("abs([[1]])", "null([core::abs] invalid argument type, expected number, actual type is list<list<number>>)"),
    (r#"abs(n: ["a"])"#, "null([core::abs] invalid argument type, expected number, actual type is list<string>)"),
    // the count of the values is not that of a signature
    ("abs([1], [2])", "null(expected 1 parameters, actual number of parameters is 2)"),
  ] {
    assert_eq!(expected, shown(text), "{text}");
  }
}

/// Every parameter of every signature that is no list and does not take every value: the
/// function answers a list of one value of its type as it answers the value.
#[test]
fn every_parameter_that_is_no_list_takes_a_list_of_one_item() {
  let takes_a_list = |feel_type: &str| feel_type.starts_with("list") || feel_type == "Any";
  let (mut compared, mut differing) = (0, vec![]);
  for built_in in &BUILT_INS {
    for signature in built_in.signatures {
      for (index, parameter) in signature.parameters.iter().enumerate() {
        // no signature of the function has a list at this place or under this name
        let list_elsewhere = built_in.signatures.iter().any(|other| {
          other
            .parameters
            .iter()
            .enumerate()
            .any(|(place, candidate)| takes_a_list(candidate.feel_type) && (place == index || candidate.name == parameter.name || candidate.rest && place < index))
        });
        if list_elsewhere {
          continue;
        }
        let values = |wrapped: bool| -> Vec<String> {
          signature
            .parameters
            .iter()
            .enumerate()
            .map(|(place, parameter)| {
              let value = sample(parameter.feel_type);
              if wrapped && place == index { format!("[{value}]") } else { value.to_string() }
            })
            .collect()
        };
        let named = |values: Vec<String>| -> Vec<String> {
          signature
            .parameters
            .iter()
            .zip(values)
            .map(|(parameter, value)| format!("{}: {value}", parameter.name))
            .collect()
        };
        let mut pairs = vec![(values(false), values(true))];
        if signature.named {
          pairs.push((named(values(false)), named(values(true))));
        }
        for (plain, wrapped) in pairs {
          let (plain, wrapped) = (format!("{}({})", built_in.name, plain.join(", ")), format!("{}({})", built_in.name, wrapped.join(", ")));
          let (expected, answer) = (shown(&plain), shown(&wrapped));
          if expected != answer {
            differing.push(format!("{wrapped}: {answer}\n  {plain}: {expected}"));
          }
          compared += 1;
        }
      }
    }
  }
  assert!(differing.is_empty(), "{} of {compared}:\n{}", differing.len(), differing.join("\n"));
  assert!(compared > 200, "{compared}");
}

#[test]
fn an_unsupplied_named_parameter_is_null() {
  // parameters not supplied are bound to null (10.3.2.13.5); optional ones keep their meaning
  assert_eq!(shown("list contains(list: [1, null])"), "true");
  assert_eq!(shown("index of(list: [null, 1])"), "[1]");
  assert_eq!(shown("insert before(list: [1], position: 1)"), "[null, 1]");
  assert_eq!(shown(r#"substring(string: "abc", start position: 2)"#), r#""bc""#);
  assert!(shown("date(year: 2020, month: 1)").starts_with("null("));
  assert_eq!(shown("is(value1: null)"), "true");
}

#[test]
fn the_duration_conversions_of_dmn_1_7() {
  // DMN 1.7 Beta 1, Table 73: days and time duration(from), years and months duration(from)
  assert_eq!(shown(r#"days and time duration("P2DT20H14M")"#), "P2DT20H14M");
  assert_eq!(
    shown(r#"date and time("2012-12-24T23:59:00") - date and time("2012-12-22T03:45:00") = days and time duration("P2DT20H14M")"#),
    "true"
  );
  assert_eq!(shown(r#"years and months duration("P2Y2M") = years and months duration("P26M")"#), "true");
  assert_eq!(shown(r#"years and months duration(from: "P1Y")"#), "P1Y");
  assert_eq!(shown(r#"days and time duration(from: "PT1H")"#), "PT1H");
  assert!(shown(r#"days and time duration("P1Y")"#).starts_with("null(days and time duration: expected a days and time duration string"));
  assert!(shown(r#"years and months duration("P1D")"#).starts_with("null(years and months duration: expected a years and months duration string"));
  assert!(shown(r#"days and time duration("x")"#).starts_with("null(days and time duration: invalid duration literal"));
  // the two-date form stays
  assert_eq!(shown(r#"years and months duration(date("2011-12-22"), date("2013-08-24"))"#), "P1Y8M");
  // a date time literal in a range (rule 60)
  assert_eq!(shown(r#"range("[days and time duration(\"P1D\")..days and time duration(\"P2D\")]")"#), "[P1D..P2D]");
}
