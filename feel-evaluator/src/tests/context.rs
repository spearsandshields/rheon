use super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  let input = r#"
  {
    Full Name: "John Doe",
    Full Name: "John Travis"
  }
  "#;
  let node = rheon_feel_parser::parse_context(&scope!(), input).unwrap();
  assert_eq!(r#"null(duplicated context entry key: Full Name)"#, crate::tests::eval(&scope!(), &node).to_string());
}

#[test]
fn _0002() {
  let input = r#"
  {
    "Full Name": "John Doe",
    "Address": "Atlanta"
  }
  "#;
  let node = rheon_feel_parser::parse_context(&scope!(), input).unwrap();
  assert_eq!(r#"{Full Name: "John Doe", Address: "Atlanta"}"#, crate::tests::eval(&scope!(), &node).to_string());
}

#[test]
fn _0003() {
  let node = AstNode::Context(vec![AstNode::String("key".to_string())]);
  assert_eq!(
    r#"null(malformed expression tree: unexpected Context node)"#,
    crate::tests::eval(&scope!(), &node).to_string()
  );
}

#[test]
fn _0004() {
  let node = AstNode::Context(vec![AstNode::ContextEntry(
    Box::new(AstNode::String("key".to_string())),
    Box::new(AstNode::String("value".to_string())),
  )]);
  assert_eq!(
    r#"null(malformed expression tree: unexpected Context node)"#,
    crate::tests::eval(&scope!(), &node).to_string()
  );
}

#[test]
fn _0005() {
  let input = r#" { convert: function(x: context <age: number, name: stringa>) x } "#;
  let node = rheon_feel_parser::parse_context(&scope!(), input).unwrap();
  assert_eq!(
    r#"{convert: null(expected type of the formal parameter)}"#,
    crate::tests::eval(&scope!(), &node).to_string()
  );
}

#[test]
fn _0006() {
  let node = AstNode::ContextType(vec![AstNode::ContextTypeEntry(
    Box::new(AstNode::String("key".to_string())),
    Box::new(AstNode::String("value".to_string())),
  )]);
  assert_eq!(r#"null(expected a name in context type entry)"#, crate::tests::eval(&scope!(), &node).to_string());
}

#[test]
fn _0007() {
  let node = AstNode::ContextType(vec![AstNode::String("key".to_string())]);
  assert_eq!(
    r#"null(expected context type entry, actual value type is string)"#,
    crate::tests::eval(&scope!(), &node).to_string()
  );
}

/// A context is shown in the order its entries are written in, whatever order they are
/// evaluated in (an entry that needs a later one is evaluated after it), and the built-in
/// functions that make a context from another keep its order: a new entry comes last, an
/// entry that is replaced keeps its place.
#[test]
fn a_context_is_shown_in_the_order_it_is_written_in() {
  let scope = &scope!();
  let shown = |text: &str| {
    let node = rheon_feel_parser::parse_expression(scope, text).unwrap_or_else(|reason| panic!("{text}: {reason}"));
    crate::tests::eval(scope, &node).to_string()
  };
  for (text, expected) in [
    ("{z: 1, a: 2, m: 3}", "{z: 1, a: 2, m: 3}"),
    ("{b: a + 1, a: 1}", "{b: 2, a: 1}"),
    ("{c: b + 1, b: a + 1, a: 1, d: c}", "{c: 3, b: 2, a: 1, d: 3}"),
    ("{z: {y: 1, x: 2}, a: [{q: 1, p: 2}]}", "{z: {y: 1, x: 2}, a: [{q: 1, p: 2}]}"),
    (r#"string({z: 1, a: 2})"#, r#""{z: 1, a: 2}""#),
    ("get entries({z: 1, a: 2})", r#"[{key: "z", value: 1}, {key: "a", value: 2}]"#),
    (r#"context put({z: 1, a: 2}, "m", 3)"#, "{z: 1, a: 2, m: 3}"),
    (r#"context put({z: 1, a: 2}, "z", 3)"#, "{z: 3, a: 2}"),
    (r#"context put({z: 1, a: {y: 1, b: 2}}, ["a", "c"], 3)"#, "{z: 1, a: {y: 1, b: 2, c: 3}}"),
    ("context merge([{z: 1, a: 2}, {m: 3, z: 4}])", "{z: 4, a: 2, m: 3}"),
    (r#"context([{key: "z", value: 1}, {key: "a", value: 2}])"#, "{z: 1, a: 2}"),
    ("for x in [1, 2] return {z: x, a: x + 1}", "[{z: 1, a: 2}, {z: 2, a: 3}]"),
    ("[{z: 1, a: 2}, {a: 3, z: 4}][z > 0]", "[{z: 1, a: 2}, {a: 3, z: 4}]"),
    ("{f: function(x) {z: x, a: x}, r: f(1)}.r", "{z: 1, a: 1}"),
    // the order is no part of the value
    ("{z: 1, a: 2} = {a: 2, z: 1}", "true"),
    ("distinct values([{z: 1, a: 2}, {a: 2, z: 1}])", "[{z: 1, a: 2}]"),
    ("{z: 1, a: 2} instance of context<a: number, z: number>", "true"),
  ] {
    assert_eq!(expected, shown(text), "{text}");
  }
}

/// An entry reads a later entry whose key is a name of several tokens (with a keyword or a
/// symbol in it) wherever the context literal stands in the text: as the value of an entry of
/// another literal, after another literal, in a list, in a function. The keys of a literal are
/// names in scope before its entries are parsed (DMN 1.6 clause 10.3.2.11.1), and they are the
/// keys of that literal, not of the first literal of the text.
#[test]
fn an_entry_reads_a_later_key_of_several_tokens_wherever_the_literal_stands() {
  let scope = &scope!();
  let shown = |text: &str| {
    let node = rheon_feel_parser::parse_expression(scope, text).unwrap_or_else(|reason| panic!("{text}: {reason}"));
    crate::tests::eval(scope, &node).to_string()
  };
  for (text, expected) in [
    ("{x: Profit and Loss + 1, Profit and Loss: 2}.x", "3"),
    ("{r: {x: Profit and Loss + 1, Profit and Loss: 2}.x}.r", "3"),
    ("{r: {x: a + b + 1, a + b: 2}.x}.r", "3"),
    ("{r: {x: f(), f: function() Profit and Loss, Profit and Loss: 2}.x}.r", "2"),
    ("[{a: 1}.a, {x: Profit and Loss + 1, Profit and Loss: 2}.x]", "[1, 3]"),
    ("{a: 1}.a + {x: Profit and Loss + 1, Profit and Loss: 2}.x", "4"),
    ("{f: function(n) {x: n or more + n, n or more: 5}.x, r: f(1)}.r", "6"),
    ("{r: {s: {x: a - b, a - b: 7}.x}.s}.r", "7"),
    // a key of one literal is no name in another: `a + b` is a sum there
    ("[{a + b: 9}, {a: 1, b: 2, x: a + b}.x][2]", "3"),
    ("{a + b: 9, r: {a: 1, b: 2, x: a + b}.x}.r", "9"),
  ] {
    assert_eq!(expected, shown(text), "{text}");
  }
}
