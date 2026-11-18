//! Values nest at most 500 levels within an evaluation. Every channel that builds a deeper value
//! answers at 500 levels and stops the evaluation at 501; a value 500 levels deep can be bound to a
//! parameter, a loop variable, `item` and `partial`, and returned, although the scope frame that
//! holds it is one level deeper.

use rheon_feel::context::FeelContext;
use rheon_feel::values::Value;
use rheon_feel::{FeelScope, Name};
use rheon_feel_evaluator::evaluate;

const VALUE_DEPTH: &str = "values are nested too deeply (more than 500 levels); evaluation stopped";

/// Runs `f` on a thread with a stack big enough for deep values in unoptimized builds.
fn with_big_stack(f: impl FnOnce() + Send + 'static) {
  std::thread::Builder::new().stack_size(512 * 1024 * 1024).spawn(f).unwrap().join().unwrap();
}

/// The nesting depth by its definition: 0 for a scalar, one more than the deepest value held for a
/// list, a context (so 1 for an empty one), a range and a function (its closure context).
fn depth(value: &Value) -> usize {
  match value {
    Value::List(items) => 1 + items.iter().map(depth).max().unwrap_or(0),
    Value::Context(ctx) => 1 + ctx.iter().map(|(_, value)| depth(value)).max().unwrap_or(0),
    Value::Range(start, _, end, _) => 1 + depth(start).max(depth(end)),
    Value::FunctionDefinition(function) => 2 + function.closure_ctx.iter().map(|(_, value)| depth(value)).max().unwrap_or(0),
    _ => 0,
  }
}

/// A list `levels` deep: `levels - 1` lists around an empty one.
fn deep_list(levels: usize) -> Value {
  let mut value = Value::List(vec![].into());
  for _ in 1..levels {
    value = Value::List(vec![value].into());
  }
  value
}

/// A context `levels` deep: `levels - 1` contexts `{a: ..}` around an empty one.
fn deep_context(levels: usize) -> Value {
  let mut value = Value::from(FeelContext::default());
  for _ in 1..levels {
    let mut ctx = FeelContext::default();
    ctx.set_entry(&Name::from("a"), value);
    value = Value::from(ctx);
  }
  value
}

/// The scope with the inputs `l498`, `l499`, `l500` (lists) and `c498`, `c499`, `c500` (contexts)
/// of those depths.
fn inputs() -> FeelScope {
  let mut ctx = FeelContext::default();
  ctx.set_entry(&Name::from("l498"), deep_list(498));
  ctx.set_entry(&Name::from("c498"), deep_context(498));
  ctx.set_entry(&Name::from("l499"), deep_list(499));
  ctx.set_entry(&Name::from("l500"), deep_list(500));
  ctx.set_entry(&Name::from("c499"), deep_context(499));
  ctx.set_entry(&Name::from("c500"), deep_context(500));
  FeelScope::from(ctx)
}

/// The value of `text`, or the text of the abort that stopped its evaluation.
fn eval(scope: &FeelScope, text: &str) -> Result<Value, String> {
  let node = rheon_feel_parser::parse_expression(scope, text).unwrap_or_else(|error| panic!("{text}: {error}"));
  evaluate(scope, &node).into_result().map_err(|abort| abort.to_string())
}

/// Every expression answers a value of the given depth.
fn answers(cases: &[(&str, usize)]) {
  let scope = inputs();
  let differ: Vec<String> = cases
    .iter()
    .filter_map(|&(text, expected)| match eval(&scope, text) {
      Ok(value) if depth(&value) == expected => None,
      Ok(value) => Some(format!("{text}: depth {} instead of {expected}", depth(&value))),
      Err(abort) => Some(format!("{text}: stopped ({abort}) instead of a value {expected} deep")),
    })
    .collect();
  assert!(differ.is_empty(), "{} of {} differ:\n{}", differ.len(), cases.len(), differ.join("\n"));
}

/// Every expression stops its evaluation with the value-depth abort, and the next evaluation is clean.
fn stopped(cases: &[&str]) {
  let scope = inputs();
  let differ: Vec<String> = cases
    .iter()
    .filter_map(|&text| match eval(&scope, text) {
      Err(abort) if abort == VALUE_DEPTH => None,
      Err(abort) => Some(format!("{text}: stopped by {abort}")),
      Ok(value) => Some(format!("{text}: a value {} deep", depth(&value))),
    })
    .collect();
  assert!(differ.is_empty(), "{} of {} differ:\n{}", differ.len(), cases.len(), differ.join("\n"));
  assert_eq!(Ok(Value::List(vec![deep_list(499)].into())), eval(&scope, "[l499]"));
}

#[test]
fn the_inputs_have_their_depths() {
  with_big_stack(|| {
    answers(&[("l498", 498), ("l499", 499), ("l500", 500), ("c498", 498), ("c499", 499), ("c500", 500)]);
  });
}

#[test]
fn every_channel_answers_at_500_levels() {
  with_big_stack(|| {
    answers(&[
      // a list literal
      ("[l499]", 500),
      // a context literal: its result, and an entry that is a context literal
      ("{a: l499}", 500),
      ("{a: {b: c499}.b}.a", 499),
      // a range literal
      ("[l499..1]", 500),
      // the results of a `for`, and as they grow by `partial`
      ("for i in 1..2 return l499", 500),
      ("for i in 1..2 return if i = 1 then l498 else [partial[-1]]", 500),
      ("for i in 1..2 return if i = 1 then c498 else {a: partial[-1]}", 500),
      // a recursion around a list literal: 99 calls of 5 levels each around a list 5 deep (the
      // function is passed to itself, as a context holding the result would be a level deeper)
      ("(function(f) f(f, 99))(function(g, n) if n = 0 then [[[[[]]]]] else [[[[[g(g, n - 1)]]]]])", 500),
      // the result of a built-in
      ("append([], l499)", 500),
      ("get entries({a: l498})", 500),
      ("context put({}, for i in 1..500 return \"a\", 0)", 500),
      ("context put({}, for i in 1..499 return \"a\", [])", 500),
      // a function: one level, and its closure context another
      ("(function(x) function() x)(l498)", 500),
      // a filter of a value that is no list, which applies to the list that holds it
      ("l499[true]", 499),
      ("c499[true]", 500),
    ]);
  });
}

#[test]
fn every_channel_stops_at_501_levels() {
  with_big_stack(|| {
    stopped(&[
      "[l500]",
      "{a: l500}",
      "{a: {b: l500}}",
      "[l500..1]",
      "{a: l500, b: count(a)}.b",
      "for i in 1..2 return l500",
      "for i in 1..3 return if i = 1 then l498 else [partial[-1]]",
      "for i in 1..3 return if i = 1 then c498 else {a: partial[-1]}",
      "(function(f) f(f, 99))(function(g, n) if n = 0 then [[[[[[]]]]]] else [[[[[g(g, n - 1)]]]]])",
      "append([], l500)",
      "get entries({a: l499})",
      "context put({}, for i in 1..501 return \"a\", 0)",
      "context put({}, for i in 1..500 return \"a\", [])",
      "(function(x) function() x)(l499)",
      "c500[true]",
    ]);
  });
}

/// The cases the previous release ran without end, or exhausted the stack in: a loop that would go
/// on deepening its results, and a key list that would build 100,000 contexts in each other.
#[test]
fn a_deeper_value_stops_the_evaluation_before_it_is_built() {
  with_big_stack(|| {
    stopped(&[
      "for i in 1..100000 return if i = 1 then l498 else [partial[-1]]",
      "for i in 1..100000 return if i = 1 then c498 else {a: partial[-1]}",
      "context put({}, for i in 1..100000 return \"a\", 0)",
    ]);
  });
}

/// A key list whose path is blocked by an entry that is not a context builds nothing, however
/// long it is: `context put` gives null, as for a short key list.
#[test]
fn a_blocked_key_path_builds_nothing() {
  with_big_stack(|| {
    let scope = inputs();
    let cases = [
      "context put({a: 1}, for i in 1..501 return \"a\", 0)",
      "context put({a: {a: {a: 1}}}, for i in 1..501 return \"a\", 0)",
      "context put({a: {a: {a: 1}}}, for i in 1..100000 return \"a\", 0)",
    ];
    let differ: Vec<String> = cases
      .iter()
      .filter_map(|&text| match eval(&scope, text) {
        Ok(Value::Null(_)) => None,
        Ok(value) => Some(format!("{text}: {value} instead of null")),
        Err(abort) => Some(format!("{text}: stopped ({abort}) instead of null")),
      })
      .collect();
    assert!(differ.is_empty(), "{} of {} differ:\n{}", differ.len(), cases.len(), differ.join("\n"));
  });
}

#[test]
fn a_value_500_levels_deep_is_bound_and_returned() {
  with_big_stack(|| {
    answers(&[
      // a parameter, and the result of the call
      ("(function(x) x)(l500)", 500),
      ("{f: function(x) count(x)}.f(c500)", 0),
      // a loop variable: a value that is no list is the only item
      ("for x in c500 return count(x)", 1),
      ("every x in c500 satisfies x = x", 0),
      ("for x in l500 return x", 500),
      // `item`, when the filter selects nothing
      ("count(c500[item.a = 0])", 0),
      ("l500[count(item) = 1]", 500),
      // `partial`
      ("for i in 1..2 return if i = 1 then l499 else count(partial)", 500),
      // a path into it
      ("c500.a", 499),
    ]);
  });
}
