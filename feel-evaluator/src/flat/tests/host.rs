//! What a function of the host sees when the flat evaluator invokes it: the binders of the
//! invocation's position, as the scopes of the evaluator of closures hold them.

use super::agree_text;
use crate::Dialect;
use rheon_feel::closure::Closure;
use rheon_feel::context::FeelContext;
use rheon_feel::values::Value;
use rheon_feel::{Evaluator, FeelScope, FeelType, FunctionBody, FunctionDefinition, Name};
use std::sync::Arc;

/// The names a body of the host looks for.
const SEEN: [&str; 9] = ["x", "a", "b", "c", "i", "j", "partial", "item", "n"];

/// A function of the host with the parameter `x`: its body is the host's, and gives the values
/// that its scope has for the names of [SEEN], with the text `-` for a name it has not.
fn see() -> Value {
  let body: Evaluator = Box::new(|scope: &FeelScope| {
    let seen: Vec<Value> = SEEN
      .iter()
      .map(|name| scope.get_value(&Name::from(*name)).unwrap_or_else(|| Value::String("-".into())))
      .collect();
    Value::List(seen.into())
  });
  let function = FunctionDefinition::new(
    vec![(Name::from("x"), FeelType::Any)],
    FunctionBody::Context(Arc::new(body)),
    false,
    Closure::default(),
    FeelContext::default(),
    FeelType::Any,
  );
  Value::FunctionDefinition(Arc::new(function))
}

/// A scope with `see`, and with `f`, a function of another expression that invokes it:
/// `function(y) see(y)`.
fn host() -> FeelScope {
  let mut input = FeelContext::default();
  input.set_entry(&Name::from("see"), see());
  let scope = FeelScope::from(input);
  let node = rheon_feel_parser::parse_expression(&scope, "function(y) see(y)", false).expect("the definition parses");
  let f = crate::evaluate(&scope, &node).into_result().expect("the definition evaluates");
  scope.set_value(&Name::from("f"), f);
  scope
}

#[track_caller]
fn agree(text: &str) {
  agree_text(Dialect::Feel, &host(), text);
}

#[test]
fn a_function_of_the_host_sees_the_entries_of_a_context_before_its_invocation() {
  for text in [
    "{a: 1, b: see(2), c: 3}",
    "{a: see(1), b: see(a), c: see(b)}",
    "{a: 1, b: {c: see(a)}, c: see(b)}",
    "{a: 1, a: see(2), b: see(3)}",
  ] {
    agree(text);
  }
}

#[test]
fn a_function_of_the_host_sees_the_variables_of_a_walk_and_its_results() {
  for text in [
    "for i in 1..3 return see(i)",
    "for i in 1..2, j in [see(i)] return see(j)",
    "for i in 1..3 return count(see(i)[7])",
    "for partial in 1..2, j in [see(1)] return see(j)",
    "for i in 1..2, partial in [see(i)] return see(i)",
    "some i in [1, 2] satisfies see(i)[5] = i",
    "every i in [1, 2], j in [see(i)] satisfies count(see(j)) = 9",
    "{a: 1, r: for i in 1..2 return {b: i, c: see(a + i)}}",
  ] {
    agree(text);
  }
}

#[test]
fn a_function_of_the_host_sees_the_item_of_a_filter() {
  for text in [
    "[1, 2, 3][see(item)[8] = item]",
    "[{a: 1}, {a: 2}][see(a)[2] = a]",
    "[{item: 1}, {item: 2}][see(item)[8] = 2]",
    "{a: 5, r: [{b: 1}, {b: 2}][count(see(b)) > a]}",
    "for i in 1..2 return [10, 20][see(i)[8] > 10]",
  ] {
    agree(text);
  }
}

#[test]
fn a_function_of_the_host_sees_the_scopes_of_every_activation_below_its_invocation() {
  for text in [
    // a function of another expression invokes it
    "f(1)",
    "{a: 1, r: f(a)}",
    "for i in 1..2 return f(i)",
    // the same layout in two activations
    "{g: function(n) if n = 0 then see(0) else [see(n), g(n - 1)], r: g(2)}.r",
    "{g: function(n) if n = 0 then f(0) else [f(n), g(n - 1)], r: g(2)}.r",
    "{a: 1, g: function(n) for i in 1..n return see(i), r: for j in 1..2 return g(j)}.r",
    "{g: function(n) function(x) see(x + n), r: g(1)(2)}.r",
  ] {
    agree(text);
  }
}
