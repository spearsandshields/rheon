//! Tests of the name lookup in the parsing scope ([ParsingScope::contains], [ParsingScope::bound]):
//! against the flattened-key search of the frozen reference front end ([super::oracle]), which
//! builds the set of all flattened keys for every name token, and against an independent model of
//! the names in scope ([ReferenceScope]). The generators are shared with [super::differential].

use super::{differential, full_size, oracle};
use crate::context::ParsingContext;
use crate::index::list_item_context_type;
use crate::lalr::TokenType;
use crate::lexer::{Lexer, TokenValue};
use crate::parser::Parser;
use crate::scope::{ParsingScope, first_word};
use rheon_feel::context::FeelContext;
use rheon_feel::values::Value;
use rheon_feel::{FeelNumber, FeelScope, FeelType, Name};
use std::collections::{BTreeMap, HashSet};

/// Deterministic xorshift generator (no dependency, reproducible failures).
pub(super) struct Rng(pub(super) u64);

impl Rng {
  fn next(&mut self) -> u64 {
    let mut x = self.0;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    self.0 = x;
    x
  }

  pub(super) fn below(&mut self, n: usize) -> usize {
    (self.next() % n as u64) as usize
  }

  pub(super) fn chance(&mut self, percent: usize) -> bool {
    self.below(100) < percent
  }

  pub(super) fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
    &items[self.below(items.len())]
  }

  pub(super) fn pick_str(&mut self, items: &[&'static str]) -> &'static str {
    items[self.below(items.len())]
  }
}

pub(super) const OGHAM: char = '\u{1680}';

/// Keys of generated contexts: multi-word names, names with additional symbols, names containing
/// the composite separator, U+1680 (a name character that `trim` removes), and keywords.
pub(super) const KEYS: &[&str] = &[
  "a",
  "b",
  "a b",
  "a b c",
  "x",
  "y",
  "x.y",
  "x . y",
  "x  .  y",
  "p/q",
  "p-q",
  "o'k",
  "m+n",
  "m*n",
  "monthly income",
  "income",
  "k0",
  "k1",
  "k2",
  "item",
  "partial",
  "start",
  "year",
  "og\u{1680}ham",
  "a\u{1680}b",
  "Customer data",
  "data",
  "Person",
  "x y z",
  "in",
  "a.b.c",
  // keys ending with additional symbols: the start of the composite names the lexer can produce
  // (`a./ . b` from the parts `a` `.` `/` U+1680 `.` U+1680 `b`)
  "a./",
  "a..",
  "b.-",
  "x./",
  "/",
];

pub(super) fn number(n: i64) -> Value {
  Value::Number(FeelNumber::from(n))
}

fn context_type(rng: &mut Rng, depth: usize) -> FeelType {
  let mut entries = BTreeMap::new();
  for _ in 0..1 + rng.below(3) {
    let t = if depth < 2 && rng.chance(30) {
      if rng.chance(50) {
        context_type(rng, depth + 1)
      } else {
        FeelType::List(Box::new(context_type(rng, depth + 1)))
      }
    } else {
      FeelType::Number
    };
    entries.insert(Name::from(*rng.pick(KEYS)), t);
  }
  FeelType::Context(entries)
}

pub(super) fn context(rng: &mut Rng, depth: usize) -> FeelContext {
  let mut ctx = FeelContext::default();
  for _ in 0..rng.below(5) {
    ctx.set_entry(&Name::from(*rng.pick(KEYS)), value(rng, depth));
  }
  ctx
}

fn value(rng: &mut Rng, depth: usize) -> Value {
  if depth >= 3 {
    return number(1);
  }
  match rng.below(14) {
    0..=3 => number(rng.below(10) as i64),
    4 => Value::String("s".into()),
    5 => Value::Null(None),
    6 | 7 => Value::Context(context(rng, depth + 1)),
    8 => {
      // a list of contexts with the same keys (conformant), or with different keys
      let first = context(rng, depth + 1);
      let mut items = vec![Value::Context(first.clone())];
      for _ in 0..rng.below(3) {
        items.push(Value::Context(if rng.chance(70) { first.clone() } else { context(rng, depth + 1) }));
      }
      Value::List(items)
    }
    9 => Value::List(match rng.below(5) {
      0 => vec![],
      1 => vec![Value::Null(None), Value::Context(context(rng, depth + 1))],
      2 => vec![Value::Context(context(rng, depth + 1)), Value::Null(None)],
      3 => vec![number(1), Value::Context(context(rng, depth + 1))],
      _ => vec![Value::List(vec![Value::Context(context(rng, depth + 1))])],
    }),
    10 => {
      // contexts with the same keys, values of different types: not conformant
      let key = Name::from(*rng.pick(KEYS));
      let mut a = FeelContext::default();
      a.set_entry(&key, number(1));
      let mut b = FeelContext::default();
      b.set_entry(&key, if rng.chance(50) { Value::String("s".into()) } else { Value::Null(None) });
      Value::List(vec![Value::Context(a), Value::Context(b)])
    }
    11 => Value::FeelType(context_type(rng, depth)),
    12 => Value::FeelType(FeelType::List(Box::new(context_type(rng, depth)))),
    _ => Value::FeelType(FeelType::Number),
  }
}

/// A key as it can appear in an expression: words separated by random whitespace (including
/// U+1680), additional symbols with or without spaces around them.
pub(super) fn key_text(rng: &mut Rng, key: &str) -> String {
  let mut text = String::new();
  for ch in key.chars() {
    match ch {
      ' ' => text.push_str(rng.pick_str(&[" ", "  ", "\t", " \u{1680} "])),
      '.' | '/' | '-' | '\'' | '+' | '*' if rng.chance(30) => {
        text.push(' ');
        text.push(ch);
        text.push(' ');
      }
      _ => text.push(ch),
    }
  }
  text
}

fn expression(rng: &mut Rng, keys: &[String], depth: usize) -> String {
  let operand = |rng: &mut Rng| -> String {
    match rng.below(14) {
      0 => "1".to_string(),
      1 => "\"t\"".to_string(),
      2 => {
        // a composite name: U+1680-only name parts (empty after trimming) around the separator
        let a = rng.pick(keys).clone();
        let b = rng.pick(keys).clone();
        format!("{a}{OGHAM}.{OGHAM} {b}")
      }
      10 | 11 => {
        // a key followed by a random run of symbols, U+1680 and short names
        let mut text = rng.pick(keys).clone();
        for _ in 0..2 + rng.below(7) {
          text.push_str(rng.pick_str(&["\u{1680}", ".", "/", "-", "+", "'", "*", " ", "a", "b", "k1", "x", "\u{1680}.\u{1680}"]));
        }
        text
      }
      3 => {
        let key = rng_key(rng, keys);
        let text = key_text(rng, &key);
        format!("{text}{}", rng.pick_str(&[".year", ".start included", ".a b", " . x", ".k1"]))
      }
      _ => {
        let key = rng_key(rng, keys);
        key_text(rng, &key)
      }
    }
  };
  let mut text = operand(rng);
  for _ in 0..rng.below(4) {
    text.push_str(rng.pick_str(&[" + ", " - ", "*", " / ", ".", " . ", " and ", " = ", "-", "+", " ", "/"]));
    text.push_str(&operand(rng));
  }
  if depth < 2 {
    text = match rng.below(10) {
      0 => format!("function({}, {}) {text}", rng_key(rng, keys), rng_key(rng, keys)),
      1 => format!("for {} in [1, 2] return {text}", rng_key(rng, keys)),
      2 => format!("{{{}: 1, {}: {text}}}", rng_key(rng, keys), rng_key(rng, keys)),
      3 => format!("some {} in [1] satisfies {text}", rng_key(rng, keys)),
      4 => format!("({text}) + {}", expression(rng, keys, depth + 1)),
      5 => format!("{{r: {}, s: {text}}}.s", expression(rng, keys, depth + 1)),
      _ => text,
    };
  }
  text
}

fn rng_key(rng: &mut Rng, keys: &[String]) -> String {
  rng.pick(keys).clone()
}

/// The token stream of a lexer run without the parser (no scope changes); with `reference`, of the
/// frozen reference lexer in its flattened-key mode (it searches the set of all flattened keys,
/// built for every name token).
fn tokens(frames: &[FeelContext], input: &str, reference: bool) -> Vec<String> {
  let mut tokens = vec![];
  let mut push = |token: dsntk_common::Result<(i16, String, bool)>| match token {
    Ok((_, _, true)) => false,
    Ok((tt, tv, false)) => {
      tokens.push(format!("{tt} {tv}"));
      true
    }
    Err(e) => {
      tokens.push(format!("error {e}"));
      false
    }
  };
  if reference {
    let scope = oracle::scope::ParsingScope::new(frames);
    let mut lexer = oracle::lexer::Lexer::new(&scope, oracle::lalr::TokenType::StartExpression, input);
    lexer.set_oracle();
    for _ in 0..1000 {
      let token = lexer.next_token().map(|(tt, tv)| {
        let tt = tt as i16;
        (tt, format!("{tv:?}"), tt == 0 || matches!(tv, oracle::lexer::TokenValue::YyEof))
      });
      if !push(token) {
        break;
      }
    }
  } else {
    let scope = ParsingScope::new(frames);
    let mut lexer = Lexer::new(&scope, TokenType::StartExpression, input);
    for _ in 0..1000 {
      let token = lexer.next_token().map(|(tt, tv)| {
        let tt = tt as i16;
        (tt, format!("{tv:?}"), tt == 0 || matches!(tv, TokenValue::YyEof))
      });
      if !push(token) {
        break;
      }
    }
  }
  tokens
}

/// The AST (tree and Debug text) or the error of a parse; with `reference`, of the frozen
/// reference parser with its lexer in the flattened-key mode.
fn parse(frames: &[FeelContext], input: &str, reference: bool) -> String {
  let result = if reference {
    let scope = oracle::scope::ParsingScope::new(frames);
    oracle::parser::Parser::new(&scope, oracle::lalr::TokenType::StartExpression, input, false)
      .with_oracle_lexer()
      .parse()
  } else {
    let scope = ParsingScope::new(frames);
    Parser::new(&scope, TokenType::StartExpression, input, false).parse()
  };
  match result {
    Ok(node) => format!("{}\n{node:?}", node.trace()),
    Err(e) => format!("error {e}"),
  }
}

#[test]
fn name_lookup_equals_flattened_keys_search() {
  let cases = if full_size() { 4_000 } else { 300 };
  let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
  let mut parsed = 0;
  let mut composite = 0;
  for case in 0..cases {
    let frames: Vec<FeelContext> = (0..1 + rng.below(2)).map(|_| context(&mut rng, 0)).collect();
    let mut keys: Vec<String> = KEYS.iter().map(|k| k.to_string()).collect();
    keys.push("zz".to_string());
    let input = expression(&mut rng, &keys, 0);
    let actual_tokens = tokens(&frames, &input, false);
    let expected_tokens = tokens(&frames, &input, true);
    assert_eq!(expected_tokens, actual_tokens, "case {case}: tokens of {input:?} in {frames:?}");
    // names found as composite keys (the only names with a space after an additional symbol)
    composite += actual_tokens
      .iter()
      .filter(|t| t.contains("Name(Name(") && ["./. ", ".. ", ".-. ", "/. "].iter().any(|c| t.contains(c)))
      .count();
    // the parse: equal to the reference's (flattened-key search), or a registered fix
    let production = differential::run_production(&frames, &TokenType::StartExpression, &input);
    let reference = differential::run_reference_flattened(&frames, &input);
    differential::compare("expression", &frames, &input, &reference, &production);
    if matches!(production.outcome, differential::Outcome::Ast(_)) {
      parsed += 1;
    }
  }
  println!("name lookup: {parsed} of {cases} expressions parsed, {composite} composite names");
  // enough of the random expressions are valid FEEL for the AST comparison to matter
  assert!(parsed > cases / 4, "only {parsed} of {cases} expressions parsed");
  assert!(composite > cases / 200, "only {composite} composite names in {cases} expressions");
}

/// The names in scope by definition: every flattened key of every context on the stack (a key at
/// any depth, or a composite key `k1 . k2 . … . kn`), and the parsed names. An independent model
/// of the parsing scope, built and changed by the same operations.
struct ReferenceScope {
  stack: Vec<ReferenceTree>,
  names: HashSet<String>,
}

/// The keys of a context, each with the tree of the names nested in its value, if any.
#[derive(Default)]
struct ReferenceTree(BTreeMap<String, Option<ReferenceTree>>);

impl ReferenceTree {
  /// The tree of an input context.
  fn of_context(ctx: &FeelContext) -> Self {
    Self(ctx.iter().map(|(name, value)| (name.to_string(), Self::of_value(value))).collect())
  }

  /// The tree of the names nested in a value: those of a context, of a type ([Self::of_type]),
  /// and of a list whose type is a list of a context type (the type of its items).
  fn of_value(value: &Value) -> Option<Self> {
    match value {
      Value::Context(ctx) => Some(Self::of_context(ctx)),
      Value::FeelType(feel_type) => Self::of_type(feel_type),
      Value::List(_) => Self::of_type(&value.type_of()),
      _ => None,
    }
  }

  /// The tree of the names nested in a type: the entries of a context type, or of the item type
  /// of a list of a context type; the types of the entries are trees in the same way.
  fn of_type(feel_type: &FeelType) -> Option<Self> {
    match feel_type {
      FeelType::Context(entries) => Some(Self(entries.iter().map(|(name, feel_type)| (name.to_string(), Self::of_type(feel_type))).collect())),
      FeelType::List(item_type) if matches!(item_type.as_ref(), FeelType::Context(_)) => Self::of_type(item_type),
      _ => None,
    }
  }

  /// The keys at any depth.
  fn keys(&self, keys: &mut HashSet<String>) {
    for (key, nested) in &self.0 {
      keys.insert(key.clone());
      if let Some(nested) = nested {
        nested.keys(keys);
      }
    }
  }

  /// The flattened keys: every key, every flattened key of a nested tree, and every
  /// `key . flattened key` of the tree nested in the value of a key.
  fn flattened(&self) -> HashSet<String> {
    let mut flattened = HashSet::new();
    for (key, nested) in &self.0 {
      flattened.insert(key.clone());
      if let Some(nested) = nested {
        for nested_key in nested.flattened() {
          flattened.insert(format!("{key} . {nested_key}"));
          flattened.insert(nested_key);
        }
      }
    }
    flattened
  }
}

impl ReferenceScope {
  fn new(frames: &[FeelContext]) -> Self {
    Self {
      stack: frames.iter().map(ReferenceTree::of_context).collect(),
      names: HashSet::new(),
    }
  }

  /// The names in scope.
  fn flattened(&self) -> HashSet<String> {
    let mut flattened: HashSet<String> = self.stack.iter().flat_map(ReferenceTree::flattened).collect();
    flattened.extend(self.names.iter().cloned());
    flattened
  }

  /// The names in scope that are not composite keys: the keys at any depth and the parsed names.
  fn plain(&self) -> HashSet<String> {
    let mut plain = self.names.clone();
    for tree in &self.stack {
      tree.keys(&mut plain);
    }
    plain
  }
}

#[test]
fn scope_operations_equal_reference_scope() {
  let mut rng = Rng(0xD1B5_4A32_D192_ED03);
  for case in 0..if full_size() { 1_500 } else { 100 } {
    let frames: Vec<FeelContext> = (0..rng.below(3)).map(|_| context(&mut rng, 0)).collect();
    let scope = ParsingScope::new(&frames);
    let mut reference = ReferenceScope::new(&frames);
    for step in 0..12 {
      // unbalanced pops and names set without a pushed context included
      match rng.below(7) {
        0 => {
          scope.pop();
          reference.stack.pop();
        }
        1 => {
          scope.push_default();
          reference.stack.push(ReferenceTree::default());
        }
        6 => {
          // the same context on both sides (the conversion is deterministic)
          let pushed = context(&mut rng, 0);
          scope.push(ParsingContext::from(&pushed));
          reference.stack.push(ReferenceTree::of_context(&pushed));
        }
        2 | 3 => {
          let name = Name::from(*rng.pick(KEYS));
          scope.set_entry_name(name.clone());
          if let Some(top) = reference.stack.last_mut() {
            top.0.insert(name.to_string(), None);
          }
        }
        4 => {
          let name = Name::from(*rng.pick(KEYS));
          scope.add_parsed_name(name.clone());
          reference.names.insert(name.to_string());
        }
        _ => {
          let name = Name::from(*rng.pick(KEYS));
          let nested = context(&mut rng, 1);
          scope.set_context(name.clone(), ParsingContext::from(&nested));
          if let Some(top) = reference.stack.last_mut() {
            top.0.insert(name.to_string(), Some(ReferenceTree::of_context(&nested)));
          }
        }
      }
      let flattened = reference.flattened();
      let mut probes: Vec<String> = flattened.iter().cloned().collect();
      for key in KEYS {
        probes.push(key.to_string());
        probes.push(format!("{key} . {}", rng.pick(KEYS)));
        probes.push(format!("{} . {key} . {}", rng.pick(KEYS), rng.pick(KEYS)));
      }
      probes.push(String::new());
      for probe in &probes {
        assert_eq!(flattened.contains(probe), scope.contains(probe), "case {case} step {step}: contains({probe:?})");
        if !probe.contains(" . ") {
          assert_eq!(flattened.contains(probe), scope.contains_plain(probe), "case {case} step {step}: contains_plain({probe:?})");
        }
      }
      // the names `contains_plain` accepts, by first word, are within `group_bound`
      for member in reference.plain() {
        let word = first_word(&member);
        assert!(
          scope.group_bound(word, usize::MAX) >= member.len(),
          "case {case} step {step}: group_bound({word:?}) for {member:?}"
        );
      }
      let plain_max = flattened.iter().filter(|k| !k.contains(" . ")).map(String::len).max().unwrap_or(0);
      let all_max = flattened.iter().map(String::len).max().unwrap_or(0);
      assert!(scope.bound(false) >= plain_max, "case {case} step {step}: bound without composite keys");
      assert!(scope.bound(true) >= all_max, "case {case} step {step}: bound with composite keys");
    }
  }
}

/// A chain of nested contexts `k0: {k1: {… {leaf: 1}}}` of the given depth, at the top level and
/// under the key `a./` (so that the lexer can produce composite names into it), next to `x: 1`.
pub(super) fn chain(depth: usize) -> FeelContext {
  let mut ctx = FeelContext::default();
  ctx.set_entry(&Name::from("leaf"), number(1));
  for level in (0..depth).rev() {
    let mut parent = FeelContext::default();
    parent.set_entry(&Name::from(format!("k{level}").as_str()), Value::Context(ctx));
    ctx = parent;
  }
  let mut top = ctx.clone();
  top.set_entry(&Name::from("a./"), Value::Context(ctx));
  top.set_entry(&Name::from("x"), number(1));
  top
}

#[test]
fn composite_names_in_a_chain_equal_flattened_keys_search() {
  let frames = vec![chain(8)];
  for input in [
    format!("a./{OGHAM}.{OGHAM} k0 + 1"),
    format!("a./{OGHAM}.{OGHAM} k5 - x"),
    format!("a./{OGHAM}.{OGHAM} leaf"),
    format!("a./{OGHAM}.{OGHAM} k9"),
    format!("a./{OGHAM}.{OGHAM}x"),
    format!("k0 {OGHAM} . {OGHAM} k1 + 1"),
    "k0.k1.k2.k3.k4.k5.k6.k7.leaf".to_string(),
    "x + 1".to_string(),
  ] {
    assert_eq!(parse(&frames, &input, true), parse(&frames, &input, false), "{input:?}");
  }
  // the composite names do match (one name, no path)
  let trace = parse(&frames, &format!("a./{OGHAM}.{OGHAM} k5 - x"), false);
  assert!(trace.contains("`a./. k5`") && !trace.contains("Path"), "{trace}");
}

/// A chain of n levels `k0 . k1 . … . k(n-1)` has more than 2^n flattened keys; the lookup builds
/// none of them. A plain name needs no composite search (the index of the input context is built
/// once), and the composite search of a name into the chain visits a number of states that grows
/// linearly with the depth: 8 per level and per name token, as measured.
#[test]
fn deep_chain_of_distinct_keys_is_linear() {
  let expected = format!("{:?}", crate::parse_expression(&FeelScope::from(chain(0)), "x + 1", false).unwrap());
  for depth in [30, 60] {
    let scope = FeelScope::from(chain(depth));
    super::work::take();
    let node = crate::parse_expression(&scope, "x + 1", false).unwrap();
    let work = super::work::take();
    assert_eq!(expected, format!("{node:?}"));
    assert_eq!(None, work.get("composite states"), "depth {depth}: {work:?}");
    assert_eq!(Some(&1), work.get("index builds"), "depth {depth}: {work:?}");
    // a composite name into the chain, `depth` levels down
    let input = format!("a./{OGHAM}.{OGHAM} leaf + 1");
    let node = crate::parse_expression(&scope, &input, false).unwrap();
    let work = super::work::take();
    let trace = node.trace();
    assert!(trace.contains("`a./. leaf`") && !trace.contains("Path"), "{trace}");
    let states = work.get("composite states").copied().unwrap_or(0);
    assert!(states > 0 && states <= 8 * (depth + 1), "depth {depth}: {states} composite states");
  }
}

/// The item type of a list in an input context is a context type exactly when [Value::type_of] of
/// the list says so, for lists whose first item is not looked at further and for lists that are
/// typed.
#[test]
fn list_item_context_types() {
  let one = |key: &str| {
    let mut ctx = FeelContext::default();
    ctx.set_entry(&Name::from(key), number(1));
    Value::Context(ctx)
  };
  let context_type = FeelType::Context(BTreeMap::from([(Name::from("a"), FeelType::Number)]));
  for (list, has_context_items) in [
    (vec![one("a"), one("a")], true),
    (vec![Value::FeelType(context_type.clone()), Value::FeelType(context_type.clone())], true),
    (vec![one("a"), one("b")], false),
    (vec![one("a"), number(1)], false),
    (vec![number(1), one("a")], false),
    (vec![Value::Null(None), one("a")], false),
    (vec![Value::List(vec![one("a")])], false),
    (vec![], false),
  ] {
    let list = Value::List(list);
    let expected = match list.type_of() {
      FeelType::List(item_type) if matches!(*item_type, FeelType::Context(_)) => Some(*item_type),
      _ => None,
    };
    assert_eq!(has_context_items, expected.is_some(), "{list:?}");
    assert_eq!(expected, list_item_context_type(&list), "{list:?}");
  }
}
