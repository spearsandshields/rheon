//! The places of a positioned parse over the expressions of the identity request sets
//! (`tools/identity/requests`): every expression that parses has a place for its nodes, the
//! tree is that of the strict parse, and a place is where its node is written (a string at its
//! quotes, a context at its braces, an `if` at its keyword, a name at its text). Native only:
//! the request sets are read from the work tree.
#![cfg(not(target_arch = "wasm32"))]

use rheon_feel::FeelScope;
use rheon_feel_parser::{AstNode, StartRule, parse_positioned, parse_start};
use serde_json::Value as Json;
use std::collections::BTreeSet;
use std::path::Path;

#[path = "shared/big_stack.rs"]
mod big_stack;

/// The expressions of `value` with their dialects, each once.
fn collect(value: &Json, found: &mut BTreeSet<(String, bool)>) {
  match value {
    Json::Array(items) => items.iter().for_each(|item| collect(item, found)),
    Json::Object(fields) => {
      if let Some(Json::String(expression)) = fields.get("expression") {
        found.insert((expression.clone(), fields.get("dialect").is_some_and(|dialect| dialect == "unaryTests")));
      }
      fields.values().for_each(|inner| collect(inner, found));
    }
    _ => {}
  }
}

/// The letters and digits of `text`: how a name is compared with the text at its place, where
/// its parts can be written with any white space of FEEL between them.
fn dense(text: &str) -> String {
  text.chars().filter(|ch| ch.is_alphanumeric()).collect()
}

#[test]
fn the_nodes_of_the_request_sets_have_their_places() {
  big_stack::with_big_stack(|| {
    let sets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/identity/requests");
    let mut expressions = BTreeSet::new();
    for entry in std::fs::read_dir(&sets).expect("the request sets") {
      let file = entry.unwrap().path();
      if file.extension().is_some_and(|extension| extension == "json")
        && let Ok(read) = serde_json::from_str::<Json>(&std::fs::read_to_string(&file).unwrap())
      {
        collect(&read, &mut expressions);
      }
    }
    let scope = FeelScope::default();
    let (mut trees, mut placed, mut names) = (0usize, 0usize, 0usize);
    for (text, unary_tests) in &expressions {
      let rule = if *unary_tests { StartRule::UnaryTests } else { StartRule::Expression };
      let strict = parse_start(&scope, rule, text);
      let positioned = parse_positioned(&scope, rule, text);
      let (strict, positioned) = match (strict, positioned) {
        (Err(strict), Err(positioned)) => {
          assert_eq!(strict.to_string(), positioned.to_string(), "{text:?}");
          continue;
        }
        (Ok(strict), Ok(positioned)) => (strict, positioned),
        (strict, positioned) => panic!("{text:?}: strict {:?}, positioned {:?}", strict.is_ok(), positioned.is_ok()),
      };
      assert!(strict == *positioned.tree(), "{text:?}: another tree");
      assert!(positioned.complete(), "{text:?}: the records of the parser do not line up with the tree");
      trees += 1;
      placed += positioned.len();
      let mut stack = vec![positioned.tree()];
      while let Some(node) = stack.pop() {
        node.for_each_child(|child| stack.push(child));
        let Some((start, end)) = positioned.place(node) else {
          continue;
        };
        let written = &text[start..end];
        let fits = match node {
          AstNode::String(_) => written.starts_with('"') && written.ends_with('"'),
          AstNode::Context(_) => written.starts_with('{') && written.ends_with('}'),
          AstNode::List(_) => written.starts_with('[') && written.ends_with(']'),
          AstNode::If(..) => written.starts_with("if"),
          AstNode::For(..) => written.starts_with("for"),
          AstNode::Some(..) => written.starts_with("some"),
          AstNode::Every(..) => written.starts_with("every"),
          AstNode::FunctionDefinition(..) => written.starts_with("function"),
          AstNode::Boolean(value) => written == if *value { "true" } else { "false" },
          AstNode::At(_) => written.starts_with('@'),
          AstNode::Numeric(..) => written.starts_with(|ch: char| ch.is_ascii_digit() || ch == '.'),
          AstNode::Neg(_) => written.starts_with('-'),
          AstNode::FunctionInvocation(..) => written.ends_with(')'),
          AstNode::Filter(..) => written.ends_with(']'),
          AstNode::Name(name) => {
            names += 1;
            dense(written) == dense(&name.to_string())
          }
          _ => !written.is_empty() || matches!(node, AstNode::Null),
        };
        assert!(fits, "{text:?}: the place {start}..{end} ({written:?}) of {node:?}");
      }
    }
    println!("{} expressions, {trees} trees, {placed} nodes with a place, {names} names", expressions.len());
    assert!(trees > 6_500 && placed > 60_000, "{trees} trees, {placed} nodes");
  });
}
