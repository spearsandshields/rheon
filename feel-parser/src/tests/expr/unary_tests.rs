use super::super::*;
use crate::token::TokenType::StartUnaryTests;
use dsntk_common::DsntkError;

#[test]
fn _0001() {
  let scope = scope!();
  accept(
    &scope,
    StartUnaryTests,
    r#"-"#,
    r#"
       Irrelevant
    "#,
    false,
  );
}

#[test]
fn _0002() {
  let scope = scope!();
  accept(
    &scope,
    StartUnaryTests,
    r#"1"#,
    r#"
       ExpressionList
       └─ Numeric
          └─ `1`
    "#,
    false,
  );
}

#[test]
fn _0003() {
  let scope = scope!();
  accept(
    &scope,
    StartUnaryTests,
    r#"1,2"#,
    r#"
       ExpressionList
       ├─ Numeric
       │  └─ `1`
       └─ Numeric
          └─ `2`
    "#,
    false,
  );
}

#[test]
fn _0004() {
  let scope = scope!();
  accept(
    &scope,
    StartUnaryTests,
    r#" true , false "#,
    r#"
       ExpressionList
       ├─ Boolean
       │  └─ `true`
       └─ Boolean
          └─ `false`
    "#,
    false,
  );
}

#[test]
fn _0005() {
  let scope = scope!();
  accept(
    &scope,
    StartUnaryTests,
    r#" date("2021-10-01") , time("15:23") "#,
    r#"
       ExpressionList
       ├─ FunctionInvocation
       │  ├─ Name
       │  │  └─ `date`
       │  └─ PositionalParameters
       │     └─ String
       │        └─ `2021-10-01`
       └─ FunctionInvocation
          ├─ Name
          │  └─ `time`
          └─ PositionalParameters
             └─ String
                └─ `15:23`
    "#,
    false,
  );
}

#[test]
fn _0006() {
  let scope = scope!();
  accept(
    &scope,
    StartUnaryTests,
    r#"1,2,3,4"#,
    r#"
       ExpressionList
       ├─ Numeric
       │  └─ `1`
       ├─ Numeric
       │  └─ `2`
       ├─ Numeric
       │  └─ `3`
       └─ Numeric
          └─ `4`
    "#,
    false,
  );
}

#[test]
fn _0007() {
  let scope = scope!();
  accept(
    &scope,
    StartUnaryTests,
    r#"not (1)"#,
    r#"
       NegatedList
       └─ Numeric
          └─ `1`
    "#,
    false,
  );
}

#[test]
fn _0008() {
  let scope = scope!();
  accept(
    &scope,
    StartUnaryTests,
    r#"not(1,2)"#,
    r#"
       NegatedList
       ├─ Numeric
       │  └─ `1`
       └─ Numeric
          └─ `2`
    "#,
    false,
  );
}

#[test]
fn _0009() {
  let scope = scope!();
  accept(
    &scope,
    StartUnaryTests,
    r#" not ( 1 , 2 , 3 , 4 ) "#,
    r#"
       NegatedList
       ├─ Numeric
       │  └─ `1`
       ├─ Numeric
       │  └─ `2`
       ├─ Numeric
       │  └─ `3`
       └─ Numeric
          └─ `4`
    "#,
    false,
  );
}

#[test]
fn _0010() {
  let scope = scope!();
  assert_eq!(
    Err(DsntkError::new(r#"ParserError"#, r#"syntax error: (1,2,3,4)"#)),
    Parser::new(&scope, StartUnaryTests, "(1,2,3,4)").parse()
  );
}

#[test]
fn _0011() {
  let scope = scope!();
  accept(
    &scope,
    StartUnaryTests,
    r#" < today() "#,
    r#"
       ExpressionList
       └─ UnaryLt
          └─ FunctionInvocation
             ├─ Name
             │  └─ `today`
             └─ PositionalParameters
                └─ (empty)
    "#,
    false,
  );
}

#[test]
fn _0012() {
  let scope = scope!();
  accept(
    &scope,
    StartUnaryTests,
    r#" = today() "#,
    r#"
       ExpressionList
       └─ UnaryEq
          └─ FunctionInvocation
             ├─ Name
             │  └─ `today`
             └─ PositionalParameters
                └─ (empty)
    "#,
    false,
  );
}

#[test]
fn _0013() {
  let scope = scope!();
  accept(
    &scope,
    StartUnaryTests,
    r#" > today() "#,
    r#"
       ExpressionList
       └─ UnaryGt
          └─ FunctionInvocation
             ├─ Name
             │  └─ `today`
             └─ PositionalParameters
                └─ (empty)
    "#,
    false,
  );
}

#[test]
fn _0014() {
  let scope = scope!();
  accept(
    &scope,
    StartUnaryTests,
    r#" != today() "#,
    r#"
       ExpressionList
       └─ UnaryNe
          └─ FunctionInvocation
             ├─ Name
             │  └─ `today`
             └─ PositionalParameters
                └─ (empty)
    "#,
    false,
  );
}

#[test]
fn _0015() {
  let scope = scope!();
  accept(
    &scope,
    StartUnaryTests,
    r#" >= today() "#,
    r#"
       ExpressionList
       └─ UnaryGe
          └─ FunctionInvocation
             ├─ Name
             │  └─ `today`
             └─ PositionalParameters
                └─ (empty)
    "#,
    false,
  );
}

#[test]
fn _0016() {
  let scope = scope!();
  accept(
    &scope,
    StartUnaryTests,
    r#" <= today() "#,
    r#"
       ExpressionList
       └─ UnaryLe
          └─ FunctionInvocation
             ├─ Name
             │  └─ `today`
             └─ PositionalParameters
                └─ (empty)
    "#,
    false,
  );
}

/// The endpoint of a unary comparison takes the arithmetic and the `in` after it; `and`, `or`,
/// a comparison and `between` end it.
#[test]
fn unary_comparison_endpoint_binds_like_a_comparison() {
  let scope = scope!();
  for name in ["a", "b", "x"] {
    scope.set_entry_name(name.into());
  }
  let tree = |start: TokenType, input: &str| match Parser::new(&scope, start, input).parse() {
    Ok(node) => node.trace(),
    Err(reason) => reason.to_string(),
  };
  // the same tree as with the endpoint in parentheses
  for (input, with_parentheses) in [
    ("> a + b", "> (a + b)"),
    ("< a * 2 - 1", "< (a * 2 - 1)"),
    (">= a ** 2", ">= (a ** 2)"),
    ("<= -a", "<= (-a)"),
    ("= a + 1", "= (a + 1)"),
    ("!= a / b", "!= (a / b)"),
    ("> a.b + 1", "> (a.b + 1)"),
    ("> a + 1, < b - 1", "> (a + 1), < (b - 1)"),
    ("not(> a + b)", "not(> (a + b))"),
  ] {
    assert_eq!(tree(StartUnaryTests, with_parentheses), tree(StartUnaryTests, input), "{input}");
    assert!(!tree(StartUnaryTests, input).contains("syntax error"), "{input}");
  }
  // lower than a comparison: not part of the endpoint
  assert_eq!(tree(StartExpression, "(> a) and b"), tree(StartExpression, "> a and b"));
  assert_eq!(tree(StartExpression, "(< a) or (> b)"), tree(StartExpression, "< a or > b"));
  // unchanged: literal, name and path endpoints
  for input in ["> 5", "< a", ">= a.b", "= \"x\"", "!= null"] {
    assert!(!tree(StartUnaryTests, input).contains("syntax error"), "{input}");
  }
  // a comparison, `between` or `in` after the endpoint ends it: the unary comparison is its operand
  assert_eq!(tree(StartExpression, "(> x) in [1, 2]"), tree(StartExpression, "> x in [1, 2]"));
  assert_eq!(tree(StartExpression, "(> a) < b"), tree(StartExpression, "> a < b"));
  assert_eq!(tree(StartExpression, "(<= 10) = (null..10]"), tree(StartExpression, "<= 10 = (null..10]"));
  assert_eq!(tree(StartExpression, "(> a) between 1 and 2"), tree(StartExpression, "> a between 1 and 2"));
  for input in ["> a < b", "<= 10 = (null..10]", "> a between 1 and 2"] {
    assert!(!tree(StartExpression, input).contains("syntax error"), "{input}");
  }
}
