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

/// A function body reads a name from the nearest scope around the function that has it (DMN
/// 1.6 clause 10.3.2.11: the contexts of a scope are looked through from the first, the
/// innermost, to the last): a parameter, a variable of a loop or the item of a filter that the
/// function is written in before an entry of a context literal further out; an entry of a
/// context literal, also one written later, before a name further out.
#[test]
fn a_function_reads_a_name_from_the_nearest_scope_around_it() {
  for (text, expected) in [
    ("{x: \"s\", f: function(x) function() x, r: f(1)()}.r", "1"),
    ("{x: \"s\", f: function(x) {g: function() x}.g(), r: f(1)}.r", "1"),
    ("{x: \"s\", f: function(x) function() function() x, r: f(1)()()}.r", "1"),
    ("{g: function() 1, f: function(g) function() g, r: f(2)()}.r", "2"),
    ("{x: {y: \"s\"}, f: function(x) function() x.y, r: f({y: 1})()}.r", "1"),
    ("{k: \"s\", r: for k in [1] return (function() k)()}.r", "[1]"),
    ("{k: \"s\", fs: for k in [1, 2] return function() k, r: for g in fs return g()}.r", "[1, 2]"),
    ("{k: \"s\", r: some k in [1] satisfies (function() k)() = 1}.r", "true"),
    ("{a: 1, r: [{a: 5}, {a: 6}][(function() a)() = 5]}.r", "[{a: 5}]"),
    ("{item: \"s\", r: [1, 2][(function() item)() = 2]}.r", "[2]"),
    ("{x: \"s\", f: function(x) [1][(function() x)() = 1], r: f(1)}.r", "[1]"),
    ("{x: \"s\", f: function(x) {x: 2, g: function() x}.g(), r: f(1)}.r", "2"),
    ("{f: function(x) {x: \"lit\", g: function() function() x}.g()(), r: f(1)}.r", "\"lit\""),
    ("{x: \"s\", f: function() function() x, r: f()()}.r", "\"s\""),
    ("{a: 1, r: [{b: 5}, {b: 6}][(function() a)() = 1]}.r", "[{b: 5}, {b: 6}]"),
    ("{f: function() k, r: f(), k: 10}.r", "10"),
    ("{mk: {f: function() g(), g: function() 7}.f, r: mk()}.r", "7"),
    ("{x: 1, f: function(a) function(b) a + b + x, r: f(10)(100)}.r", "111"),
  ] {
    te_value(false, &scope!(), text, expected);
  }
}
