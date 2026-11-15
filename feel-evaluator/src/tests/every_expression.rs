use super::*;

#[test]
fn _0001() {
  let scope = &te_scope("{}");
  te_bool(false, scope, r#"every n in [1,2,3] satisfies n > 0.999"#, true);
}

#[test]
fn _0002() {
  let scope = &te_scope("{}");
  te_bool(false, scope, r#"every n in [1,2,3] satisfies n > 1.1"#, false);
}

#[test]
fn _0003() {
  let scope = &te_scope("{}");
  te_bool(false, scope, r#"every n in 1 satisfies n > 1.1"#, false);
}

#[test]
fn _0004() {
  let scope = &te_scope("{}");
  te_bool(false, scope, r#"every n in 1.11 satisfies n > 1.1"#, true);
}

#[test]
fn _0005() {
  let scope = FeelScope::default();
  let node = AstNode::Every(Box::new(AstNode::Name("n".into())), Box::new(AstNode::Name("n".into())));
  assert_eq!(
    r#"null(expected AST node QuantifiedContexts, actual AST node is Name(Name("n")))"#,
    crate::tests::eval(&scope, &node).to_string()
  );
}

#[test]
fn _0006() {
  let scope = FeelScope::default();
  let node = AstNode::Every(Box::new(AstNode::QuantifiedContexts(vec![])), Box::new(AstNode::Name("n".into())));
  assert_eq!(
    r#"null(expected AST node Satisfies, actual AST node is Name(Name("n")))"#,
    crate::tests::eval(&scope, &node).to_string()
  );
}

#[test]
fn _0007() {
  let scope = &te_scope("{}");
  te_null(false, scope, r#"every element in 1 satisfies "not a boolean""#, "");
}

#[test]
fn _0008() {
  // the first `false` or non-boolean value decides the result
  let scope = &te_scope("{}");
  te_null(false, scope, "every x in [true, null] satisfies x", "");
  te_bool(false, scope, "every x in [true, false, null] satisfies x", false);
  te_bool(false, scope, "every x in [1, 2], y in [2, 3] satisfies x < y + 2", true);
  te_bool(false, scope, "every x in [] satisfies x", true);
}

#[test]
fn _0009() {
  // A body that removes the iteration context from the scope (no FEEL body does, an evaluator
  // supplied by a caller can) gives a null, and the scope is left as it was found.
  let scope = &te_scope("{}");
  let popping: rheon_feel::Evaluator = Box::new(|scope: &FeelScope| {
    scope.pop();
    Value::Boolean(true)
  });
  let booleans: crate::Domain = Box::new(|_: &FeelScope| Ok(crate::Sequence::of(Value::List(vec![Value::Boolean(true), Value::Boolean(false)].into()))));
  let evaluator = crate::Iteration::for_feel(vec![("i".into(), booleans)]);
  let depth = scope.depth();
  assert_eq!(
    Value::Null(Some("internal error: the evaluation removed the iteration context of a `every` expression".to_string())),
    evaluator.every(scope, &popping)
  );
  assert_eq!(depth, scope.depth());
}

/// DMN 1.6 §10.3.2.15 defines the iteration of `every` as that of `for`: a context is evaluated
/// with the variables before it bound, and a context without elements contributes no iteration.
#[test]
fn a_quantified_context_sees_the_variables_bound_before_it() {
  te_errors("every x in [[1,2],[3]], y in x satisfies y > 0", "true", &[]);
  te_errors("every x in [[1,2],[-3]], y in x satisfies y > 0", "false", &[]);
  te_errors("every x in [[],[1]], y in x satisfies y > 0", "true", &[]);
}
