use super::super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_value(false, &scope!(), r#"context merge([{x:1}, {y:2}])"#, "{x:1, y:2}");
}

#[test]
fn _0002() {
  te_value(false, &scope!(), r#"context merge(contexts: [{x:1}, {y:2}])"#, "{x:1, y:2}");
}

#[test]
fn _0003() {
  te_null(
    false,
    &scope!(),
    r#"context merge(context: [{x:1}, {y:2}])"#,
    "expected the parameters of context merge(contexts): context, found the parameter name 'context'",
  );
}

#[test]
fn _0004() {
  te_null(
    false,
    &scope!(),
    r#"context merge([1,{}])"#,
    "[core::context merge] invalid argument type, expected context, actual type is number",
  );
}

#[test]
fn _0005() {
  te_null(
    false,
    &scope!(),
    r#"context merge(1)"#,
    "[core::context merge] invalid argument type, expected list<context>, actual type is number",
  );
}

#[test]
fn _0006() {
  // the domain is a list of contexts: the empty list merges to the empty context, as context([]) is
  // {}, and a null item is outside the domain like any other item that is no context
  te_value(false, &scope!(), r#"context merge([])"#, "{}");
  te_null(
    false,
    &scope!(),
    r#"context merge([{a: 1}, null])"#,
    "[core::context merge] invalid argument type, expected context, actual type is Null",
  );
}
