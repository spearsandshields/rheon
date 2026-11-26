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
    crate::tests::eval(&scope, &node).to_string()
  );
}

#[test]
fn _0006() {
  let scope = FeelScope::default();
  let node = AstNode::Some(Box::new(AstNode::QuantifiedContexts(vec![])), Box::new(AstNode::Name("n".into())));
  assert_eq!(
    r#"null(expected AST node Satisfies, actual AST node is Name(Name("n")))"#,
    crate::tests::eval(&scope, &node).to_string()
  );
}

#[test]
fn _0007() {
  let scope = &te_scope("{}");
  te_null(false, scope, r#"some element in 1 satisfies "not a boolean""#, "");
}

#[test]
fn _0008() {
  // the or of the values (Table 51): a true decides it wherever it is, a null does not
  let scope = &te_scope("{}");
  te_bool(false, scope, "some x in [null, true] satisfies x", true);
  te_null(false, scope, "some x in [null, false] satisfies x", "");
  te_bool(false, scope, "every x in [true, null, false] satisfies x", false);
  te_null(false, scope, "every x in [true, null] satisfies x", "");
  te_bool(false, scope, "some x in [false, true, null] satisfies x", true);
  te_bool(false, scope, "some x in [1, 2], y in [2, 3] satisfies x = y", true);
  te_bool(false, scope, "some x in [] satisfies x", false);
}

#[test]
fn _0009() {
  // A body that removes the iteration context from the scope (no FEEL body does, an evaluator
  // supplied by a caller can) gives a null, and the scope is left as it was found.
  let scope = &te_scope("{}");
  let popping: rheon_feel::Evaluator = Box::new(|scope: &FeelScope| {
    scope.pop();
    Value::Boolean(false)
  });
  let booleans: crate::Domain = Box::new(|_: &FeelScope| Ok(crate::Sequence::of(Value::List(vec![Value::Boolean(true), Value::Boolean(false)].into()))));
  let evaluator = crate::Iteration::new(vec![("i".into(), booleans)]);
  let depth = scope.depth();
  assert_eq!(
    Value::Null(Some("internal error: the evaluation removed the iteration context of a `some` expression".to_string())),
    evaluator.some(scope, &popping)
  );
  assert_eq!(depth, scope.depth());
}

/// DMN 1.6 §10.3.2.15 defines the iteration of `some` as that of `for`: a context is evaluated
/// with the variables before it bound, and a context without elements contributes no iteration.
#[test]
fn a_quantified_context_sees_the_variables_bound_before_it() {
  te_errors("some x in [[1,2],[3]], y in x satisfies y = 3", "true", &[]);
  te_errors("some x in [[],[1]], y in x satisfies y = 1", "true", &[]);
  te_errors("some x in [], y in [1/0] satisfies true", "false", &[]);
}

#[test]
fn an_interval_is_an_iteration_context() {
  // DMN 1.7 Beta 1, grammar rule 46: some and every take the iteration contexts of for
  let scope = &te_scope("{}");
  te_bool(false, scope, "some i in 1..3 satisfies i > 2", true);
  te_bool(false, scope, "every i in 1..3 satisfies i > 0", true);
  te_bool(false, scope, "every i in 3..1 satisfies i > 1", false);
  te_bool(false, scope, "some i in 1..2, j in i..3 satisfies i + j = 5", true);
  te_bool(false, scope, r#"some d in @"2024-02-28"..@"2024-03-01" satisfies day of week(d) = "Friday""#, true);
  te_null(
    false,
    scope,
    "some i in 1..2.5 satisfies true",
    "<FeelEvaluatorError> interval a..b expected two integers or two dates, found 1 and 2.5",
  );
}
