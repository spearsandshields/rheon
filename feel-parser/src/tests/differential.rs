//! Differential tests of the lexer and parser against the frozen prototype ([super::oracle]).
//!
//! For every input, start rule and scope, both front ends must read the same tokens (token type,
//! Debug text of the value, start offset of the token in the input), and give the same AST (tree
//! and Debug text) or the same error. The only allowed differences are the registered bug fixes
//! ([registered_difference]).
//!
//! Generators:
//! - G1: every string of up to 3 characters over [ALPHABET], in [scopes], with every start rule;
//!   also prefixed with the heads of iteration contexts. Longer strings: `LEXPARSE_G1_LEN`.
//! - G2: random FEEL texts of every construct ([feel]), with random input contexts (G3, from
//!   [super::names_index]).
//! - G4: the expressions of the evaluator corpus (`LEXPARSE_CORPUS`, a directory of `*.jsonl`).
//! - G5: random edits and every prefix of G2 and G4 texts (cut-off inputs, syntax errors).

use super::names_index::{KEYS, OGHAM, Rng, context, key_text, number};
use super::oracle;
use crate::lalr::TokenType;
use crate::parser::Parser;
use crate::scope::ParsingScope;
use rheon_feel::context::FeelContext;
use rheon_feel::values::Value;
use rheon_feel::{FeelType, Name};
use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// The start rules of the grammar.
const START_RULES: [(&str, TokenType, oracle::lalr::TokenType); 9] = [
  ("expression", TokenType::StartExpression, oracle::lalr::TokenType::StartExpression),
  ("boxed expression", TokenType::StartBoxedExpression, oracle::lalr::TokenType::StartBoxedExpression),
  ("textual expression", TokenType::StartTextualExpression, oracle::lalr::TokenType::StartTextualExpression),
  ("textual expressions", TokenType::StartTextualExpressions, oracle::lalr::TokenType::StartTextualExpressions),
  ("unary tests", TokenType::StartUnaryTests, oracle::lalr::TokenType::StartUnaryTests),
  ("simple expression", TokenType::StartSimpleExpression, oracle::lalr::TokenType::StartSimpleExpression),
  ("simple expressions", TokenType::StartSimpleExpressions, oracle::lalr::TokenType::StartSimpleExpressions),
  ("simple value", TokenType::StartSimpleValue, oracle::lalr::TokenType::StartSimpleValue),
  ("range literal", TokenType::StartRangeLiteral, oracle::lalr::TokenType::StartRangeLiteral),
];

/// The characters of the G1 strings: name characters, keyword letters, white space (U+1680 is a
/// name character too), the additional name symbols, digits and the punctuation of the grammar.
const ALPHABET: [char; 24] = [
  'a', 'b', 'i', 'n', 'o', 'r', 'f', ' ', '.', '+', '-', '*', '/', ':', ',', '(', ')', '[', ']', '"', '1', OGHAM, '<', '?',
];

/// The second alphabet (`LEXPARSE_G1_ALPHABET=2`): the remaining punctuation, escapes and white space.
const ALPHABET_2: [char; 24] = [
  'a', 'e', 'i', 'n', 't', ' ', '{', '}', '=', '@', '\'', '\\', '\t', '\n', '\u{FEFF}', '\u{180E}', '.', ':', ',', '"', '5', 'E', '>', '!',
];

/// What a front end did with an input.
#[derive(Debug, PartialEq)]
enum Outcome {
  /// The AST: tree text and Debug text.
  Ast(String),
  /// The error message.
  Error(String),
  /// The panic message.
  Panic(String),
}

/// The tokens read by the parser and the outcome.
#[derive(Debug, PartialEq)]
struct Run {
  tokens: Vec<(i16, String, usize)>,
  outcome: Outcome,
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
  payload
    .downcast_ref::<String>()
    .cloned()
    .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
    .unwrap_or_default()
}

/// Runs the current front end; a panic is an outcome.
fn run_new(frames: &[FeelContext], start: &TokenType, input: &str) -> Run {
  let mut tokens = vec![];
  let outcome = catch_unwind(AssertUnwindSafe(|| {
    let scope = ParsingScope::new(frames);
    let mut parser = Parser::new(&scope, start.clone(), input, false).with_token_log();
    let result = parser.parse();
    tokens = parser.token_log().into_iter().map(|(tt, tv, start)| (tt as i16, format!("{tv:?}"), start)).collect();
    result
  }));
  Run {
    tokens,
    outcome: match outcome {
      Ok(Ok(node)) => Outcome::Ast(format!("{}\n{node:?}", node.trace())),
      Ok(Err(e)) => Outcome::Error(e.to_string()),
      Err(payload) => Outcome::Panic(panic_message(payload)),
    },
  }
}

/// Runs the frozen prototype; a panic is an outcome.
fn run_oracle(frames: &[FeelContext], start: &oracle::lalr::TokenType, input: &str) -> Run {
  let mut tokens = vec![];
  let outcome = catch_unwind(AssertUnwindSafe(|| {
    let scope = oracle::scope::ParsingScope::new(frames);
    let mut parser = oracle::parser::Parser::new(&scope, start.clone(), input, false).with_token_log();
    let result = parser.parse();
    tokens = parser.token_log();
    result
  }));
  Run {
    tokens,
    outcome: match outcome {
      Ok(Ok(node)) => Outcome::Ast(format!("{}\n{node:?}", node.trace())),
      Ok(Err(e)) => Outcome::Error(e.to_string()),
      Err(payload) => Outcome::Panic(panic_message(payload)),
    },
  }
}

/// The registered differences: bug fixes, each with the old and the new behaviour. Returns the
/// name of the fix when the difference between `old` (the prototype) and `new` is one of them.
fn registered_difference(_start: &str, _input: &str, _old: &Run, _new: &Run) -> Option<&'static str> {
  None
}

/// Statistics of a differential run.
#[derive(Default, Debug)]
struct Stats {
  cases: usize,
  asts: usize,
  errors: usize,
  /// Panics of the current front end (each one also a panic of the prototype, else a difference).
  panics: usize,
  registered: BTreeMap<&'static str, usize>,
}

impl Stats {
  /// Compares both front ends on one input; panics on an unregistered difference.
  fn check(&mut self, frames: &[FeelContext], input: &str) {
    for (name, start, oracle_start) in &START_RULES {
      self.check_rule(frames, name, start, oracle_start, input);
    }
  }

  fn check_rule(&mut self, frames: &[FeelContext], name: &str, start: &TokenType, oracle_start: &oracle::lalr::TokenType, input: &str) {
    let new = run_new(frames, start, input);
    let old = run_oracle(frames, oracle_start, input);
    self.cases += 1;
    match &new.outcome {
      Outcome::Ast(_) => self.asts += 1,
      Outcome::Error(_) => self.errors += 1,
      Outcome::Panic(_) => self.panics += 1,
    }
    if new != old {
      match registered_difference(name, input, &old, &new) {
        Some(fix) => *self.registered.entry(fix).or_default() += 1,
        None => panic!("{name} {input:?} in {frames:?}:\nprototype {old:#?}\ncurrent {new:#?}"),
      }
    }
  }
}

fn ctx(entries: &[(&str, Value)]) -> FeelContext {
  let mut ctx = FeelContext::default();
  for (key, value) in entries {
    ctx.set_entry(&Name::from(*key), value.clone());
  }
  ctx
}

/// The scopes of the G1 strings (each a list of input contexts, bottom first).
fn scopes() -> Vec<Vec<FeelContext>> {
  let nested = |key: &str, inner: FeelContext| ctx(&[(key, Value::Context(inner))]);
  let mut rng = Rng(0x2545_F491_4F6C_DD1D);
  vec![
    vec![],
    vec![ctx(&[("a", number(1))])],
    vec![ctx(&[
      ("in", number(1)),
      ("in+1", number(2)),
      ("a b", number(3)),
      ("x", Value::Context(ctx(&[("y", number(1))]))),
      ("i", number(4)),
      ("n", number(5)),
    ])],
    vec![context(&mut rng, 0), context(&mut rng, 0)],
    vec![ctx(&[
      ("a+b", number(1)),
      ("a.b", number(2)),
      ("a/b", number(3)),
      ("a-b", number(4)),
      ("a*b", number(5)),
      ("a'b", number(6)),
      ("a b", number(7)),
      ("b", number(8)),
    ])],
    vec![nested("a", nested("b", nested("a", ctx(&[("b", number(1))]))))],
    vec![ctx(&[
      ("for", number(1)),
      ("if", number(2)),
      ("not", number(3)),
      ("item", number(4)),
      ("partial", number(5)),
      ("date", number(6)),
      ("number", number(7)),
      ("or", number(8)),
      ("?", number(9)),
      ("list", Value::List(vec![Value::Context(ctx(&[("a", number(1))]))])),
      ("t", Value::FeelType(FeelType::Context(BTreeMap::from([(Name::from("b"), FeelType::Number)])))),
    ])],
    vec![
      ctx(&[("a", number(1)), (&format!("a{OGHAM}b"), number(2))]),
      ctx(&[("b", Value::Context(ctx(&[("a", number(2)), ("i n", number(3))])))]),
    ],
  ]
}

/// Calls `f` with every string of exactly `len` characters over `alphabet`.
fn for_each_string(alphabet: &[char], len: usize, mut f: impl FnMut(&str)) {
  let mut counters = vec![0_usize; len];
  let mut text = String::new();
  loop {
    text.clear();
    text.extend(counters.iter().map(|&i| alphabet[i]));
    f(&text);
    let mut i = 0;
    while i < len && counters[i] == alphabet.len() - 1 {
      counters[i] = 0;
      i += 1;
    }
    if i == len {
      return;
    }
    counters[i] += 1;
  }
}

fn env_usize(name: &str, default: usize) -> usize {
  std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

#[test]
fn g1_short_strings_equal_prototype() {
  let max_len = env_usize("LEXPARSE_G1_LEN", 3);
  let alphabet: &[char] = if env_usize("LEXPARSE_G1_ALPHABET", 1) == 2 { &ALPHABET_2 } else { &ALPHABET };
  let mut stats = Stats::default();
  for frames in scopes() {
    for len in 0..=max_len {
      for_each_string(alphabet, len, |text| stats.check(&frames, text));
    }
  }
  println!("G1: {stats:?}");
  assert!(stats.asts > 1000, "{stats:?}");
}

#[test]
fn g1_iteration_contexts_equal_prototype() {
  // the heads that set the lexer's `till_in` flag, followed by every short string
  let max_len = env_usize("LEXPARSE_G1_LEN", 3);
  let mut stats = Stats::default();
  for frames in &scopes()[..3] {
    for head in ["for ", "some ", "every ", "for x in [1], ", "for item in [1] return "] {
      for len in 0..=max_len {
        for_each_string(&ALPHABET, len, |text| {
          let input = format!("{head}{text}");
          for (name, start, oracle_start) in &START_RULES[..1] {
            stats.check_rule(frames, name, start, oracle_start, &input);
          }
          let input = format!("{head}{text} in [1] return 1");
          for (name, start, oracle_start) in &START_RULES[..1] {
            stats.check_rule(frames, name, start, oracle_start, &input);
          }
        });
      }
    }
  }
  println!("G1 (iteration contexts): {stats:?}");
}

/// Random FEEL texts (G2): every construct of the grammar, names of the input contexts written in
/// several ways ([key_text]), comments and every kind of white space.
struct Feel<'a> {
  rng: &'a mut Rng,
  keys: &'a [String],
  /// Operands left; then only leaves (keeps the texts short).
  budget: usize,
}

impl Feel<'_> {
  fn ws(&mut self) -> &'static str {
    self
      .rng
      .pick_str(&["", " ", " ", " ", "  ", "\t", "\n", " /* c */ ", " // c\n", "\u{1680}", "\u{180E}", "\u{FEFF}", "\u{2003}"])
  }

  fn name(&mut self) -> String {
    let key = self.rng.pick(self.keys).clone();
    key_text(self.rng, &key)
  }

  fn number(&mut self) -> String {
    self
      .rng
      .pick_str(&[
        "1",
        "0",
        "42",
        "1.5",
        ".5",
        "5.",
        "007",
        "1e3",
        "1E-3",
        "2e+2",
        "1e",
        "12345678901234567890123456789012345",
        "0.000001",
      ])
      .to_string()
  }

  fn string(&mut self) -> String {
    self
      .rng
      .pick_str(&[
        r#""s""#,
        r#""a b""#,
        r#""\"q\"""#,
        r#""\\\n\t\r\'""#,
        r#""A""#,
        r#""🙀""#,
        r#""\U01F600""#,
        r#""""#,
        r#""unterminated"#,
        r#""\uD800""#,
        r#""\u12G4""#,
      ])
      .to_string()
  }

  fn temporal(&mut self) -> String {
    match self.rng.below(8) {
      0 => r#"date("2012-01-01")"#.to_string(),
      1 => r#"time("10:00:00")"#.to_string(),
      2 => r#"date and time("2012-01-01T10:00:00")"#.to_string(),
      3 => r#"duration("P1D")"#.to_string(),
      4 => r#"date(from: "2012-01-01")"#.to_string(),
      5 => r#"@"2012-01-01""#.to_string(),
      6 => "date(2012, 1, 1)".to_string(),
      _ => r#"@"P1Y""#.to_string(),
    }
  }

  fn type_text(&mut self, depth: usize) -> String {
    match if depth > 2 { self.rng.below(2) } else { self.rng.below(9) } {
      0 => self
        .rng
        .pick_str(&[
          "number",
          "string",
          "boolean",
          "date",
          "time",
          "date and time",
          "years and months duration",
          "days and time duration",
          "Any",
          "Null",
        ])
        .to_string(),
      1 => self.name(),
      2 => format!("list<{}>", self.type_text(depth + 1)),
      3 => format!("range<{}>", self.type_text(depth + 1)),
      4 => format!("context<{}: {}, {}: {}>", self.name(), self.type_text(depth + 1), self.name(), self.type_text(depth + 1)),
      5 => format!("function<{}, {}> -> {}", self.type_text(depth + 1), self.type_text(depth + 1), self.type_text(depth + 1)),
      6 => format!("function<> -> {}", self.type_text(depth + 1)),
      7 => format!("{}.{}", self.name(), self.name()),
      _ => format!("context<{}: {}>", self.name(), self.type_text(depth + 1)),
    }
  }

  fn operand(&mut self, depth: usize) -> String {
    self.budget = self.budget.saturating_sub(1);
    let leaf = depth > 3 || self.budget == 0;
    match if leaf { self.rng.below(6) } else { self.rng.below(30) } {
      0 | 1 => self.number(),
      2 => self.string(),
      3 | 4 => self.name(),
      5 => self.rng.pick_str(&["true", "false", "null", "item", "partial", "?"]).to_string(),
      6 => self.temporal(),
      7 => format!("[{}]", self.list(depth)),
      8 => format!("{{{}}}", self.context_entries(depth)),
      9 => format!("({})", self.expression(depth + 1)),
      10 => format!(
        "{}.{}",
        self.operand(depth + 1),
        self.rng.pick_str(&["year", "start", "start included", "end", "time offset", "a", "x"])
      ),
      11 => format!("{}...{}", self.operand(depth + 1), self.name()),
      12 => format!("{}[{}]", self.operand(depth + 1), self.expression(depth + 1)),
      13 => format!("{}({})", self.name(), self.list(depth)),
      14 => format!(
        "{}({}: {}, {}: {})",
        self.name(),
        self.name(),
        self.expression(depth + 1),
        self.name(),
        self.expression(depth + 1)
      ),
      15 => format!("if {} then {} else {}", self.expression(depth + 1), self.expression(depth + 1), self.expression(depth + 1)),
      16 => format!("for {} in {} return {}", self.name(), self.iteration_value(depth), self.expression(depth + 1)),
      17 => format!(
        "for {} in {}, {} in {} return {}",
        self.name(),
        self.iteration_value(depth),
        self.name(),
        self.iteration_value(depth),
        self.expression(depth + 1)
      ),
      18 => format!(
        "{} {} in {} satisfies {}",
        self.rng.pick_str(&["some", "every"]),
        self.name(),
        self.expression(depth + 1),
        self.expression(depth + 1)
      ),
      19 => format!("{} between {} and {}", self.operand(depth + 1), self.operand(depth + 1), self.operand(depth + 1)),
      20 => format!("{} instance of {}", self.operand(depth + 1), self.type_text(0)),
      21 => format!("function({}, {}) {}", self.name(), self.name(), self.expression(depth + 1)),
      22 => format!("function({}: {}) {}", self.name(), self.type_text(0), self.expression(depth + 1)),
      23 => r#"function() external {java: {class: "c", method signature: "m"}}"#.to_string(),
      24 => format!("{} in ({})", self.operand(depth + 1), self.list(depth)),
      25 => format!("{} in {}", self.operand(depth + 1), self.interval(depth)),
      26 => self.interval(depth),
      27 => format!("-{}", self.operand(depth + 1)),
      28 => format!("{} {}", self.rng.pick_str(&["<", "<=", ">", ">=", "=", "!="]), self.operand(depth + 1)),
      _ => format!(
        "{}{}{}",
        self.name(),
        self.rng.pick_str(&[" ", ".", "+", "-", "/", "*", "'", " . ", "\u{1680}.\u{1680}", " in "]),
        self.name()
      ),
    }
  }

  fn iteration_value(&mut self, depth: usize) -> String {
    if self.rng.chance(30) {
      format!("{}..{}", self.operand(depth + 1), self.operand(depth + 1))
    } else {
      self.operand(depth + 1)
    }
  }

  fn interval(&mut self, depth: usize) -> String {
    let open = self.rng.pick_str(&["[", "(", "]"]);
    let close = self.rng.pick_str(&["]", ")", "["]);
    format!("{open}{}..{}{close}", self.operand(depth + 1), self.operand(depth + 1))
  }

  fn list(&mut self, depth: usize) -> String {
    let n = self.rng.below(4);
    (0..n).map(|_| self.expression(depth + 1)).collect::<Vec<_>>().join(", ")
  }

  fn context_entries(&mut self, depth: usize) -> String {
    let n = self.rng.below(4);
    (0..n)
      .map(|_| {
        let key = if self.rng.chance(20) { self.string() } else { self.name() };
        format!("{key}: {}", self.expression(depth + 1))
      })
      .collect::<Vec<_>>()
      .join(", ")
  }

  fn expression(&mut self, depth: usize) -> String {
    let mut text = self.operand(depth);
    for _ in 0..self.rng.below(if depth > 1 || self.budget == 0 { 2 } else { 4 }) {
      let op = self.rng.pick_str(&["+", "-", "*", "/", "**", "and", "or", "=", "!=", "<", "<=", ">", ">="]);
      let (w1, w2) = (self.ws(), self.ws());
      let w1 = if w1.is_empty() && op.chars().all(char::is_alphabetic) { " " } else { w1 };
      let w2 = if w2.is_empty() && op.chars().all(char::is_alphabetic) { " " } else { w2 };
      text = format!("{text}{w1}{op}{w2}{}", self.operand(depth));
    }
    text
  }

  /// A text for any start rule: an expression, unary tests or a range literal.
  fn text(&mut self) -> String {
    match self.rng.below(10) {
      0 => format!("not({})", self.list(1)),
      1 => "-".to_string(),
      2 => self.list(1),
      3 => {
        let open = self.rng.pick_str(&["[", "(", "]"]);
        let close = self.rng.pick_str(&["]", ")", "["]);
        let endpoint = |feel: &mut Self| match feel.rng.below(6) {
          0 => String::new(),
          1 => feel.number(),
          2 => feel.string(),
          3 => r#"@"2012-01-01""#.to_string(),
          4 => format!(r#"{}(from: "2012-01-01")"#, feel.rng.pick_str(&["date", "time", "date and time", "duration"])),
          _ => format!(r#"{}("2012-01-01")"#, feel.rng.pick_str(&["date", "time", "date and time", "duration"])),
        };
        format!("{open}{}..{}{close}", endpoint(self), endpoint(self))
      }
      _ => self.expression(0),
    }
  }
}

/// The keys of the G2 scopes, with names that the generated texts use too.
fn g2_keys() -> Vec<String> {
  let mut keys: Vec<String> = KEYS.iter().map(|k| k.to_string()).collect();
  keys.extend(["zz", "in x", "a in b", "date", "number", "string length", "Full Name"].map(String::from));
  keys
}

#[test]
fn g2_random_texts_equal_prototype() {
  let count = env_usize("LEXPARSE_G2_CASES", 20_000);
  let mut rng = Rng(0x853C_49E6_748F_EA9B);
  let keys = g2_keys();
  let mut stats = Stats::default();
  for _ in 0..count {
    let frames: Vec<FeelContext> = (0..rng.below(3)).map(|_| context(&mut rng, 0)).collect();
    let input = Feel {
      rng: &mut rng,
      keys: &keys,
      budget: 24,
    }
    .text();
    stats.check(&frames, &input);
  }
  println!("G2: {stats:?}");
  assert!(stats.asts > count, "{stats:?}");
}

/// Random edits of a text (G5): deletions, insertions of characters and keywords, duplications.
fn mutate(rng: &mut Rng, text: &str) -> String {
  let mut chars: Vec<char> = text.chars().collect();
  for _ in 0..1 + rng.below(3) {
    let at = rng.below(chars.len() + 1);
    match rng.below(4) {
      0 if at < chars.len() => {
        let end = (at + 1 + rng.below(4)).min(chars.len());
        chars.drain(at..end);
      }
      1 => {
        let snippet = rng.pick_str(&[
          "in", " in ", "for ", "(", ")", "[", "]", "{", "}", ":", ",", ".", "..", "\"", " ", "\u{1680}", "item", "?", "not(", "@", "//", "/*", "*/", "\\u",
        ]);
        chars.splice(at..at, snippet.chars());
      }
      2 if at < chars.len() => {
        let end = (at + 1 + rng.below(6)).min(chars.len());
        let copy: Vec<char> = chars[at..end].to_vec();
        chars.splice(at..at, copy);
      }
      _ => chars.insert(at, *rng.pick(&ALPHABET)),
    }
  }
  chars.into_iter().collect()
}

#[test]
fn g5_edited_and_cut_texts_equal_prototype() {
  let count = env_usize("LEXPARSE_G5_CASES", 3_000);
  let mut rng = Rng(0xDA94_2042_E4DD_58B5);
  let keys = g2_keys();
  let mut stats = Stats::default();
  for _ in 0..count {
    let frames: Vec<FeelContext> = (0..rng.below(3)).map(|_| context(&mut rng, 0)).collect();
    let text = Feel {
      rng: &mut rng,
      keys: &keys,
      budget: 24,
    }
    .text();
    for _ in 0..3 {
      let edited = mutate(&mut rng, &text);
      stats.check(&frames, &edited);
    }
    // every prefix: a syntax error at every point of the text
    if rng.chance(10) {
      for (end, _) in text.char_indices() {
        stats.check(&frames, &text[..end]);
      }
    }
  }
  println!("G5: {stats:?}");
}

/// An input context with the keys of a FEEL context literal (the values are numbers, nested
/// literals become nested contexts), parsed by the current parser.
fn keys_of_context_literal(text: &str) -> FeelContext {
  use crate::AstNode;
  fn convert(node: &AstNode) -> FeelContext {
    let mut ctx = FeelContext::default();
    if let AstNode::Context(entries) = node {
      for entry in entries {
        if let AstNode::ContextEntry(key, value) = entry
          && let AstNode::ContextEntryKey(name) = key.as_ref()
        {
          let value = match value.as_ref() {
            inner @ AstNode::Context(_) => Value::Context(convert(inner)),
            _ => number(1),
          };
          ctx.set_entry(name, value);
        }
      }
    }
    ctx
  }
  match crate::parse_context(&Default::default(), text, false) {
    Ok(node) => convert(&node),
    Err(_) => FeelContext::default(),
  }
}

/// The expressions and context literals of the evaluator corpus (`tools/evaluator-corpus/corpora`).
fn corpus() -> Vec<(String, String)> {
  let Ok(dir) = std::env::var("LEXPARSE_CORPUS") else {
    return vec![];
  };
  let mut lines = vec![];
  let mut files: Vec<_> = std::fs::read_dir(&dir).expect("LEXPARSE_CORPUS").map(|e| e.unwrap().path()).collect();
  files.sort();
  for file in files.into_iter().filter(|f| f.extension().is_some_and(|e| e == "jsonl")) {
    for line in std::fs::read_to_string(file).unwrap().lines() {
      // the lines are {"src": "...", "ctx": "...", "expr": "..."}: read the two strings without a JSON dependency
      let field = |name: &str| -> String {
        let key = format!("\"{name}\": \"");
        let Some(start) = line.find(&key) else { return String::new() };
        let mut out = String::new();
        let mut chars = line[start + key.len()..].chars();
        while let Some(ch) = chars.next() {
          match ch {
            '"' => break,
            '\\' => match chars.next() {
              Some('n') => out.push('\n'),
              Some('t') => out.push('\t'),
              Some('r') => out.push('\r'),
              Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                out.push(char::from_u32(u32::from_str_radix(&hex, 16).unwrap_or(0xFFFD)).unwrap_or('\u{FFFD}'));
              }
              Some(other) => out.push(other),
              None => {}
            },
            other => out.push(other),
          }
        }
        out
      };
      lines.push((field("ctx"), field("expr")));
    }
  }
  lines
}

#[test]
#[ignore]
fn g4_corpus_equals_prototype() {
  let corpus = corpus();
  assert!(!corpus.is_empty(), "set LEXPARSE_CORPUS to tools/evaluator-corpus/corpora");
  let mut rng = Rng(0x1234_5678_9ABC_DEF1);
  let rich = context(&mut rng, 0);
  let mut stats = Stats::default();
  for (ctx_text, expr) in &corpus {
    for frames in [vec![], vec![keys_of_context_literal(ctx_text)], vec![rich.clone()]] {
      stats.check(&frames, expr);
    }
    // cut-off and edited corpus texts (G5)
    let frames = vec![keys_of_context_literal(ctx_text)];
    stats.check(&frames, &mutate(&mut rng, expr));
    if rng.chance(5) {
      for (end, _) in expr.char_indices() {
        stats.check(&frames, &expr[..end]);
      }
    }
  }
  println!("G4: {} lines, {stats:?}", corpus.len());
}

/// Every reduction of a rule without symbols whose action changes the state of the lexer or of the
/// scope happens before the lexer reads the next token: in a state with a default reduction that
/// needs no lookahead. Rule 137 (`{/* type_name */}` at the start of `function_type_parameters`) is
/// the known exception: it is reduced after the lookahead was read.
#[test]
fn feedback_actions_are_reduced_before_the_next_token() {
  use crate::lalr::*;
  const FEEDBACK: [&str; 14] = [
    "after_dot",
    "between_begin",
    "type_name",
    "for_begin",
    "some_begin",
    "every_begin",
    "context_begin",
    "context_end",
    "iteration_context_variable_name_begin",
    "iteration_context_variable_name",
    "quantified_expression_variable_name_begin",
    "quantified_expression_variable_name",
    "formal_parameters_begin",
    "unary_tests_begin",
  ];
  // rule number -> action name, from the dispatch in lalr.rs
  let source = include_str!("../lalr.rs");
  let mut actions = BTreeMap::new();
  for line in source.lines() {
    if let Some((rule, action)) = line.trim().split_once(" => reduce_actions.action_")
      && let Ok(rule) = rule.parse::<usize>()
    {
      actions.insert(rule, action.trim_end_matches("(),").to_string());
    }
  }
  let feedback = |rule: usize| YY_R2[rule] == 0 && actions.get(&rule).is_some_and(|a| FEEDBACK.contains(&a.as_str()));
  let mut late = vec![];
  for state in 0..YY_PACT.len() {
    let mut reduced = vec![YY_DEF_ACT[state] as usize];
    let base = YY_PACT[state];
    if base != YY_PACT_N_INF {
      for token in 0..YY_N_TOKENS as i16 {
        let i = base + token;
        if (0..=YY_LAST).contains(&i) && YY_CHECK[i as usize] == token && YY_TABLE[i as usize] < 0 && YY_TABLE[i as usize] != YY_TABLE_N_INF {
          reduced.push((-YY_TABLE[i as usize]) as usize);
        }
      }
    }
    for rule in reduced.into_iter().filter(|&rule| rule != 0 && feedback(rule)) {
      if base != YY_PACT_N_INF {
        late.push((rule, actions[&rule].clone()));
      }
    }
  }
  late.sort();
  late.dedup();
  assert_eq!(vec![(137, "type_name".to_string())], late);
}

/// The parser's use of the native stack does not depend on the input: deep and long inputs parse,
/// and their ASTs are dropped, in a thread with a small stack.
#[test]
fn deep_inputs_parse_in_a_small_stack() {
  let n = 5_000;
  let inputs = vec![
    format!("{}1{}", "(1+".repeat(n), ")".repeat(n)),
    format!("{}1{}", "[".repeat(n), "]".repeat(n)),
    format!("{}1{}", "{a:".repeat(n), "}".repeat(n)),
    format!("{}1", "-".repeat(n)),
    format!("{}1", "if true then 1 else ".repeat(n)),
    format!("{}1{}", "f(".repeat(n), ")".repeat(n)),
    format!("1{}", "+1".repeat(4 * n)),
    format!("a{}", ".a".repeat(4 * n)),
    format!("a{}", " + a".repeat(4 * n)),
    format!("{{{}}}", (0..n).map(|i| format!("k{i}: {i}")).collect::<Vec<_>>().join(", ")),
    format!("[{}]", (0..n).map(|i| i.to_string()).collect::<Vec<_>>().join(", ")),
    format!("{}x", "for x in [1] return ".repeat(n / 10)),
  ];
  // the stack of a debug build: about 10x the frames of an optimized one
  let handle = std::thread::Builder::new()
    .stack_size(256 * 1024)
    .spawn(move || {
      for input in inputs {
        let node = crate::parse_expression(&Default::default(), &input, false).unwrap_or_else(|e| panic!("{e}: {}", &input[..80]));
        drop(node);
      }
    })
    .unwrap();
  handle.join().unwrap();
}

/// Inputs of the measurements below: (name, input contexts, expression) of suite workloads
/// (perf/requests) and of parse workloads (perf/lexparse).
fn measured_inputs() -> Vec<(&'static str, Vec<FeelContext>, String)> {
  let numbers = |keys: &[&str]| ctx(&keys.iter().map(|k| (*k, number(1))).collect::<Vec<_>>());
  let nested = |key: &str, inner: FeelContext| (key.to_string(), Value::Context(inner));
  let mut names_10f = numbers(&["price", "quantity", "discount", "fee", "bonus", "adjustment", "f0"]);
  for (key, value) in [
    nested("tax", numbers(&["rate", "region"])),
    nested("shipping", numbers(&["cost", "carrier"])),
    nested("customer", {
      let mut c = numbers(&["age", "name"]);
      c.set_entry(&Name::from("loyalty"), Value::Context(numbers(&["points", "tier"])));
      c
    }),
  ] {
    names_10f.set_entry(&Name::from(key.as_str()), value);
  }
  let deep = ctx(&[("a", Value::Context(ctx(&[("b", Value::Context(ctx(&[("c", Value::Context(numbers(&["d"])))])))])))]);
  let ctx200 = format!("{{a: 2, b: 3, {}}}.r7", (0..200).map(|i| format!("r{i}: a * {i} + b")).collect::<Vec<_>>().join(", "));
  let chain = (0..100).map(|i| format!("k{}", i % 10)).collect::<Vec<_>>().join(" + ");
  let mw = ctx(&[("monthly salary", number(1)), ("yearly bonus amount", number(2)), ("tax rate", number(3))]);
  vec![
    ("tiny", vec![], "1 + 2".to_string()),
    ("decimal-arith", vec![numbers(&["a", "b", "c", "d"])], "(a * b + c) / d".to_string()),
    (
      "names-10f",
      vec![names_10f],
      "price * quantity - discount + tax.rate * price + shipping.cost + (if customer.age >= 18 then fee else 0) + bonus - adjustment".to_string(),
    ),
    (
      "str-concat",
      vec![numbers(&["name", "code"])],
      r#"upper case(name) + " " + substring(code, 1, 3)"#.to_string(),
    ),
    ("date-add", vec![], r#"date("2026-01-01") + duration("P1M")"#.to_string()),
    (
      "datetime-compare",
      vec![],
      r#"date and time("2026-03-01T10:00:00+01:00") > date and time("2026-03-01T08:30:00Z")"#.to_string(),
    ),
    ("deep-path", vec![deep], "a.b.c.d".to_string()),
    ("ctx200-0f", vec![], ctx200),
    ("chain-names-100", vec![numbers(&["k0", "k1", "k2", "k3", "k4", "k5", "k6", "k7", "k8", "k9"])], chain),
    ("mw-names", vec![mw], "monthly salary * 12 + yearly bonus amount - tax rate * monthly salary".to_string()),
  ]
}

/// The work counters of a parse (`work!`).
fn work_of(frames: &[FeelContext], input: &str) -> BTreeMap<&'static str, usize> {
  super::work::take();
  let scope = ParsingScope::new(frames);
  let _ = Parser::new(&scope, TokenType::StartExpression, input, false).parse();
  super::work::take()
}

/// Prints the work counters of the measured inputs (run with `--ignored --nocapture`).
#[test]
#[ignore]
fn work_counters() {
  for (name, frames, input) in measured_inputs() {
    println!("{name}: {:?}", work_of(&frames, &input));
  }
}

/// Splits the parse time into the lexer and the rest (LALR driver, actions, AST): the parse of the
/// recorded tokens (no lexer) against the full parse. Run with `--release --ignored --nocapture`.
#[test]
#[ignore]
fn replay_split() {
  use std::time::Instant;
  for (name, frames, input) in measured_inputs() {
    let scope = ParsingScope::new(&frames);
    let mut parser = Parser::new(&scope, TokenType::StartExpression, &input, false).with_token_log();
    parser.parse().unwrap();
    let tokens: Vec<_> = parser.token_log().into_iter().map(|(tt, tv, _)| (tt, tv)).collect();
    let n = (200_000 / input.len()).clamp(20, 20_000);
    let time = |replay: bool| {
      let mut best = f64::MAX;
      for _ in 0..5 {
        let started = Instant::now();
        for _ in 0..n {
          let scope = ParsingScope::new(&frames);
          let mut parser = Parser::new(&scope, TokenType::StartExpression, &input, false);
          if replay {
            parser = parser.with_replay(tokens.clone());
          }
          std::hint::black_box(parser.parse().unwrap());
        }
        best = best.min(started.elapsed().as_secs_f64() * 1e6 / n as f64);
      }
      best
    };
    // the clone of the recorded tokens is part of the replay time
    let clone = {
      let started = Instant::now();
      for _ in 0..n {
        std::hint::black_box(tokens.clone());
      }
      started.elapsed().as_secs_f64() * 1e6 / n as f64
    };
    let (full, replay) = (time(false), time(true) - clone);
    println!(
      "{name}: {} tokens, full parse {full:.2} us, without the lexer {replay:.2} us, lexer {:.2} us ({:.0}%)",
      tokens.len(),
      full - replay,
      100.0 * (full - replay) / full
    );
  }
}
