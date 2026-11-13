use super::*;
use rheon_feel::{FeelType, scope};

#[test]
fn _0001() {
  let node = AstNode::FunctionDefinition(
    Box::new(AstNode::FeelType(FeelType::Boolean)),
    Box::new(AstNode::FunctionBody(Box::new(AstNode::Numeric("1".into(), "0".into(), '+', "".into())), false)),
  );
  assert_eq!(
    r#"null(expected formal parameters, actual value type is boolean)"#,
    crate::evaluate(&scope!(), &node).to_string()
  );
}

#[test]
fn _0002() {
  // A function returned by a function body closes over the names it uses as they are at the end
  // of that body (`closed_over` in invocation.rs): the `k` that `f` returns captures `a` = 100, so
  // `r` is 101. The function definition is shared with the entry `k` and copied before the
  // capture, so the stored `k` is unchanged and still has no `a`: `s` is null.
  let expression = "{k: function(y) y + a, f: function(a) k, r: f(100)(1), s: k(1)}";
  te_number(false, &scope!(), &format!("{expression}.r"), 101, 0);
  te_null(false, &scope!(), &format!("{expression}.s"), "context has no value for key 'a'");
}
