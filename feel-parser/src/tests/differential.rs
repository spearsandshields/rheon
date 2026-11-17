//! Tests of the lexer and parser against their recorded outcomes ([Golden]).
//!
//! For every input, start rule and scope, the front end must not panic, and must read the
//! recorded tokens (Debug text of the value, start offset of the token in the input) and give
//! the recorded AST (tree and Debug text) or the recorded error. A change of an outcome is a
//! change of an answer: the recorded outcomes are rewritten in the commit that makes it
//! (`RHEON_GOLDEN=update`, at the default and at the full size), and the commit says why.
//!
//! Generators (default size, and full size with `RHEON_FULL_TESTS` set, see [super::full_size]):
//! - short strings: every string of up to 2 characters (4) over [ALPHABET] (and over
//!   [ALPHABET_2]), in [scopes], with every start rule; also after the heads of iteration contexts.
//! - random texts: random FEEL texts of every construct ([Feel]), with random input contexts (from
//!   [super::names_index]): 1,000 (20,000).
//! - corpus: the expressions of the FEEL corpus, run only when `RHEON_PARSER_CORPUS` is the
//!   absolute path of a directory of `*.jsonl` files (`tools/feel-corpus`; the test
//!   is ignored by default: `-- --include-ignored`).
//! - edited and cut texts: random edits and every prefix of random texts and corpus texts (cut-off
//!   inputs, syntax errors): 60 (3,000).
//! - unicode texts: random texts of multi-byte characters: 1,000 (20,000).
//!
//! The other tests cover chosen inputs; some of them use longer inputs in full size. The full
//! size is meant for a release build: `RHEON_FULL_TESTS=1 cargo test --release -p rheon-feel-parser`.

use super::full_size;
use super::names_index::{KEYS, OGHAM, Rng, context, key_text, number};
use crate::lalr::TokenType;
use crate::parser::Parser;
use crate::scope::ParsingScope;
use rheon_feel::context::FeelContext;
use rheon_feel::values::Value;
use rheon_feel::{FeelType, Name};
use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// The start rules of the grammar.
const START_RULES: [(&str, TokenType); 9] = [
  ("expression", TokenType::StartExpression),
  ("boxed expression", TokenType::StartBoxedExpression),
  ("textual expression", TokenType::StartTextualExpression),
  ("textual expressions", TokenType::StartTextualExpressions),
  ("unary tests", TokenType::StartUnaryTests),
  ("simple expression", TokenType::StartSimpleExpression),
  ("simple expressions", TokenType::StartSimpleExpressions),
  ("simple value", TokenType::StartSimpleValue),
  ("range literal", TokenType::StartRangeLiteral),
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

/// Statistics of a run over a group of cases.
#[derive(Default, Debug)]
struct Stats {
  cases: usize,
  asts: usize,
  errors: usize,
  /// The recorded outcomes of the group of cases, when the run is one ([Stats::golden]).
  golden: Option<Golden>,
}

/// The recorded outcomes of a group of generated cases (the golden data of the front end): for
/// every case of the group, in the order in which the test makes them, a digest of what the
/// parser did with it (the start rule, the input, the tokens it read and the tree or the
/// error). The digests are kept in `src/tests/golden/<group>.txt`, one per line, for the
/// default size of the tests; for the full size (`RHEON_FULL_TESTS`) one digest of all cases is
/// kept in `<group>.full.txt`.
///
/// A case whose outcome is not the recorded one fails the test, which names the first of them
/// with their new outcome. After a change that is meant, `RHEON_GOLDEN=update` writes the files
/// again. `RHEON_GOLDEN_DUMP=<directory>` writes every case with its whole outcome to
/// `<directory>/<group>.txt`, to compare two commits case by case.
#[derive(Default)]
pub(super) struct Golden {
  group: &'static str,
  /// The recorded digests (none when the files are written again or at full size).
  expected: Vec<u64>,
  recorded: Vec<u64>,
  /// The number of cases with another outcome than the recorded one, and the first of them.
  changed: usize,
  first_changed: Vec<String>,
  dump: Option<String>,
}

impl std::fmt::Debug for Golden {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{} recorded, {} changed", self.recorded.len(), self.changed)
  }
}

/// FNV-1a over the bytes of a text: a digest that does not depend on the toolchain.
fn digest(text: &str) -> u64 {
  text
    .bytes()
    .fold(0xCBF2_9CE4_8422_2325, |hash, byte| (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01B3))
}

impl Golden {
  fn file(group: &str, full: bool) -> std::path::PathBuf {
    let name = if full { format!("{group}.full.txt") } else { format!("{group}.txt") };
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tests/golden").join(name)
  }

  fn updating() -> bool {
    std::env::var("RHEON_GOLDEN").is_ok_and(|value| value == "update")
  }

  pub(super) fn new(group: &'static str) -> Self {
    let expected = if Self::updating() || full_size() {
      vec![]
    } else {
      let path = Self::file(group, false);
      let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}; write it with RHEON_GOLDEN=update", path.display()));
      text
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| u64::from_str_radix(line, 16).unwrap_or_else(|e| panic!("{}: {line}: {e}", path.display())))
        .collect()
    };
    Self {
      group,
      expected,
      dump: std::env::var_os("RHEON_GOLDEN_DUMP").map(|_| String::new()),
      ..Default::default()
    }
  }

  pub(super) fn record(&mut self, start: &str, input: &str, run: &Run) {
    // the tokens by their values and offsets, without the numbers the grammar gives their types
    let tokens: Vec<(&str, usize)> = run.tokens.iter().map(|(_, value, offset)| (value.as_str(), *offset)).collect();
    let outcome = format!("{start}\u{1f}{input}\u{1f}{tokens:?}\u{1f}{:?}", run.outcome);
    self.record_outcome(start, input, &outcome, &|| format!("{run:#?}"));
  }

  /// Records the outcome `outcome` of a case; `shown` is its text for a person.
  pub(super) fn record_outcome(&mut self, start: &str, input: &str, outcome: &str, shown: &dyn Fn() -> String) {
    let index = self.recorded.len();
    let recorded = digest(outcome);
    self.recorded.push(recorded);
    if self.expected.get(index).is_some_and(|expected| *expected != recorded) {
      self.changed += 1;
      if self.first_changed.len() < 5 {
        self.first_changed.push(format!("case {index}, {start}, {input:?}:\n{}", shown()));
      }
    }
    if let Some(dump) = &mut self.dump {
      dump.push_str(&format!("case {index}, {start}, {input:?}:\n{}\n", shown()));
    }
  }

  pub(super) fn finish(&self) {
    let full = full_size();
    if let (Some(dump), Some(directory)) = (&self.dump, std::env::var_os("RHEON_GOLDEN_DUMP")) {
      let path = std::path::Path::new(&directory).join(format!("{}{}.txt", self.group, if full { ".full" } else { "" }));
      std::fs::write(&path, dump).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    }
    // one digest of all cases, for the full size
    let all = digest(&self.recorded.iter().map(|digest| format!("{digest:016x}")).collect::<String>());
    let path = Self::file(self.group, full);
    if Self::updating() {
      let text = if full {
        format!("# {}: {} cases at full size, one digest of all\n{all:016x}\n", self.group, self.recorded.len())
      } else {
        let digests: String = self.recorded.iter().map(|digest| format!("{digest:016x}\n")).collect();
        format!("# {}: {} cases, a digest of the outcome of each\n{digests}", self.group, self.recorded.len())
      };
      std::fs::write(&path, text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
      return;
    }
    if full {
      // the full size is recorded when it was written once
      if let Ok(text) = std::fs::read_to_string(&path) {
        let expected = text.lines().find(|line| !line.starts_with('#')).unwrap_or_default();
        assert_eq!(
          expected,
          format!("{all:016x}"),
          "{}: the outcomes of the {} cases at full size are not the recorded ones",
          self.group,
          self.recorded.len()
        );
      }
      return;
    }
    assert!(
      self.changed == 0 && self.expected.len() == self.recorded.len(),
      "{}: {} of {} cases have another outcome than the recorded one ({} recorded); after a change that is meant, write the files again with RHEON_GOLDEN=update. The first:\n{}",
      self.group,
      self.changed,
      self.recorded.len(),
      self.expected.len(),
      self.first_changed.join("\n")
    );
  }
}

impl Stats {
  /// The statistics of a group of cases whose outcomes are recorded ([Golden]).
  fn golden(group: &'static str) -> Self {
    Self {
      golden: Some(Golden::new(group)),
      ..Default::default()
    }
  }

  /// Compares the outcomes of the group with the recorded ones.
  fn finish(&self) {
    if let Some(golden) = &self.golden {
      golden.finish();
    }
  }

  /// Parses one input with every start rule; panics when the parser panics, and records the
  /// outcomes.
  fn check(&mut self, frames: &[FeelContext], input: &str) {
    for (name, start) in &START_RULES {
      self.check_rule(frames, name, start, input);
    }
  }

  fn check_rule(&mut self, frames: &[FeelContext], name: &str, start: &TokenType, input: &str) {
    let production = run_production(frames, start, input);
    if let Outcome::Panic(message) = &production.outcome {
      panic!("{name} {input:?} in {frames:?}: the parser panicked: {message}");
    }
    if let Some(golden) = &mut self.golden {
      golden.record(name, input, &production);
    }
    self.cases += 1;
    if matches!(production.outcome, Outcome::Ast(_)) {
      self.asts += 1;
    } else {
      self.errors += 1;
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

/// The scopes of the short strings (each a list of input contexts, bottom first).
fn scopes() -> Vec<Vec<FeelContext>> {
  let nested = |key: &str, inner: FeelContext| ctx(&[(key, Value::from(inner))]);
  let mut rng = Rng(0x2545_F491_4F6C_DD1D);
  vec![
    vec![],
    vec![ctx(&[("a", number(1))])],
    vec![ctx(&[
      ("in", number(1)),
      ("in+1", number(2)),
      ("a b", number(3)),
      ("x", Value::from(ctx(&[("y", number(1))]))),
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
      ("list", Value::List(vec![Value::from(ctx(&[("a", number(1))]))].into())),
      ("t", Value::FeelType(FeelType::Context(BTreeMap::from([(Name::from("b"), FeelType::Number)])))),
    ])],
    vec![
      ctx(&[("a", number(1)), (&format!("a{OGHAM}b"), number(2))]),
      ctx(&[("b", Value::from(ctx(&[("a", number(2)), ("i n", number(3))])))]),
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

/// The inputs of the fixes of the front end have their recorded outcomes: names starting with
/// `in` where an iteration variable is read, named temporal parameters in a range literal,
/// string literals and block comments that are not closed, exponent letters without digits,
/// unary comparisons with arithmetic after them, and signed and missing endpoints of a range
/// literal.
#[test]
fn inputs_of_the_fixes_have_their_recorded_outcome() {
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
  let signed = ["[-1..1]", "(-2.5..-1)", "]-1e3..0[", "[0..-1]", "(..)", "]..[", "(..[", "]..)"];
  let endpoints = [
    "> 1 + 2",
    "< a * 2",
    "[>= 1 - 1, 5]",
    "= 1 in [1]",
    "f(> 1 + 1)",
    "!= 2 ** 2",
    "> 1 < 2",
    "<= 10 = (null..10]",
  ];
  let exponents = ["1e", "1e+", "1E-", "1.5e", "2e x", "1e + 2", "1else", "[1e, 2]", "1e+ 3"];
  let unclosed = ["1 + 2 \"abc", "\"", "1 \"a\\\"b", "1 /* abc", "/*", "1 /* a */ + 2 /* b *", "1 /*/"];
  let mut stats = Stats::golden("fixes");
  for (inputs, rule) in [
    (&in_names[..], 0),
    (&range_literals[..], 8),
    (&unclosed[..], 0),
    (&exponents[..], 0),
    (&endpoints[..], 0),
    (&signed[..], 8),
  ] {
    let (name, start) = &START_RULES[rule];
    for input in inputs {
      stats.check_rule(&[], name, start, input);
    }
  }
  println!("fixes: {stats:?}");
  stats.finish();
}

/// The length of the longest short strings.
fn short_string_length() -> usize {
  if full_size() { 4 } else { 2 }
}

#[test]
fn short_strings_equal_reference() {
  let alphabets: &[&[char]] = if full_size() { &[&ALPHABET, &ALPHABET_2] } else { &[&ALPHABET] };
  let mut stats = Stats::golden("short-strings");
  for alphabet in alphabets {
    for frames in scopes() {
      for len in 0..=short_string_length() {
        for_each_string(alphabet, len, |text| stats.check(&frames, text));
      }
    }
  }
  println!("short strings: {stats:?}");
  stats.finish();
  assert!(stats.asts > 1000, "{stats:?}");
}

#[test]
fn short_strings_after_iteration_heads_equal_reference() {
  // the heads that set the lexer's `till_in` flag, followed by every short string
  let max_len = short_string_length();
  let mut stats = Stats::golden("short-strings-after-iteration-heads");
  for frames in &scopes()[..3] {
    for head in ["for ", "some ", "every ", "for x in [1], ", "for item in [1] return "] {
      for len in 0..=max_len {
        for_each_string(&ALPHABET, len, |text| {
          let input = format!("{head}{text}");
          for (name, start) in &START_RULES[..1] {
            stats.check_rule(frames, name, start, &input);
          }
          let input = format!("{head}{text} in [1] return 1");
          for (name, start) in &START_RULES[..1] {
            stats.check_rule(frames, name, start, &input);
          }
        });
      }
    }
  }
  println!("short strings after iteration heads: {stats:?}");
  stats.finish();
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
  let mut stats = Stats::golden("random-texts");
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
  stats.finish();
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
  let mut stats = Stats::golden("edited-and-cut-texts");
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
  stats.finish();
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
            inner @ AstNode::Context(_) => Value::from(convert(inner)),
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

/// The expressions and context literals of the FEEL corpus: the `*.jsonl` files of the
/// directory `RHEON_PARSER_CORPUS` (an absolute path, such as that of `tools/feel-corpus`).
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
  assert!(!corpus.is_empty(), "set RHEON_PARSER_CORPUS to the absolute path of tools/feel-corpus");
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
/// needs no lookahead.
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
  assert_eq!(Vec::<(usize, String)>::new(), late);
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
      let (name, start) = &START_RULES[*rule];
      let mut stats = Stats::default();
      stats.check_rule(&frames, name, start, text);
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
  let mut stats = Stats::golden("reused-chains");
  for frames in scopes() {
    for input in &inputs {
      stats.check(&frames, input);
    }
  }
  println!("reused chains: {stats:?}");
  stats.finish();
}

/// A chain of name tokens is scanned once: the parts scanned grow linearly with the length.
#[test]
fn name_chains_are_scanned_once() {
  let scope = [ctx(&[("a", number(1)), ("b", Value::from(ctx(&[("b", number(2))])))])];
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

/// After an iteration variable `item` the lexer stays in iteration-variable mode, so every later
/// name token looks for a part `in` in the rest of its chain: each part of a chain is compared
/// once, whatever the number of name tokens that start in the chain.
#[test]
fn parts_compared_with_in_grow_linearly_with_the_chain() {
  let scope = [ctx(&[("a", number(1))])];
  let terms = 1_000;
  for (head, chain) in [
    ("for item in [1] return ", vec!["a"; terms].join(" + ")),
    ("some item in [1] satisfies ", vec!["a"; terms].join(" * ")),
    ("for item in [1] return ", vec!["b"; terms].join(" + ")),
    ("for item in [1] return ", format!("{} + for x in a return a + x", vec!["a"; terms].join(" + "))),
    (
      "for item in [1], item in [2] return ",
      format!("{} + (for item in [3] return a + a - a)", vec!["a"; terms].join(" + ")),
    ),
  ] {
    let text = format!("{head}{chain}");
    let work = work_of(&scope, &text);
    let (parts, compared) = (work["parts"], work.get("in parts").copied().unwrap_or(0));
    assert!(compared <= parts, "{compared} parts compared with `in` of {parts} parts: {text:.60}");
  }
}

/// The tokens after an iteration variable `item`, which leaves the lexer in iteration-variable
/// mode, equal the reference's: later names end before a part `in`, also names in scope that have
/// it, and an `in` without white space after it is the keyword.
#[test]
fn names_after_an_item_variable_equal_reference() {
  let frames = [ctx(&[("a", number(1)), ("x in y", number(2)), ("in", number(3)), ("b in", number(4))])];
  let mut stats = Stats::golden("after-an-item-variable");
  for text in [
    "for item in [1] return a",
    "for item in[1] return item",
    "for item in [1] return x in y",
    "for item in [1] return a + x in y + a",
    "for item in [1] return a + a + x in y + x in y",
    "for item in [1] return a + b in + a in a",
    "for item in [1] return a + in + a",
    "for item in [1] return a + (in) + a + x in y",
    "for item in [1] return for x in a return x in y + a",
    "for item in [1] return for item in a return x in y + a + x in y",
    "for item in [1], x in [2] return a + x in y",
    "for item in [1], item in [2] return a + x in y + x in y",
    "some item in [1] satisfies a + x in y = a + x in y",
    "every item in [1] satisfies a in[1]",
    "for item in [1] return a + b + c in d in e",
    "for item in [1] return a + a",
    "for item in a + a + x in y return a",
    "for item",
    "for item in",
    "for item in in",
  ] {
    stats.check(&frames, text);
    stats.check(&[], text);
  }
  println!("after an item variable: {stats:?}");
  stats.finish();
}

/// The input contexts of the tests of lists: `rows` holds contexts of the same type, `mixed`
/// contexts of different types (so its item keys are no names in scope), `one` a single
/// context, `deep` lists in the items of a list.
fn contexts_with_lists() -> [FeelContext; 1] {
  let row = |keys: &[&str]| Value::from(ctx(&keys.iter().map(|key| (*key, number(1))).collect::<Vec<_>>()));
  let list = |items: Vec<Value>| Value::List(items.into());
  let lines = |key: &str| Value::from(ctx(&[("id", number(1)), ("lines", list(vec![row(&[key, "qty"]), row(&[key, "qty"])]))]));
  [ctx(&[
    (
      "rows",
      list(vec![
        row(&["amount", "unit price", "n"]),
        row(&["amount", "unit price", "n"]),
        row(&["amount", "unit price", "n"]),
      ]),
    ),
    ("mixed", list(vec![row(&["net value", "m"]), row(&["gross value", "m"])])),
    ("one", list(vec![row(&["only one"])])),
    ("deep", list(vec![lines("line total"), lines("line total")])),
    ("uneven", list(vec![lines("part total"), lines("other total")])),
    ("total", number(1)),
    ("grand total", number(2)),
  ])]
}

/// The index of an input context compares the items of its lists with their first item only
/// when a name is looked up that the items alone can hold. No list is decided by an expression
/// whose names are keys of the input context itself, of nested contexts or of a list of one
/// item, or names that are not looked up: a name of one word at the end of its chain or before
/// a keyword that ends a name, such as the list of a `for` and the operands of `and`.
#[test]
fn lists_are_decided_when_a_lookup_needs_them() {
  let frames = contexts_with_lists();
  for (text, decided) in [
    ("sum(rows.amount)", false),
    ("sum(for r in rows return r.amount) + total", false),
    ("grand total + total + count(rows)", false),
    ("one[1].only one + grand total", false),
    ("grand total + unknown name + rows", false),
    ("for m in deep return sum(for s in m.lines return s.qty)", false),
    ("for r in rows return if r.n in [1, 2] and r.amount between 1 and 2 then r.n else r.amount", false),
    ("some r in rows satisfies r.n instance of number or r.amount", false),
    ("rows . amount", false),
    ("rows[n > 1].amount + grand total", false),
    ("rows[1].amount * rows[2].n - total / 2", false),
    ("rows[unit price > 1]", true),
    ("grand total + unit price", true),
    ("mixed[net value > 1]", true),
    ("deep[1].lines[line total > 1]", true),
    ("uneven[1].lines[part total > 1]", true),
  ] {
    let work = work_of(&frames, text);
    assert_eq!(decided, work.contains_key("lists decided"), "{text}: {work:?}");
  }
}

/// With lists in the input context, the tokens and trees equal the reference's, which compares
/// the items of every list: the keys of items of one type are names in scope, the keys of
/// items of different types are not.
#[test]
fn names_of_list_items_equal_reference() {
  let frames = contexts_with_lists();
  let mut stats = Stats::golden("names-of-list-items");
  for text in [
    "sum(rows.amount)",
    "rows[unit price > 1]",
    "unit price + 1",
    "unit price x + grand total",
    "net value + 1",
    "net value x",
    "gross value x + net value",
    "mixed[net value > 1 and gross value < 2]",
    "only one + only one x",
    "line total x + qty",
    "part total x + other total x",
    "deep.lines.line total + deep . lines . line total",
    "uneven.lines.part total + uneven . lines . part total",
    "rows . unit price + rows.unit price + mixed . net value + one . only one",
    "grand total + total + unit price + net value + only one + line total + part total",
    "grand total x + unit price",
    "for unit price in rows return unit price x",
    "for m in deep return sum(for s in m.lines return s.qty)",
    "for r in rows return if r.n in [1, 2] and r.amount between 1 and 2 then r.n else r.amount",
    "amount and n or m and qty then id else lines in total between m and m return m satisfies m instance of m",
    "rows[1].amount instance of number and x instance of date and time and y",
    "x instance of n and y instance of amount",
    "rows[1].amount.year and deep.lines.qty.value or one.only one.month",
    "date and time and amount",
    "years and months duration and n",
  ] {
    stats.check(&frames, text);
    // the names in another order of the lookups
    stats.check(&frames, &format!("net value + {text} + unit price"));
  }
  println!("names of list items: {stats:?}");
  stats.finish();
  assert!(stats.asts > 20, "{stats:?}");
}

/// Nested binders push one context each. A name is looked up among them in one step, and their
/// keys are bounded together once there are many, so the contexts walked do not grow with the
/// square of the nesting; the tokens and trees are the reference's.
#[test]
fn pushed_contexts_walked_grow_linearly_with_the_nesting() {
  let nested = |levels: usize, body: &str| {
    let mut text = String::new();
    for level in 0..levels {
      text.push_str(&format!("for v{level} in [1] return "));
    }
    text.push_str(body);
    text
  };
  let levels = if full_size() { 2_000 } else { 400 };
  for body in ["v0 + v1 + a", "v3 x + v5.b + a b", "some q in v1 satisfies q + v2 > a"] {
    let text = nested(levels, body);
    let walked = work_of(&[], &text).get("local contexts").copied().unwrap_or(0);
    assert!(walked <= 40 * levels, "{walked} pushed contexts walked for {levels} levels: {body}");
  }
  let mut stats = Stats::golden("nested-binders");
  let frames = [ctx(&[("a", number(1)), ("a b", number(2)), ("v3 x", number(3))])];
  for levels in [1, 2, 15, 16, 17, 18, 40] {
    for body in [
      "v0 + v1 + a",
      "v3 x + v5.b + a b",
      "v0 v1 + v16 + v17 x",
      "some q in v1 satisfies q + v2 > a",
      "function(v1, p q) v1 + p q + v0",
      "{v1: 1, w: v1 + v0, n: {m: w}}.n.m + v0",
    ] {
      let text = nested(levels, body);
      stats.check(&frames, &text);
      stats.check(&[], &text);
    }
  }
  println!("nested binders: {stats:?}");
  stats.finish();
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
    ("n", Value::from(ctx(&[("p q", number(1)), ("p", number(2)), ("q r s", number(3))]))),
    ("items", Value::List(vec![Value::from(ctx(&[("u v", number(1)), ("u", number(2))]))].into())),
    ("d", Value::from(ctx(&[("a", Value::from(ctx(&[("b", number(1))])))]))),
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
  let mut stats = Stats::golden("first-word-bounds");
  for frames in [vec![top.clone()], vec![nested.clone()], vec![top, nested], vec![]] {
    for input in inputs {
      stats.check(&frames, input);
    }
  }
  println!("first-word bounds: {stats:?}");
  stats.finish();
}

/// The input contexts are indexed only when a name is looked up beyond their own keys.
#[test]
fn input_contexts_are_indexed_on_first_use() {
  let items = Value::List(vec![Value::from(ctx(&[("amount", number(1))])); 3].into());
  let frames = [ctx(&[("a", Value::from(ctx(&[("b", number(1))]))), ("c", number(2)), ("items", items)])];
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
    ("d", Value::from(ctx(&[("year", number(1))]))),
    ("x", number(2)),
    ("date", number(3)),
    ("number", number(4)),
  ])];
  let mut stats = Stats::golden("one-part-names");
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
  stats.finish();
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
  let mut stats = Stats::golden("literals");
  for input in &inputs {
    stats.check(&[], input);
    stats.check(&[], &format!("[{input}, {input}]"));
  }
  println!("literals: {stats:?}");
  stats.finish();
}

/// U+1680 between words: after a symbol it starts a part (empty after trimming when alone), after
/// white space it is white space; so two empty parts never follow each other.
#[test]
fn empty_parts_equal_reference() {
  let mut stats = Stats::golden("empty-parts");
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
  stats.finish();
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
  let mut stats = Stats::golden("unicode-texts");
  for _ in 0..count {
    let text: String = (0..1 + rng.below(12)).map(|_| *rng.pick(&PIECES)).collect();
    stats.check(&frames, &text);
  }
  println!("unicode texts: {stats:?}");
  stats.finish();
}

/// A chosen set of expressions with their whole outcome, readable: for each, the names in scope,
/// the tokens the parser reads and the tree or the error (`src/tests/golden/expressions.txt`).
/// It states what the front end does in words a person can check, where the groups of generated
/// cases keep digests. After a change that is meant, `RHEON_GOLDEN=update` writes it again.
#[test]
fn chosen_expressions_have_their_recorded_outcome() {
  let cases: &[(&[&str], &str)] = &[
    (&[], "1 + 2 * 3"),
    (&[], "-(1 - 2) ** 2 / 4"),
    (&[], "1e3 + 1.5E-2 + .5"),
    (&[], "1e"),
    (&[], "\"a\" + \"b\\n\\\"c\\\"\""),
    (&[], "\"never closed"),
    (&[], "1 /* never closed"),
    (&[], "true and false or null"),
    (&[], "not(true)"),
    (&["a", "b"], "a < b and b >= a or a != b"),
    (&["a"], "a between 1 and 10"),
    (&["a"], "a in [1..10]"),
    (&["a"], "a in (< 5, [10..20), 30)"),
    (&["a"], "a instance of number"),
    (&["a"], "a instance of list<context<n: number, s: string>>"),
    (&["a"], "a instance of function<number, string> -> boolean"),
    (&["a"], "if a then 1 else 2"),
    (&["items"], "for i in items return i * 2"),
    (&["items"], "for i in 1..3, j in items return i + j"),
    (&["items"], "some i in items satisfies i > 2"),
    (&["items"], "every i in items satisfies i.n > 2"),
    (&["items"], "items[n > 1].n"),
    (&["items"], "items[1]"),
    (&["items"], "items[item > 1]"),
    (&[], "[1, [2, 3], []]"),
    (&[], "{a: 1, b: a + 1, \"c d\": {e: b}}.b"),
    (&[], "{a: 1}.a.b.c"),
    (&[], "function(a, b: number) a + b"),
    (&[], "function(a) external {java: {class: \"c\", method signature: \"m\"}}"),
    (&["f"], "f(1, 2)"),
    (&["f"], "f(a: 1, b: 2)"),
    (&[], "date(\"2026-01-31\") + duration(\"P1D\")"),
    (&[], "date and time(\"2026-01-31T10:00:00\")"),
    (&[], "@\"2026-01-31\""),
    (&[], "[1..2]"),
    (&[], "(1..2]"),
    (&[], "]1..2["),
    (&[], "> 5"),
    (&["x"], "> x + 1"),
    (&["x"], "<= 10 = (null..10]"),
    (&["x"], "x.y.z"),
    (&["x"], "x...y"),
    (&["Grace's age"], "Grace's age + 1"),
    (&["unit price", "qty"], "unit price * qty"),
    (&["tax return"], "for t in tax return return t"),
    (&["a-b", "a", "b"], "a-b + a - b"),
    (&["profit/loss"], "profit/loss / 2"),
    (&["fr**n*s"], "fr**n*s ** 2"),
    (&["x and y"], "x and y and z"),
    // names that are not in scope
    (&[], "x"),
    (&[], "unknown name + 1"),
    (&[], "x - 1"),
    (&[], "x.y"),
    (&[], "x and y"),
    (&[], "for s in x return s.z"),
    (&[], "if x then y else z"),
    (&[], "price list"),
    (&[], "Grace's age"),
    (&[], "opt-in rate"),
    (&[], "Is in range"),
    (&[], "date and time"),
    // iteration variables
    (&[], "for item in [1] return item"),
    (&[], "for in+1 in [1] return 1"),
    (&[], "for a b in [1] return a b"),
    // keywords next to brackets
    (&["x"], "x in[1]"),
    (&["x"], "x in [1]"),
    (&["list"], "list < 2"),
    // the words of the types outside a type
    (&["list"], "list<2"),
    (&["range", "context"], "range < context"),
    (&[], "{list: 1, range: 2, context: 3}.list < 2"),
    (&[], "for list in [1, 2] return list<3"),
    (&[], "function(list: list<number>) list<2"),
    (&["a", "list"], "a instance of list<number> and list<2"),
    (&["a", "T", "list"], "a instance of T and list<2"),
    (&["a"], "a instance of range<number>"),
    (&["a"], "a instance of function<list<number>, range<date>> -> context<n: number>"),
    (&[], "list<2"),
    // a built-in property is looked for right after a dot only
    (&["a", "start date"], "a.b + start date"),
    (&["a", "year end"], "a.b.c = year end"),
    (&["a", "start date"], "a.start + start date"),
    (&["a", "time offset x"], "a...b + time offset x"),
    (&["a"], "a.b + start included"),
    (&["a"], "a.b + other words"),
    (&["a", "start"], "a.b + start included"),
    (&["a"], "a.start included + a.b.time offset"),
    // `and` ends the `between` of its own bracket depth
    (&["x"], "x between 1 and 2 and true"),
    (&["x", "a", "b"], "x between (if a and b then 1 else 2) and 5"),
    (&["x", "f", "a", "b"], "x between f(a and b) and [a and b, 5][2]"),
    (&["x", "a", "b"], "x between {c: a and b}.c and 5 and b"),
    (&["x", "y"], "x between y between 1 and 2 and 3"),
    (&["x", "a", "b"], "[x between 1, a and b]"),
    (&["x", "a", "b"], "(x between 1) and b"),
    // errors
    (&[], ""),
    (&[], "1 +"),
    (&[], "(1"),
    (&[], "1 2"),
    (&[], "a b c d"),
    (&[], "$"),
  ];
  let mut text = String::from("# Chosen expressions: the names in scope, the tokens read (each with its offset) and the tree or the error.\n");
  for (names, input) in cases {
    let frames = [ctx(&names.iter().map(|name| (*name, number(1))).collect::<Vec<_>>())];
    let run = run_production(&frames, &TokenType::StartExpression, input);
    text.push_str(&format!(
      "\n## {input:?}\nin scope: {}\n",
      if names.is_empty() { "nothing".to_string() } else { names.join(", ") }
    ));
    for (_, value, offset) in &run.tokens {
      text.push_str(&format!("  {value} @{offset}\n"));
    }
    match &run.outcome {
      // the tree, without the Debug text that follows it
      Outcome::Ast(tree) => text.push_str(tree.rsplit_once('\n').map_or(tree.as_str(), |(tree, _)| tree).trim_end()),
      Outcome::Error(message) => text.push_str(&format!("error: {message}")),
      Outcome::Panic(message) => panic!("{input:?}: the parser panicked: {message}"),
    }
    text.push('\n');
  }
  let path = Golden::file("expressions", false);
  if Golden::updating() {
    std::fs::write(&path, &text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    return;
  }
  let recorded = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}; write it with RHEON_GOLDEN=update", path.display()));
  // the first case that differs
  let differs = recorded.split("\n## ").zip(text.split("\n## ")).find(|(a, b)| a != b);
  assert!(
    recorded == text,
    "the outcome of a chosen expression is not the recorded one; after a change that is meant, write the file again with RHEON_GOLDEN=update\nrecorded and now:\n{differs:#?}"
  );
}
