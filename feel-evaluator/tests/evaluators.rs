use rheon_feel::values::Value;
use rheon_feel::{FeelNumber, FeelScope, scope};
use rheon_feel_evaluator::*;
use rheon_feel_parser::AstNode;

#[test]
fn _0001() {
  assert_eq!("{}", evaluate_context(&scope!(), r#"{}"#).unwrap().to_string());
}

#[test]
fn _0002() {
  let node = AstNode::Context(vec![AstNode::ContextEntry(
    Box::new(AstNode::ContextEntryKey("alpha".into())),
    Box::new(AstNode::Boolean(true)),
  )]);
  assert_eq!("{alpha: true}", evaluate_context_node(&scope!(), &node).unwrap().to_string());
}

#[test]
fn _0003() {
  let node = AstNode::Boolean(true);
  assert_eq!(
    "<FeelEvaluatorError> expected FEEL context on input",
    evaluate_context_node(&scope!(), &node).err().unwrap().to_string()
  );
}

#[test]
fn _0004() {
  let node = AstNode::Add(
    Box::new(AstNode::Numeric("1".to_string(), "23".to_string(), '+', "".into())),
    Box::new(AstNode::Numeric("1".to_string(), "77".to_string(), '+', "".into())),
  );
  let evaluator = prepare(&node);
  assert_eq!("3", evaluator(&scope!()).to_string());
}

#[test]
fn _0005() {
  let value = evaluate_sum(vec![Value::Number(FeelNumber::new(123, 2)), Value::Number(FeelNumber::new(177, 2))]);
  assert_eq!("3", value.to_string());
}

#[test]
fn _0006() {
  let value = evaluate_min(vec![Value::Number(FeelNumber::new(123, 2)), Value::Number(FeelNumber::new(177, 2))]);
  assert_eq!("1.23", value.to_string());
}

#[test]
fn _0007() {
  let value = evaluate_max(vec![Value::Number(FeelNumber::new(123, 2)), Value::Number(FeelNumber::new(177, 2))]);
  assert_eq!("1.77", value.to_string());
}

#[test]
fn _0008() {
  let node = AstNode::FunctionInvocation(Box::new(AstNode::Name("calculate".into())), Box::new(AstNode::Boolean(true)));
  assert_eq!(
    "null(expected positional or named parameter, actual AST node is Boolean(true))",
    evaluate(&scope!(), &node).into_result().unwrap().to_string()
  );
}

#[test]
fn _0009() {
  let node = AstNode::FunctionInvocation(
    Box::new(AstNode::Name("calculate".into())),
    Box::new(AstNode::NamedParameters(vec![AstNode::NamedParameter(
      Box::new(AstNode::Boolean(true)),
      Box::new(AstNode::Boolean(false)),
    )])),
  );
  assert_eq!(
    "null(expected AST node ParameterName, actual node is Boolean(true))",
    evaluate(&scope!(), &node).into_result().unwrap().to_string()
  );
}

#[test]
fn _0010() {
  let node = AstNode::Every(Box::new(AstNode::Boolean(true)), Box::new(AstNode::Boolean(false)));
  assert_eq!(
    "null(expected AST node QuantifiedContexts, actual AST node is Boolean(true))",
    evaluate(&scope!(), &node).into_result().unwrap().to_string()
  );
}

#[test]
fn _0011() {
  let node = AstNode::Some(Box::new(AstNode::Boolean(true)), Box::new(AstNode::Boolean(false)));
  assert_eq!(
    r#"null(expected AST node QuantifiedContexts, actual AST node is Boolean(true))"#,
    evaluate(&scope!(), &node).into_result().unwrap().to_string()
  );
}

#[test]
fn _0012() {
  let node = AstNode::CommaList(vec![]);
  assert_eq!("null(unexpected AST node: CommaList([]))", evaluate(&scope!(), &node).into_result().unwrap().to_string());
}

#[test]
fn _0013() {
  assert_eq!(
    "<ParserError> syntax error: [false..true]",
    evaluate_range_literal(&scope!(), "[false..true]").err().unwrap().to_string()
  );
}

#[test]
fn _0014() {
  let node = AstNode::Boolean(true);
  assert_eq!(
    "<FeelEvaluatorError> expected FEEL range on input",
    evaluate_range_literal_node(&scope!(), &node).err().unwrap().to_string()
  );
}

/// Runs `f` on a thread with a stack big enough for the call depth limit in unoptimized builds.
fn with_big_stack(f: impl FnOnce() + Send + 'static) {
  std::thread::Builder::new().stack_size(512 * 1024 * 1024).spawn(f).unwrap().join().unwrap();
}

const CALL_DEPTH: &str = "function calls are nested too deeply (more than 200 levels, e.g. runaway recursion); evaluation stopped";

#[test]
fn a_runaway_request_body_leaves_nothing_for_the_next() {
  with_big_stack(|| {
    let error = evaluate_context(&scope!(), "{f: function(n) f(n + 1), r: f(1)}").err().map(|error| error.to_string());
    assert_eq!(Some(format!("<EvaluationAborted> {CALL_DEPTH}")), error);
    assert_eq!("{a: 1}", evaluate_context(&scope!(), "{a: abs(-1)}").unwrap().to_string());
  });
}

#[test]
fn evaluate_reports_an_abort() {
  with_big_stack(|| {
    let scope = scope!();
    let runaway = rheon_feel_parser::parse_textual_expression(&scope, "{f: function(n) f(n + 1), r: f(1)}.r", false).unwrap();
    let abort = evaluate(&scope, &runaway).into_result().unwrap_err();
    assert!(matches!(abort, rheon_feel::diagnostics::Abort::CallDepth { limit: 200, .. }));
    assert_eq!(CALL_DEPTH, abort.to_string());
    let next = rheon_feel_parser::parse_textual_expression(&scope, "abs(-1)", false).unwrap();
    assert_eq!("1", evaluate(&scope, &next).into_result().unwrap().to_string());
  });
}

#[test]
fn nested_sessions_share_one_abort() {
  with_big_stack(|| {
    let scope = scope!();
    let runaway = prepare(&rheon_feel_parser::parse_textual_expression(&scope, "{f: function(n) f(n + 1), r: f(1)}.r", false).unwrap());
    let outer = Session::new().run(|| Session::new().run(|| runaway(&scope)).into_result().is_err());
    assert!(matches!(outer.into_result(), Err(rheon_feel::diagnostics::Abort::CallDepth { .. })));
    // a session started afterwards is clean
    assert_eq!(Ok(true), Session::new().run(|| true).into_result());
  });
}
