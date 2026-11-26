//! The start rules of S-FEEL and the lists of expressions (DT-002): `simple expression` and
//! `simple expressions` (DMN 1.7 Beta 1, clause 9.2, rules 3 and 4), `textual expressions`
//! (grammar rule 3).

use crate::{AstNode, StartRule, parse_start, parse_tolerant_start};
use rheon_feel::context::FeelContext;
use rheon_feel::values::Value;
use rheon_feel::{FeelScope, Name};

fn scope() -> FeelScope {
  let mut context = FeelContext::default();
  for name in ["Age", "Applicant Age", "a", "b", "c", "x"] {
    context.set_entry(&Name::from(name), Value::Number(1.into()));
  }
  FeelScope::from(context)
}

fn parses(rule: StartRule, text: &str) -> bool {
  parse_start(&scope(), rule, text).is_ok()
}

/// The messages of a tolerant parse.
fn messages(rule: StartRule, text: &str) -> Vec<String> {
  parse_tolerant_start(&scope(), text, rule)
    .diagnostics
    .into_iter()
    .map(|diagnostic| diagnostic.message)
    .collect()
}

#[test]
fn simple_expressions_of_s_feel() {
  for text in [
    "1",
    "-1.5",
    "\"text\"",
    "true",
    "Age",
    "Applicant Age",
    "?",
    "a.b.c",
    "Age + 1",
    "a - b * c / 2 ** x",
    "-Age",
    "Age >= 18",
    "a + 1 = b",
    "date(\"2020-01-01\")",
    "time(\"10:00:00\")",
    "duration(\"P1D\")",
    "days and time duration(\"P1D\")",
    "years and months duration(\"P1Y\")",
    "date(\"2020-01-01\") + duration(\"P1D\")",
  ] {
    assert!(parses(StartRule::SimpleExpression, text), "{text:?} is a simple expression");
    assert!(messages(StartRule::SimpleExpression, text).is_empty(), "{text:?}");
  }
}

#[test]
fn constructs_that_s_feel_has_not() {
  for (text, found) in [
    ("(Age + 1) * 2", "parentheses `(Age + 1)`"),
    ("null", "the null literal `null`"),
    ("@\"2020-01-01\"", "an `@` literal"),
    ("1e3", "a number with an exponent `1e3`"),
    ("[1, 2]", "a list `[1, 2]`"),
    ("[1..2]", "an interval `[1..2]`"),
    ("{x: 1}", "a context"),
    ("function(y) y", "a function definition"),
    ("if a then 1 else 2", "an if expression `if a then 1 else 2`"),
    ("for y in a return y", "a for expression"),
    ("some y in a satisfies y", "a quantified expression"),
    ("a and b", "a conjunction `a and b`"),
    ("a or b", "a disjunction `a or b`"),
    ("a in [1..2]", "an in test"),
    ("a between 1 and 2", "a between test"),
    ("a instance of number", "an instance of test"),
    ("abs(a)", "a function invocation `abs(a)`"),
    ("a[1]", "a filter"),
    ("abs(a).b", "a path `abs(a).b`"),
    ("date and time(\"2020-01-01T10:00:00\")", "a date and time"),
    ("date(2020, 1, 1)", "a temporal literal of other than one string literal"),
    ("< 5", "a unary test"),
  ] {
    assert!(!parses(StartRule::SimpleExpression, text), "{text:?} is no simple expression");
    // but an expression
    assert!(parses(StartRule::Expression, text) || text == "< 5", "{text:?} is an expression");
    let messages = messages(StartRule::SimpleExpression, text);
    assert_eq!(messages.len(), 1, "{text:?}: {messages:?}");
    assert!(
      messages[0].starts_with(&format!("Expected a simple expression (S-FEEL, DMN 9.2), found {found}")),
      "{text:?}: {}",
      messages[0]
    );
  }
}

#[test]
fn one_error_for_the_outermost_construct() {
  // the list is the error, not the parentheses in it
  assert_eq!(messages(StartRule::SimpleExpression, "[(1)]").len(), 1);
  // two constructs side by side are two errors
  assert_eq!(messages(StartRule::SimpleExpression, "(a) + (b)").len(), 2);
}

#[test]
fn lists_of_expressions() {
  let list = |rule: StartRule, text: &str| match &parse_start(&scope(), rule, text) {
    Ok(AstNode::List(items)) => items.len(),
    other => panic!("{text:?}: {other:?}"),
  };
  assert_eq!(list(StartRule::SimpleExpressions, "Age, Age + 1, \"x\""), 3);
  assert_eq!(list(StartRule::SimpleExpressions, "Age"), 1);
  assert_eq!(list(StartRule::TextualExpressions, "if a then 1 else 2, for y in a return y, < 5"), 3);
  // commas inside the expressions are theirs
  assert_eq!(list(StartRule::TextualExpressions, "abs(a), ([1, 2])"), 2);
  assert!(!parses(StartRule::SimpleExpressions, "Age, if a then 1 else 2"));
  assert!(!parses(StartRule::SimpleExpressions, "Age,"));
  // an expression alone is no list
  assert!(!parses(StartRule::Expression, "1, 2"));
  // a boxed expression alone is no textual expression
  assert!(!parses(StartRule::TextualExpressions, "[1, 2], 3"));
  assert_eq!(
    messages(StartRule::TextualExpressions, "{x: 1}, 3"),
    vec!["Expected a textual expression, found a list, a context or a function definition alone `{x: 1}`: write it in parentheses.".to_string()]
  );
}

#[test]
fn a_long_path_is_read_in_linear_time() {
  // review of DT-002: the check of a qualified name does not read the path again at each dot
  let text = format!("a{}", ".a".repeat(5_000));
  for rule in [StartRule::Expression, StartRule::SimpleExpression] {
    let started = std::time::Instant::now();
    let _ = parse_tolerant_start(&scope(), &text, rule);
    let _ = parse_start(&scope(), rule, &text);
    assert!(started.elapsed() < std::time::Duration::from_secs(1), "{rule:?}: {:?}", started.elapsed());
  }
  assert!(parses(StartRule::SimpleExpression, "a.b.c.d"));
  assert!(!parses(StartRule::SimpleExpression, "(a).b"));
}
