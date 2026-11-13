//! Differential tests of the lexer and parser against the frozen reference front end
//! ([super::oracle]).
//!
//! For every input, start rule and scope, both front ends must read the same tokens (token type,
//! Debug text of the value, start offset of the token in the input), and give the same AST (tree
//! and Debug text) or the same error. The only allowed differences are the registered bug fixes
//! ([registered_difference]).
//!
//! Generators (default size, and full size with `RHEON_FULL_TESTS` set, see [super::full_size]):
//! - short strings: every string of up to 2 characters (4) over [ALPHABET] (and over
//!   [ALPHABET_2]), in [scopes], with every start rule; also after the heads of iteration contexts.
//! - random texts: random FEEL texts of every construct ([Feel]), with random input contexts (from
//!   [super::names_index]): 1,000 (20,000).
//! - corpus: the expressions of the evaluator corpus, run only when `RHEON_PARSER_CORPUS` is the
//!   absolute path of a directory of `*.jsonl` files (`tools/evaluator-corpus/corpora`; the test
//!   is ignored by default: `-- --include-ignored`).
//! - edited and cut texts: random edits and every prefix of random texts and corpus texts (cut-off
//!   inputs, syntax errors): 60 (3,000).
//! - unicode texts: random texts of multi-byte characters: 1,000 (20,000).
//!
//! The other tests cover chosen inputs; some of them use longer inputs in full size. The full
//! size is meant for a release build: `RHEON_FULL_TESTS=1 cargo test --release -p rheon-feel-parser`.

use super::full_size;
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

/// The characters of the short strings: name characters, keyword letters, white space (U+1680 is a
/// name character too), the additional name symbols, digits and the punctuation of the grammar.
const ALPHABET: [char; 24] = [
  'a', 'b', 'i', 'n', 'o', 'r', 'f', ' ', '.', '+', '-', '*', '/', ':', ',', '(', ')', '[', ']', '"', '1', OGHAM, '<', '?',
];

/// The second alphabet of the short strings (full size only): the remaining punctuation, escapes and white space.
const ALPHABET_2: [char; 24] = [
  'a', 'e', 'i', 'n', 't', ' ', '{', '}', '=', '@', '\'', '\\', '\t', '\n', '\u{FEFF}', '\u{180E}', '.', ':', ',', '"', '5', 'E', '>', '!',
];

/// What a front end did with an input.
#[derive(Debug, PartialEq)]
pub(super) enum Outcome {
  /// The AST: tree text and Debug text.
  Ast(String),
  /// The error message.
  Error(String),
  /// The panic message.
  Panic(String),
}

/// The tokens read by the parser (type, Debug text of the value, start offset) and the outcome.
#[derive(Debug, PartialEq)]
pub(super) struct Run {
  pub(super) tokens: Vec<(i16, String, usize)>,
  pub(super) outcome: Outcome,
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
  payload
    .downcast_ref::<String>()
    .cloned()
    .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
    .unwrap_or_default()
}

/// Runs the production front end; a panic is an outcome.
pub(super) fn run_production(frames: &[FeelContext], start: &TokenType, input: &str) -> Run {
  let scope = ParsingScope::new(frames);
  let mut parser = Parser::new(&scope, start.clone(), input, false).with_token_log();
  let outcome = catch_unwind(AssertUnwindSafe(|| parser.parse()));
  Run {
    tokens: parser.token_log().into_iter().map(|(tt, tv, start)| (tt as i16, format!("{tv:?}"), start)).collect(),
    outcome: match outcome {
      Ok(Ok(node)) => Outcome::Ast(format!("{}\n{node:?}", node.trace())),
      Ok(Err(e)) => Outcome::Error(e.to_string()),
      Err(payload) => Outcome::Panic(panic_message(payload)),
    },
  }
}

/// Runs the frozen reference front end; a panic is an outcome.
fn run_reference(frames: &[FeelContext], start: &oracle::lalr::TokenType, input: &str) -> Run {
  let scope = oracle::scope::ParsingScope::new(frames);
  reference_run(oracle::parser::Parser::new(&scope, start.clone(), input, false).with_token_log(), input)
}

/// Runs the frozen reference front end on an expression, with its lexer in the flattened-key mode
/// (it searches the set of all flattened keys); a panic is an outcome.
pub(super) fn run_reference_flattened(frames: &[FeelContext], input: &str) -> Run {
  let scope = oracle::scope::ParsingScope::new(frames);
  let parser = oracle::parser::Parser::new(&scope, oracle::lalr::TokenType::StartExpression, input, false)
    .with_token_log()
    .with_oracle_lexer();
  reference_run(parser, input)
}

/// Parses with a reference parser that records its tokens.
fn reference_run(mut parser: oracle::parser::Parser<'_>, input: &str) -> Run {
  let outcome = catch_unwind(AssertUnwindSafe(|| parser.parse()));
  // the reference lexer counts positions in characters, the production lexer in bytes
  let bytes: Vec<usize> = input.char_indices().map(|(offset, _)| offset).chain([input.len()]).collect();
  Run {
    tokens: parser.token_log().into_iter().map(|(tt, tv, start)| (tt, tv, bytes[start.min(bytes.len() - 1)])).collect(),
    outcome: match outcome {
      Ok(Ok(node)) => Outcome::Ast(format!("{}\n{node:?}", node.trace())),
      Ok(Err(e)) => Outcome::Error(e.to_string()),
      Err(payload) => Outcome::Panic(panic_message(payload)),
    },
  }
}

/// The registered differences: bug fixes of the production front end, each with the reference's
/// and the production behaviour. Returns the name of the fix when the difference between
/// `reference` and `production` is one of them.
fn registered_difference(start: &str, reference: &Run, production: &Run) -> Option<&'static str> {
  // A name token in iteration-variable mode (the lexer's `till_in`) whose parts start with `in`:
  // an iteration variable (`for in+1 in [1] return 1`), or the `in` after an iteration variable
  // `item`, which leaves the mode set (`for item in[1] return item`):
  // - reference: `consumed_positions[index - 1]` with index 0 in the lexer, an overflow panic (a
  //   trap; an index out of bounds without overflow checks);
  // - production: the tokens read before, then the keyword `in`; a syntax error, unless an
  //   iteration variable `item` comes before (`for item in[1] return item` is a for expression).
  if matches!(&reference.outcome, Outcome::Panic(message) if message == "attempt to subtract with overflow" || message.ends_with(&format!("the index is {}", usize::MAX)))
    && production.tokens.starts_with(&reference.tokens)
    && production
      .tokens
      .get(reference.tokens.len())
      .is_some_and(|(tt, tv, _)| *tt == TokenType::In as i16 && tv == "In")
    && (matches!(&production.outcome, Outcome::Error(message) if message.starts_with("<ParserError> syntax error")) || has_item_variable(&reference.tokens))
  {
    return Some("a name starting with `in` in iteration-variable mode");
  }
  // A named parameter of a temporal function in a range literal (`[date(from: "2012-01-01")..x]`):
  // - reference: the parameter name was read from the slot of the colon, so the invocation had
  //   `NamedParameters([String(..)])` (no name; the range() built-in returned null);
  // - production: `NamedParameters([NamedParameter(ParameterName(..), String(..))])`, with the
  //   name token before the colon; otherwise the same AST.
  if start == "range literal"
    && reference.tokens == production.tokens
    && let (Outcome::Ast(reference_ast), Outcome::Ast(production_ast)) = (&reference.outcome, &production.outcome)
    && let (Some((_, reference_debug)), Some((_, production_debug))) = (reference_ast.rsplit_once('\n'), production_ast.rsplit_once('\n'))
    && reference_debug != production_debug
    && with_named_parameters(reference_debug, &reference.tokens).is_some_and(|expected| expected == production_debug)
  {
    return Some("a named temporal parameter in a range literal");
  }
  None
}

/// Returns `true` when the tokens have an iteration variable `item`: a name `item` right after
/// `for`, `some`, `every` or a comma.
fn has_item_variable(tokens: &[(i16, String, usize)]) -> bool {
  let heads = [TokenType::For, TokenType::Some, TokenType::Every, TokenType::Comma].map(|tt| tt as i16);
  tokens.windows(2).any(|pair| heads.contains(&pair[0].0) && pair[1].1 == r#"Name(Name("item"))"#)
}

/// Returns the Debug text of a range literal's AST with the parameter names put into its named
/// parameters: every `NamedParameters([String(s)])` becomes
/// `NamedParameters([NamedParameter(ParameterName(name), String(s))])`, with the name of the
/// name token before the corresponding colon. `None` when the names and parameters do not pair up.
fn with_named_parameters(debug: &str, tokens: &[(i16, String, usize)]) -> Option<String> {
  const PARAMETERS: &str = "NamedParameters([String(";
  let mut names = tokens
    .windows(2)
    .filter(|pair| pair[1].0 == TokenType::Colon as i16)
    .map(|pair| pair[0].1.strip_prefix("Name(").and_then(|name| name.strip_suffix(')')));
  let mut text = String::new();
  let mut rest = debug;
  while let Some(at) = rest.find(PARAMETERS) {
    let name = names.next()??;
    text.push_str(&rest[..at]);
    rest = &rest[at + PARAMETERS.len()..];
    let literal = debug_string_len(rest)?;
    text.push_str(&format!("NamedParameters([NamedParameter(ParameterName({name}), String({}))", &rest[..literal]));
    rest = rest[literal..].strip_prefix(")")?;
  }
  if names.next().is_some() {
    return None;
  }
  text.push_str(rest);
  Some(text)
}

/// Returns the length in bytes of the Debug text of a string at the start of `text` (quotes
/// included), or `None` when `text` does not start with one.
fn debug_string_len(text: &str) -> Option<usize> {
  let mut chars = text.char_indices();
  if chars.next()?.1 != '"' {
    return None;
  }
  while let Some((offset, ch)) = chars.next() {
    match ch {
      '\\' => {
        chars.next();
      }
      '"' => return Some(offset + 1),
      _ => {}
    }
  }
  None
}

/// Statistics of a differential run.
#[derive(Default, Debug)]
struct Stats {
  cases: usize,
  asts: usize,
  errors: usize,
  registered: BTreeMap<&'static str, usize>,
}

impl Stats {
  /// Compares both front ends on one input; panics on an unregistered difference.
  fn check(&mut self, frames: &[FeelContext], input: &str) {
    for (name, start, reference_start) in &START_RULES {
      self.check_rule(frames, name, start, reference_start, input);
    }
  }

  fn check_rule(&mut self, frames: &[FeelContext], name: &str, start: &TokenType, reference_start: &oracle::lalr::TokenType, input: &str) {
    let production = run_production(frames, start, input);
    let reference = run_reference(frames, reference_start, input);
    // a panic of the production front end fails in compare
    if let Some(fix) = compare(name, frames, input, &reference, &production) {
      *self.registered.entry(fix).or_default() += 1;
    }
    self.cases += 1;
    if matches!(production.outcome, Outcome::Ast(_)) {
      self.asts += 1;
    } else {
      self.errors += 1;
    }
  }
}

/// Compares a run of the production front end with a run of the reference on the same input,
/// start rule `start` and scope `frames`. Returns `None` when both read the same tokens and have
/// the same outcome, and the name of the fix when they differ by a registered difference
/// ([registered_difference]); panics when the production front end panicked or the difference is
/// not registered.
pub(super) fn compare(start: &str, frames: &[FeelContext], input: &str, reference: &Run, production: &Run) -> Option<&'static str> {
  if let Outcome::Panic(message) = &production.outcome {
    panic!("{start} {input:?} in {frames:?}: the parser panicked: {message}");
  }
  if (&production.tokens, &production.outcome) == (&reference.tokens, &reference.outcome) {
    return None;
  }
  match registered_difference(start, reference, production) {
    Some(fix) => Some(fix),
    None => panic!("{start} {input:?} in {frames:?}:\nreference {reference:#?}\nproduction {production:#?}"),
  }
}

fn ctx(entries: &[(&str, Value)]) -> FeelContext {
  let mut ctx = FeelContext::default();
  for (key, value) in entries {
    ctx.set_entry(&Name::from(*key), value.clone());
  }
  ctx
}

/// The scopes of the short strings (each a list of input contexts, bottom first).
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

/// Every registered difference is registered for the inputs of its fix: the reference traps on
/// a name starting with `in` in iteration-variable mode, and drops the name of a named temporal
/// parameter in a range literal.
#[test]
fn registered_fixes_differ_as_registered() {
  let in_names = [
    "for in+1 in [1] return 1",
    "for in+1 in [1] return in+1",
    "for in[1] return 1",
    "for in(1) return 1",
    "for in.x in [1] return 1",
    "for in: 1 return 1",
    "for x in [1], in*2 in [2] return x",
    r#"some in"a" satisfies true"#,
    "every in-x in [] satisfies false",
    "for item in [1] return in+1",
    "for item in, x in [1] return 1",
    "for item in[1] return item",
    "some item in[1] satisfies item > 0",
  ];
  let range_literals = [
    r#"[date(from: "2012-01-01")..date(from: "2012-12-31")]"#,
    r#"(date and time(from: "2012-01-01T10:00:00")..10]"#,
    r#"]"a"..duration(from: "P1D\"x")["#,
  ];
  for (inputs, rule, fix) in [
    (&in_names[..], 0, "a name starting with `in` in iteration-variable mode"),
    (&range_literals[..], 8, "a named temporal parameter in a range literal"),
  ] {
    let (name, start, reference_start) = &START_RULES[rule];
    for input in inputs {
      let mut stats = Stats::default();
      stats.check_rule(&[], name, start, reference_start, input);
      assert_eq!(Some(&1), stats.registered.get(fix), "{input}: {stats:?}");
    }
  }
}

/// The length of the longest short strings.
fn short_string_length() -> usize {
  if full_size() { 4 } else { 2 }
}

#[test]
fn short_strings_equal_reference() {
  let alphabets: &[&[char]] = if full_size() { &[&ALPHABET, &ALPHABET_2] } else { &[&ALPHABET] };
  let mut stats = Stats::default();
  for alphabet in alphabets {
    for frames in scopes() {
      for len in 0..=short_string_length() {
        for_each_string(alphabet, len, |text| stats.check(&frames, text));
      }
    }
  }
  println!("short strings: {stats:?}");
  assert!(stats.asts > 1000, "{stats:?}");
}

#[test]
fn short_strings_after_iteration_heads_equal_reference() {
  // the heads that set the lexer's `till_in` flag, followed by every short string
  let max_len = short_string_length();
  let mut stats = Stats::default();
  for frames in &scopes()[..3] {
    for head in ["for ", "some ", "every ", "for x in [1], ", "for item in [1] return "] {
      for len in 0..=max_len {
        for_each_string(&ALPHABET, len, |text| {
          let input = format!("{head}{text}");
          for (name, start, reference_start) in &START_RULES[..1] {
            stats.check_rule(frames, name, start, reference_start, &input);
          }
          let input = format!("{head}{text} in [1] return 1");
          for (name, start, reference_start) in &START_RULES[..1] {
            stats.check_rule(frames, name, start, reference_start, &input);
          }
        });
      }
    }
  }
  println!("short strings after iteration heads: {stats:?}");
}

/// Random FEEL texts: every construct of the grammar, names of the input contexts written in
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

/// The keys of the scopes of the random texts, with names that the generated texts use too.
fn random_text_keys() -> Vec<String> {
  let mut keys: Vec<String> = KEYS.iter().map(|k| k.to_string()).collect();
  keys.extend(["zz", "in x", "a in b", "date", "number", "string length", "Full Name"].map(String::from));
  keys
}

#[test]
fn random_texts_equal_reference() {
  let count = if full_size() { 20_000 } else { 1_000 };
  let mut rng = Rng(0x853C_49E6_748F_EA9B);
  let keys = random_text_keys();
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
  println!("random texts: {stats:?}");
  assert!(stats.asts > count, "{stats:?}");
}

/// Random edits of a text: deletions, insertions of characters and keywords, duplications.
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
fn edited_and_cut_texts_equal_reference() {
  let count = if full_size() { 3_000 } else { 60 };
  let mut rng = Rng(0xDA94_2042_E4DD_58B5);
  let keys = random_text_keys();
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
  println!("edited and cut texts: {stats:?}");
}

/// An input context with the keys of a FEEL context literal (the values are numbers, nested
/// literals become nested contexts), parsed by the production parser.
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

/// The expressions and context literals of the evaluator corpus: the `*.jsonl` files of the
/// directory `RHEON_PARSER_CORPUS` (an absolute path, such as that of `tools/evaluator-corpus/corpora`).
fn corpus() -> Vec<(String, String)> {
  let Some(dir) = std::env::var_os("RHEON_PARSER_CORPUS") else {
    return vec![];
  };
  let mut lines = vec![];
  let mut files: Vec<_> = std::fs::read_dir(&dir).expect("RHEON_PARSER_CORPUS").map(|e| e.unwrap().path()).collect();
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
fn corpus_equals_reference() {
  let corpus = corpus();
  assert!(!corpus.is_empty(), "set RHEON_PARSER_CORPUS to the absolute path of tools/evaluator-corpus/corpora");
  let mut rng = Rng(0x1234_5678_9ABC_DEF1);
  let rich = context(&mut rng, 0);
  let mut stats = Stats::default();
  for (ctx_text, expr) in &corpus {
    for frames in [vec![], vec![keys_of_context_literal(ctx_text)], vec![rich.clone()]] {
      stats.check(&frames, expr);
    }
    // cut-off and edited corpus texts (as in [edited_and_cut_texts_equal_reference])
    let frames = vec![keys_of_context_literal(ctx_text)];
    stats.check(&frames, &mutate(&mut rng, expr));
    if rng.chance(5) {
      for (end, _) in expr.char_indices() {
        stats.check(&frames, &expr[..end]);
      }
    }
  }
  println!("corpus: {} lines, {stats:?}", corpus.len());
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

/// The work counters of a parse (`work!`).
fn work_of(frames: &[FeelContext], input: &str) -> BTreeMap<&'static str, usize> {
  super::work::take();
  let scope = ParsingScope::new(frames);
  let _ = Parser::new(&scope, TokenType::StartExpression, input, false).parse();
  super::work::take()
}

/// Every kind of list keeps the source order of its items, at every length: the AST equals the
/// reference's for up to 100 items (1,000 in full size), and the items are in order for 2,000
/// (20,000; the reference shifts every collected item for each new one, so it is not run there).
#[test]
fn lists_keep_source_order() {
  let (compared, ordered) = if full_size() { (1_000, 20_000) } else { (100, 2_000) };
  let join = |n: usize, item: &dyn Fn(usize) -> String| (0..n).map(item).collect::<Vec<_>>().join(", ");
  let texts = |n: usize| -> Vec<(usize, String)> {
    vec![
      (0, format!("{{{}}}", join(n, &|i| format!("k{i}: {i}")))),
      (0, format!("[{}]", join(n, &|i| i.to_string()))),
      (0, format!("f({})", join(n, &|i| i.to_string()))),
      (0, format!("f({})", join(n, &|i| format!("p{i}: {i}")))),
      (0, format!("x in ({})", join(n, &|i| i.to_string()))),
      (0, format!("x instance of context<{}>", join(n, &|i| format!("k{i}: number")))),
      (
        0,
        format!(
          "x instance of function<{}> -> number",
          join(n, &|i| if i % 2 == 0 { format!("t{i}") } else { "number".to_string() })
        ),
      ),
      (0, format!("for {} return 0", join(n, &|i| format!("v{i} in [{i}]")))),
      (0, format!("some {} satisfies true", join(n, &|i| format!("v{i} in [{i}]")))),
      (0, format!("every {} satisfies true", join(n, &|i| format!("v{i} in [{i}]")))),
      (4, join(n, &|i| i.to_string())),
      (4, format!("not({})", join(n, &|i| i.to_string()))),
      (3, join(n, &|i| format!("{i} + 1"))),
      (6, join(n, &|i| format!("{i} * 2"))),
    ]
  };
  // `x` and `f` are names in scope (else `x instance of context` would be one name)
  let frames = [ctx(&[("x", number(1)), ("f", number(2))])];
  for n in [1, 2, 3, 10, compared] {
    let mut texts = texts(n);
    // qualified names (a chain of names is lexed again for every name token)
    let path = (0..n).map(|i| format!("q{i}")).collect::<Vec<_>>().join(".");
    texts.push((0, format!("x instance of {path}")));
    texts.push((7, path));
    for (rule, text) in &texts {
      let (name, start, reference_start) = &START_RULES[*rule];
      let mut stats = Stats::default();
      stats.check_rule(&frames, name, start, reference_start, text);
      assert_eq!(1, stats.asts, "{n} items: {text:.80}");
    }
  }
  // the longest lists: in source order
  for (rule, text) in texts(ordered) {
    let scope = ParsingScope::new(&frames);
    let node = Parser::new(&scope, START_RULES[rule].1.clone(), &text, false).parse().unwrap();
    let debug = format!("{node:?}");
    // the first `{i}"` is the item i (`"{i}"`, `"k{i}"`, ...): other texts ending with these digits are larger numbers
    let positions: Vec<usize> = (0..ordered)
      .step_by(ordered / 20 - 2)
      .map(|i| debug.find(&format!("{i}\"")).unwrap_or_else(|| panic!("{i} in {text:.80}")))
      .collect();
    assert!(positions.is_sorted(), "{text:.80}");
  }
}

/// Name tokens that reuse a scanned chain (see `ChainPart` in the lexer) read the same tokens as the
/// reference, which scans the chain for every name token: chains of up to 50 names (300 in full size).
#[test]
fn reused_name_chains_equal_reference() {
  let mut inputs = vec![];
  for n in [1, 2, 3, 10, if full_size() { 300 } else { 50 }] {
    inputs.push(vec!["a"; n].join("+"));
    inputs.push(vec!["a"; n].join(" + "));
    inputs.push(vec!["a"; n].join("."));
    inputs.push(vec!["a b"; n].join(" - "));
    inputs.push((0..n).map(|i| format!("x{i}")).collect::<Vec<_>>().join(" * "));
  }
  for text in [
    "a /*x*/ + b",
    "a //c\n+ b",
    "a /* ( */ + b",
    "a * 1.5e3x",
    "a * 0 + b",
    "a*1e+b",
    "a + if b then a else b",
    "a + for x in b return x",
    "a b + not(a) + item",
    "item.a + a",
    "for a in b return a + b",
    "for item in a return item + b",
    "{a: a + b, b: a.b + a}",
    "a + b: 1",
    "f(a: a + b)",
    "x instance of number + date",
    "a + date(\"2012-01-01\") + date",
    "a\u{1680}.\u{1680}b + a",
    "q + a\u{1680}.\u{1680} b",
    "a\u{180E}b + a\u{FEFF}b + a",
    "a.\u{180E}b + a",
    "a+in\u{1680}b",
    "a + b.c.d + b . c",
    "a.b + start included",
    "a.b + time offset",
    "a b c d + a b",
    "a +",
    "a + b ",
    "a + b.",
    "a + b..",
    "a + b /",
    "a + b \u{1680}",
  ] {
    inputs.push(text.to_string());
  }
  let mut stats = Stats::default();
  for frames in scopes() {
    for input in &inputs {
      stats.check(&frames, input);
    }
  }
  println!("reused chains: {stats:?}");
}

/// A chain of name tokens is scanned once: the parts scanned grow linearly with the length.
#[test]
fn name_chains_are_scanned_once() {
  let scope = [ctx(&[("a", number(1)), ("b", Value::Context(ctx(&[("b", number(2))])))])];
  for (text, parts) in [(vec!["a"; 20_000].join(" + "), 39_999), (format!("b{}", ".b".repeat(19_999)), 39_999)] {
    assert_eq!(Some(&parts), work_of(&scope, &text).get("parts"), "{text:.20}");
  }
}

/// The candidate names a name token looks up are bounded by the longest name in scope: in an
/// `a + a + …` chain with `a` in scope, every name token but the last (a single part, looked up
/// by no candidate) looks up one candidate, whatever the length of the rest of the chain.
#[test]
fn candidates_grow_linearly_with_the_chain() {
  let scope = [ctx(&[("a", number(1))])];
  let terms = 1_000;
  let text = vec!["a"; terms].join(" + ");
  let candidates = work_of(&scope, &text).get("candidates").copied().unwrap_or(0);
  assert!(candidates <= terms, "{candidates} candidates for {terms} terms");
}

/// The bound of the candidate names by the first word of the token (`ParsingScope::group_bound`):
/// keys that share their first word, keys with a byte below `0` after a word, more keys with
/// the same first word than the window looks at, long multi-word keys, list item keys, nested
/// keys, parsed names, parameters and built-in properties after a dot.
#[test]
fn first_word_bounds_equal_reference() {
  let mut keys: Vec<String> = ["a", "a b", "a.b", "a!b", "a\tb", "a0", "ab c", "a b c d e f g h", "x", "x y", "start", "start x", "end"]
    .map(String::from)
    .to_vec();
  keys.extend((0..40).map(|i| format!("a/{i}")));
  keys.push("monthly salary amount before tax and social security deductions".to_string());
  keys.push("monthly".to_string());
  let top = ctx(&keys.iter().map(|k| (k.as_str(), number(1))).collect::<Vec<_>>());
  let nested = ctx(&[
    ("n", Value::Context(ctx(&[("p q", number(1)), ("p", number(2)), ("q r s", number(3))]))),
    ("items", Value::List(vec![Value::Context(ctx(&[("u v", number(1)), ("u", number(2))]))])),
    ("d", Value::Context(ctx(&[("a", Value::Context(ctx(&[("b", number(1))])))]))),
  ]);
  let inputs = [
    "a + a b + a.b + a!b + a\tb + a0 + ab c",
    "a b c d e f g h + a b c d + a / 3 + a/39 + a/0 - a",
    "monthly salary amount before tax and social security deductions * 2 - monthly salary",
    "x y + x + p q + q r s + u v + u + n.p q + items.u v",
    "d.a.b + start + start x + end",
    "d.a.b + start included + time offset + end included",
    "d.a.b + x y + start x",
    "{p q: 1, p q r: 2, r: p q + p q r}",
    "{a b c: 1, a: a b c + a b}.a",
    "function(p q, x y z) p q + x y z",
    "for p q in [1], r s in [2] return p q + r s",
    "some u v in items satisfies u v > a",
    "x instance of context<p q: number>",
    "date(\"2012-01-01\").year + d.a.b + time offset",
    "a\u{1680} b + a\u{FEFF} b + a\u{180E}b",
  ];
  let mut stats = Stats::default();
  for frames in [vec![top.clone()], vec![nested.clone()], vec![top, nested], vec![]] {
    for input in inputs {
      stats.check(&frames, input);
    }
  }
  println!("first-word bounds: {stats:?}");
}

/// The input contexts are indexed only when a name is looked up beyond their own keys.
#[test]
fn input_contexts_are_indexed_on_first_use() {
  let items = Value::List(vec![Value::Context(ctx(&[("amount", number(1))])); 3]);
  let frames = [ctx(&[("a", Value::Context(ctx(&[("b", number(1))]))), ("c", number(2)), ("items", items)])];
  for (input, builds) in [
    ("1 + 2", None),
    ("{x: 1, y: \"s\"}", None),
    ("sum(items[amount > 100].amount)", None),
    ("a + c", Some(&1)),
    ("a.b * 2", Some(&1)),
  ] {
    assert_eq!(builds, work_of(&frames, input).get("index builds"), "{input}");
  }
}

/// One-part names skip the lookup unless the scope decides the token (`consume_name`): built-in
/// property names after a dot, built-in type names after `instance of`, temporal function names
/// before `(`; also with U+1680 (trimmed) and U+180E (not trimmed) inside the part.
#[test]
fn one_part_names_equal_reference() {
  let frames_d = vec![ctx(&[
    ("d", Value::Context(ctx(&[("year", number(1))]))),
    ("x", number(2)),
    ("date", number(3)),
    ("number", number(4)),
  ])];
  let mut stats = Stats::default();
  for frames in [vec![], frames_d] {
    for ws in ["\u{1680}", "\u{180E}", "", " "] {
      for input in [
        format!("[d.year{ws}, start-1]"),
        format!("date{ws}(\"2020-01-01\")"),
        format!("x instance of number{ws}"),
        format!("x instance of number{ws} and date{ws}(\"2020-01-01\") > x"),
        format!("d.year{ws} + start + 1"),
        format!("d . year{ws}.start"),
        format!("date{ws} + time{ws}(\"10:00:00\") + duration"),
        format!("[x{ws}, number{ws}, string{ws}]"),
        format!("x // c\n{ws}+ d"),
        format!("x{ws}"),
      ] {
        stats.check(&frames, &input);
      }
    }
  }
  println!("one-part names: {stats:?}");
}

/// String and number literals (taken in one allocation when they need no unescaping): escapes,
/// surrogates, unterminated literals, vertical space, long literals, every form of number.
#[test]
fn literals_equal_reference() {
  let long = "x\u{1F600}é".repeat(3_000);
  let mut inputs: Vec<String> = [
    r#""""#,
    r#""a""#,
    r#""\"""#,
    r#""\\""#,
    r#""\n\r\t\'""#,
    r#""A\U01F600""#,
    r#""🙀""#,
    r#""\uD83D""#,
    r#""\uDE40""#,
    r#""\uD83Dx""#,
    r#""\u12G4""#,
    r#""\x""#,
    r#""abc"#,
    r#""ab\"#,
    "\"a\nb\"",
    "\"a\rb\"",
    "\"a\u{2028}b\"",
    r#""a" + "b\"c" + "d""#,
    r#""2026-03-01T10:00:00+01:00""#,
    ".5",
    "5.",
    "5.e3",
    "1e",
    "1e+",
    "1e-",
    "1E-3",
    "1e+30",
    "007",
    "0.000001",
    "1.5e3x",
    "12345678901234567890123456789012345",
    "12345678901234567890.12345678901234567890e12345678901234567890",
    "1..2",
    "1.2.3",
  ]
  .map(String::from)
  .to_vec();
  inputs.push(format!("\"{long}\""));
  inputs.push(format!("\"{long}"));
  inputs.push(format!("\"{long}\\n{long}\""));
  inputs.push("9".repeat(10_000));
  inputs.push(format!("1.{}e-{}", "5".repeat(5_000), "3".repeat(5_000)));
  let mut stats = Stats::default();
  for input in &inputs {
    stats.check(&[], input);
    stats.check(&[], &format!("[{input}, {input}]"));
  }
  println!("literals: {stats:?}");
}

/// U+1680 between words: after a symbol it starts a part (empty after trimming when alone), after
/// white space it is white space; so two empty parts never follow each other.
#[test]
fn empty_parts_equal_reference() {
  let mut stats = Stats::default();
  let frames = [ctx(&[("x", number(1)), ("x y", number(2)), ("a", number(3)), (&format!("x {OGHAM} y"), number(4))])];
  for n in [0, 1, 2, 3, 5, 20] {
    let run = format!(" {OGHAM}").repeat(n);
    for input in [
      format!("x{run} y"),
      format!("x{run}"),
      format!("x{run} y{run} + a"),
      format!("x{run}.{run}y + 1"),
      format!("x {OGHAM}.{OGHAM}{run} y"),
      format!("a{run} + x{run} y{run} * a"),
    ] {
      stats.check(&frames, &input);
      stats.check(&[], &input);
    }
  }
  println!("empty parts: {stats:?}");
}

/// Composite names into deep input contexts are searched without recursion: a 2,000-level chain
/// (built and dropped in a thread with a large stack: values are cloned and dropped recursively),
/// parsed in a thread with a small stack.
#[test]
fn composite_names_in_deep_contexts_parse_in_a_small_stack() {
  let depth = 2_000;
  std::thread::Builder::new()
    .stack_size(256 * 1024 * 1024)
    .spawn(move || {
      let frames = vec![super::names_index::chain(depth)];
      std::thread::scope(|scope| {
        std::thread::Builder::new()
          .stack_size(256 * 1024)
          .spawn_scoped(scope, || {
            let parsing_scope = ParsingScope::new(&frames);
            let input = format!("a./{OGHAM}.{OGHAM} leaf + 1");
            let node = Parser::new(&parsing_scope, TokenType::StartExpression, &input, false).parse().unwrap();
            assert!(matches!(&node, crate::AstNode::Add(lhs, _) if matches!(lhs.as_ref(), crate::AstNode::Name(name) if name.as_str() == "a./. leaf")));
          })
          .unwrap()
          .join()
          .unwrap();
      });
    })
    .unwrap()
    .join()
    .unwrap();
}

/// Random texts of multi-byte characters (the lexer reads the input in place, by byte offsets):
/// names of 2-, 3- and 4-byte characters and combining marks, every kind of white space, CRLF,
/// comments, escapes, unterminated strings and comments, keywords at the end of the input.
#[test]
fn unicode_texts_equal_reference() {
  const PIECES: [&str; 40] = [
    "é",
    "中",
    "😀",
    "a\u{0300}",
    "\u{00B7}",
    "x",
    "in",
    "for ",
    "if ",
    "then ",
    "else ",
    "not",
    "and ",
    "or",
    " ",
    "\t",
    "\r\n",
    "\u{00A0}",
    "\u{0085}",
    "\u{1680}",
    "\u{180E}",
    "\u{2003}",
    "\u{2028}",
    "\u{3000}",
    "\u{FEFF}",
    "//c\n",
    "/*é*/",
    "/*",
    "\"é",
    "\"中\\\"\"",
    "\"\\u00E9\"",
    "\"\\uD83D\\uDE00\"",
    ".",
    ":",
    "(",
    ")",
    "+",
    "1",
    "1.5e3",
    "@\"",
  ];
  let count = if full_size() { 20_000 } else { 1_000 };
  let mut rng = Rng(0x6A09_E667_F3BC_C908);
  let frames = [ctx(&[("é", number(1)), ("中 文", number(2)), ("😀", number(3)), ("x é", number(4))])];
  let mut stats = Stats::default();
  for _ in 0..count {
    let text: String = (0..1 + rng.below(12)).map(|_| *rng.pick(&PIECES)).collect();
    stats.check(&frames, &text);
  }
  println!("unicode texts: {stats:?}");
}
