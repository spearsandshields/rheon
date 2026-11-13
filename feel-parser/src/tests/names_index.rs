//! Differential tests of the name lookup in the parsing scope ([ParsingScope::contains],
//! [ParsingScope::bound]) against the lexer's search before the prototype (the set of all flattened
//! keys built for every name token: the frozen prototype's oracle mode, [oracle::lexer::Lexer::set_oracle]),
//! and against the previous parsing scope. The generators are shared with [super::differential].

use super::oracle;
use crate::context::ParsingContext;
use crate::lalr::TokenType;
use crate::lexer::{Lexer, TokenValue};
use crate::parser::Parser;
use crate::scope::{ParsingScope, first_word};
use rheon_feel::bip::BUILT_IN_PROPERTY_NAMES;
use rheon_feel::context::FeelContext;
use rheon_feel::values::Value;
use rheon_feel::{FeelNumber, FeelScope, FeelType, Name};
use std::collections::{BTreeMap, HashSet};
use std::time::Instant;

/// Deterministic xorshift generator (no dependency, reproducible failures).
pub(crate) struct Rng(pub(crate) u64);

impl Rng {
  pub(crate) fn next(&mut self) -> u64 {
    let mut x = self.0;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    self.0 = x;
    x
  }

  pub(crate) fn below(&mut self, n: usize) -> usize {
    (self.next() % n as u64) as usize
  }

  pub(crate) fn chance(&mut self, percent: usize) -> bool {
    self.below(100) < percent
  }

  pub(crate) fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
    &items[self.below(items.len())]
  }

  pub(crate) fn pick_str(&mut self, items: &[&'static str]) -> &'static str {
    items[self.below(items.len())]
  }
}

pub(crate) const OGHAM: char = '\u{1680}';

/// Keys of generated contexts: multi-word names, names with additional symbols, names containing
/// the composite separator, U+1680 (a name character that `trim` removes), and keywords.
pub(crate) const KEYS: &[&str] = &[
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

pub(crate) fn number(n: i64) -> Value {
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

pub(crate) fn context(rng: &mut Rng, depth: usize) -> FeelContext {
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
    4 => Value::String("s".to_string()),
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
      b.set_entry(&key, if rng.chance(50) { Value::String("s".to_string()) } else { Value::Null(None) });
      Value::List(vec![Value::Context(a), Value::Context(b)])
    }
    11 => Value::FeelType(context_type(rng, depth)),
    12 => Value::FeelType(FeelType::List(Box::new(context_type(rng, depth)))),
    _ => Value::FeelType(FeelType::Number),
  }
}

/// A key as it can appear in an expression: words separated by random whitespace (including
/// U+1680), additional symbols with or without spaces around them.
pub(crate) fn key_text(rng: &mut Rng, key: &str) -> String {
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

pub(crate) fn expression(rng: &mut Rng, keys: &[String], depth: usize) -> String {
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

/// The token stream of a lexer run without the parser (no scope changes); with `oracle`, the frozen
/// prototype's lexer searching the set of all flattened keys.
fn tokens(frames: &[FeelContext], input: &str, oracle: bool) -> Vec<String> {
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
  if oracle {
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

fn parse(frames: &[FeelContext], input: &str, oracle: bool) -> String {
  let result = if oracle {
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
  let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
  let mut parsed = 0;
  let mut composite = 0;
  for case in 0..4000 {
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
    // the old lexer panics when an iteration variable's name parts start with `in` (`for in+1 in
    // [1] return 1`: `consumed_positions[index - 1]` underflows in the `till_in` tweak); now the
    // `in` is the keyword, and a syntax error (see differential::registered_difference)
    let actual = parse(&frames, &input, false);
    let expected = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| parse(&frames, &input, true))).unwrap_or_else(|_| "panic".to_string());
    if expected == "panic" {
      assert!(
        actual.starts_with("error <ParserError> syntax error"),
        "case {case}: parse of {input:?} in {frames:?}: {actual}"
      );
    } else {
      assert_eq!(expected, actual, "case {case}: parse of {input:?} in {frames:?}");
    }
    if !actual.starts_with("error") {
      parsed += 1;
    }
  }
  // enough of the random expressions are valid FEEL for the AST comparison to matter
  assert!(parsed > 1000, "only {parsed} expressions parsed");
  assert!(composite > 20, "only {composite} composite names");
}

/// The parsing scope before [ParsingScope::contains]: a stack of parsing contexts (input
/// contexts converted on creation) and the parsed names.
struct PreviousScope {
  stack: Vec<ParsingContext>,
  names: HashSet<Name>,
}

impl PreviousScope {
  fn new(frames: &[FeelContext]) -> Self {
    Self {
      stack: frames.iter().map(ParsingContext::from).collect(),
      names: HashSet::new(),
    }
  }

  fn flattened(&self) -> HashSet<String> {
    let keys = self.stack.iter().flat_map(|ctx| ctx.flattened_keys()).collect::<HashSet<String>>();
    let names = self.names.iter().map(|name| name.to_string()).collect::<HashSet<String>>();
    keys.union(&names).cloned().collect()
  }
}

#[test]
fn scope_operations_equal_previous_scope() {
  let mut rng = Rng(0xD1B5_4A32_D192_ED03);
  for case in 0..1500 {
    let frames: Vec<FeelContext> = (0..rng.below(3)).map(|_| context(&mut rng, 0)).collect();
    let scope = ParsingScope::new(&frames);
    let mut previous = PreviousScope::new(&frames);
    for step in 0..12 {
      // unbalanced pops and names set without a pushed context included
      match rng.below(6) {
        0 => {
          scope.pop();
          previous.stack.pop();
        }
        1 => {
          scope.push_default();
          previous.stack.push(ParsingContext::default());
        }
        2 | 3 => {
          let name = Name::from(*rng.pick(KEYS));
          scope.set_entry_name(name.clone());
          if let Some(top) = previous.stack.last_mut() {
            top.set_name(name);
          }
        }
        4 => {
          let name = Name::from(*rng.pick(KEYS));
          scope.add_parsed_name(name.clone());
          previous.names.insert(name);
        }
        _ => {
          // the same nested context on both sides (the conversion is deterministic)
          let name = Name::from(*rng.pick(KEYS));
          let nested = context(&mut rng, 1);
          scope.set_context(name.clone(), ParsingContext::from(&nested));
          if let Some(top) = previous.stack.last_mut() {
            top.set_context(name, ParsingContext::from(&nested));
          }
        }
      }
      let flattened = previous.flattened();
      assert_eq!(flattened, scope.flattened(), "case {case} step {step}: flattened keys");
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
      let plain = previous.stack.iter().flat_map(ParsingContext::keys_deep).chain(previous.names.iter().map(Name::to_string));
      for member in plain {
        let word = first_word(&member);
        assert!(
          scope.group_bound(word, false, usize::MAX) >= member.len(),
          "case {case} step {step}: group_bound({word:?}) for {member:?}"
        );
      }
      for member in BUILT_IN_PROPERTY_NAMES {
        assert!(
          scope.group_bound(first_word(member), true, usize::MAX) >= member.len(),
          "case {case} step {step}: group_bound after dot for {member:?}"
        );
      }
      let plain_max = flattened.iter().filter(|k| !k.contains(" . ")).map(String::len).max().unwrap_or(0);
      let all_max = flattened.iter().map(String::len).max().unwrap_or(0);
      assert!(scope.bound(false, false) >= plain_max, "case {case} step {step}: bound without composite keys");
      assert!(scope.bound(false, true) >= all_max, "case {case} step {step}: bound with composite keys");
      assert!(scope.bound(true, false) >= plain_max.max(16), "case {case} step {step}: bound after dot");
    }
  }
}

/// A chain of nested contexts `k0: {k1: {… {leaf: 1}}}` of the given depth, at the top level and
/// under the key `a./` (so that the lexer can produce composite names into it), next to `x: 1`.
fn chain(depth: usize) -> FeelContext {
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

#[test]
fn deep_chain_of_distinct_keys_is_linear() {
  // a 30-level chain k0 . k1 . … . k29 has more than 2^30 flattened keys; the lookup must not build them
  let scope = FeelScope::from(chain(30));
  let started = Instant::now();
  let node = crate::parse_expression(&scope, "x + 1", false).unwrap();
  assert_eq!(
    format!("{:?}", crate::parse_expression(&FeelScope::from(chain(0)), "x + 1", false).unwrap()),
    format!("{node:?}")
  );
  // a composite name into the chain, 30 levels down
  let input = format!("a./{OGHAM}.{OGHAM} leaf + 1");
  let node = crate::parse_expression(&scope, &input, false).unwrap();
  let trace = node.trace();
  assert!(trace.contains("`a./. leaf`") && !trace.contains("Path"), "{trace}");
  assert!(started.elapsed().as_secs() < 5, "took {:?}", started.elapsed());
}
