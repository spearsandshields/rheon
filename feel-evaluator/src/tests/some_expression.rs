use super::*;

#[test]
fn _0001() {
  let scope = &te_scope("{}");
  te_bool(false, scope, r#"some n in [1,2,3] satisfies n > 1.5"#, true);
}

#[test]
fn _0002() {
  let scope = &te_scope("{}");
  te_bool(false, scope, r#"some n in [1,2,3] satisfies n > 10.5"#, false);
}

#[test]
fn _0003() {
  let scope = &te_scope("{}");
  te_bool(false, scope, r#"some n in 1 satisfies n > 10.5"#, false);
}

#[test]
fn _0004() {
  let scope = &te_scope("{}");
  te_bool(false, scope, r#"some n in 12.58 satisfies n > 10.5"#, true);
}

#[test]
fn _0005() {
  let scope = FeelScope::default();
  let node = AstNode::Some(Box::new(AstNode::Name("n".into())), Box::new(AstNode::Name("n".into())));
  assert_eq!(
    r#"null(expected AST node QuantifiedContexts, actual AST node is Name(Name("n")))"#,
    crate::evaluate(&scope, &node).to_string()
  );
}

#[test]
fn _0006() {
  let scope = FeelScope::default();
  let node = AstNode::Some(Box::new(AstNode::QuantifiedContexts(vec![])), Box::new(AstNode::Name("n".into())));
  assert_eq!(
    r#"null(expected AST node Satisfies, actual AST node is QuantifiedContexts([]))"#,
    crate::evaluate(&scope, &node).to_string()
  );
}

#[test]
fn _0007() {
  let scope = &te_scope("{}");
  te_null(false, scope, r#"some element in 1 satisfies "not a boolean""#, "");
}

#[test]
fn _some_stops_at_the_first_decisive_value() {
  let scope = &te_scope("{}");
  te_null(false, scope, "some x in [null, true] satisfies x", "");
  te_bool(false, scope, "some x in [false, true, null] satisfies x", true);
  te_bool(false, scope, "some x in [1, 2], y in [2, 3] satisfies x = y", true);
  te_bool(false, scope, "some x in [] satisfies x", false);
}

#[test]
fn _0008() {
  // A body that removes the iteration context from the scope (no FEEL body does, an evaluator
  // supplied by a caller can) gives a null, and the scope is left as it was found.
  let scope = &te_scope("{}");
  let popping: rheon_feel::Evaluator = Box::new(|scope: &FeelScope| {
    scope.pop();
    Value::Boolean(false)
  });
  let mut evaluator = crate::SomeExpressionEvaluator::for_feel();
  evaluator.add_list("i".into(), Value::List(vec![Value::Boolean(true), Value::Boolean(false)]));
  let depth = scope.depth();
  assert_eq!(
    Value::Null(Some("internal error: the evaluation removed the iteration context of a `some` expression".to_string())),
    evaluator.evaluate(scope, &popping)
  );
  assert_eq!(depth, scope.depth());
}
