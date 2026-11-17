use super::*;
use rheon_feel::{FeelType, scope};

#[test]
fn _0001() {
  let node = AstNode::FunctionDefinition(
    Box::new(AstNode::FeelType(FeelType::Boolean)),
    Box::new(AstNode::FunctionBody(Box::new(AstNode::Numeric("1".into(), "0".into(), '+', "".into())), false)),
  );
  assert_eq!(
    r#"null(malformed expression tree: unexpected FunctionDefinition node)"#,
    crate::tests::eval(&scope!(), &node).to_string()
  );
}

#[test]
fn _0002() {
  // FEEL functions are lexical closures (DMN 1.6 §10.3.2.13.2): `k` sees the names in scope
  // where it is written, where no `a` is. That `f`, which returns it, has a parameter `a` gives
  // `k` none: `r` is null, as `s` is.
  let expression = "{k: function(y) y + a, f: function(a) k, r: f(100)(1), s: k(1)}";
  te_null(
    false,
    &scope!(),
    &format!("{expression}.r"),
    "[addition] incompatible types: 1 + null(context has no value for key 'a')",
  );
  te_null(
    false,
    &scope!(),
    &format!("{expression}.s"),
    "[addition] incompatible types: 1 + null(context has no value for key 'a')",
  );
}
