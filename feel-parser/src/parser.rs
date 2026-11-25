//! The parser of the `FEEL` grammar, written by hand.
//!
//! Recursive descent for the constructs, and one loop with the precedence levels of the
//! grammar for the operators ([Parser::infix]). It reads the tokens of today's [Lexer] and
//! tells it what it expects at each place (a type, a name after a dot, the variable of an
//! iteration). It replaced a parser driven by tables that Bison generated from
//! `feel-grammar/src/feel.y`, and gives the same [AstNode] for every text.
//!
//! A token is read from the lexer when the parser first looks at it, never earlier: the lexer
//! reads a name with the scope and the expectations of that moment.

use crate::AstNode;
use crate::errors::*;
use crate::lexer::{Lexer, TokenValue};
use crate::scope::ParsingScope;
use crate::token::TokenType;
use dsntk_common::Result;
use rheon_feel::{FeelType, IntervalType, Name};

/// The most expressions that are open at once while parsing: an operand inside an operand
/// counts one. A text that is nested deeper is an error.
pub const MAX_NESTING: usize = 640;

/// The precedence levels of the operators, lowest first (grammar rules 2 and 4). The operators
/// of a level are left associative.
const LOWEST: u8 = 0;
const OR: u8 = 1;
const AND: u8 = 2;
/// The comparisons of grammar rule 49: `=`, `!=`, `<`, `<=`, `>`, `>=`, `between` and `in`.
const COMPARISON: u8 = 3;
const ADDITION: u8 = 4;
const MULTIPLICATION: u8 = 5;
const EXPONENTIATION: u8 = 6;
const NEGATION: u8 = 7;

/// The kind of the token the parser looks at.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum T {
  And,
  At,
  Between,
  BetweenAnd,
  Boolean,
  BuiltInTypeName,
  Colon,
  Comma,
  Context,
  Descendant,
  Div,
  Dot,
  Ellipsis,
  Else,
  Eof,
  Eq,
  Every,
  Exp,
  External,
  For,
  Function,
  Ge,
  Gt,
  If,
  In,
  As,
  Instance,
  Le,
  LeftBrace,
  LeftBracket,
  LeftParen,
  List,
  Lt,
  Minus,
  Mul,
  Name,
  NameDateTime,
  Ne,
  Not,
  Null,
  Numeric,
  Of,
  Or,
  Other,
  Plus,
  Range,
  Return,
  RightArrow,
  RightBrace,
  RightBracket,
  RightParen,
  Satisfies,
  Some,
  String,
  Then,
}

/// Returns the kind of a token.
fn kind_of(value: &TokenValue) -> T {
  match value {
    TokenValue::And => T::And,
    TokenValue::At => T::At,
    TokenValue::Between => T::Between,
    TokenValue::BetweenAnd => T::BetweenAnd,
    TokenValue::Boolean(_) => T::Boolean,
    TokenValue::BuiltInTypeName(_) => T::BuiltInTypeName,
    TokenValue::Colon => T::Colon,
    TokenValue::Comma => T::Comma,
    TokenValue::Context => T::Context,
    TokenValue::Descendant => T::Descendant,
    TokenValue::Div => T::Div,
    TokenValue::Dot => T::Dot,
    TokenValue::Ellipsis => T::Ellipsis,
    TokenValue::Else => T::Else,
    TokenValue::YyEof => T::Eof,
    TokenValue::Eq => T::Eq,
    TokenValue::Every => T::Every,
    TokenValue::Exp => T::Exp,
    TokenValue::External => T::External,
    TokenValue::For => T::For,
    TokenValue::Function => T::Function,
    TokenValue::Ge => T::Ge,
    TokenValue::Gt => T::Gt,
    TokenValue::If => T::If,
    TokenValue::In => T::In,
    TokenValue::As => T::As,
    TokenValue::Instance => T::Instance,
    TokenValue::Le => T::Le,
    TokenValue::LeftBrace => T::LeftBrace,
    TokenValue::LeftBracket => T::LeftBracket,
    TokenValue::LeftParen => T::LeftParen,
    TokenValue::List => T::List,
    TokenValue::Lt => T::Lt,
    TokenValue::Minus => T::Minus,
    TokenValue::Mul => T::Mul,
    TokenValue::Name(_) => T::Name,
    TokenValue::NameDateTime(_) => T::NameDateTime,
    TokenValue::Nq => T::Ne,
    TokenValue::Not => T::Not,
    TokenValue::Null => T::Null,
    TokenValue::Numeric(..) => T::Numeric,
    TokenValue::Of => T::Of,
    TokenValue::Or => T::Or,
    TokenValue::Plus => T::Plus,
    TokenValue::Range => T::Range,
    TokenValue::Return => T::Return,
    TokenValue::RightArrow => T::RightArrow,
    TokenValue::RightBrace => T::RightBrace,
    TokenValue::RightBracket => T::RightBracket,
    TokenValue::RightParen => T::RightParen,
    TokenValue::Satisfies => T::Satisfies,
    TokenValue::Some => T::Some,
    TokenValue::String(_) => T::String,
    TokenValue::Then => T::Then,
    _ => T::Other,
  }
}

/// The most errors a tolerant parse records.
pub const MAX_DIAGNOSTICS: usize = 100;

/// What a tolerant parse records ([crate::parse_tolerant]). The places are byte offsets in the
/// input.
#[derive(Debug, Default)]
pub struct Syntax {
  /// Every token of the input in its order, white space and comments included: together they
  /// are the whole input.
  pub tokens: Vec<SyntaxToken>,
  /// The nodes, each before the nodes inside it.
  pub nodes: Vec<SyntaxNode>,
  /// The errors, in the order of their places.
  pub diagnostics: Vec<Diagnostic>,
  /// Whether the text has more errors than the [MAX_DIAGNOSTICS] that are recorded.
  pub truncated: bool,
  /// What the parser expected where the text ends, when it ends before the expression does.
  pub end: Option<TextEnd>,
}

/// What a tolerant parse expected at the end of a text that ends too early: what an editor can
/// offer there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEnd {
  /// The code of the error at the end of the text (see [Diagnostic::code]).
  pub code: &'static str,
  /// For the code `expected`: the missing token as it is written (`then`, `)`).
  pub token: &'static str,
  /// For the code `expected-name`: the part of the expression the name has.
  pub role: TokenClass,
  /// The names in scope there that the text itself declares (iteration variables, parameters,
  /// keys of the contexts around the place), sorted.
  pub names: Vec<String>,
}

/// A token of a tolerant parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntaxToken {
  pub class: TokenClass,
  pub start: usize,
  pub end: usize,
}

/// What a token is in its expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenClass {
  Whitespace,
  Comment,
  Keyword,
  Name,
  Number,
  String,
  Constant,
  Operator,
  Punctuation,
  /// The name of the function of a date and time literal.
  Function,
  /// The name of a type.
  Type,
  /// The key of a context entry.
  Key,
  /// The name of a parameter.
  Parameter,
  /// The variable of an iteration.
  Variable,
  /// The name after a dot.
  Property,
  /// Text that starts no token.
  Error,
}

impl TokenClass {
  /// The name of the class in the answers of the editor operations.
  pub fn as_str(&self) -> &'static str {
    match self {
      TokenClass::Whitespace => "whitespace",
      TokenClass::Comment => "comment",
      TokenClass::Keyword => "keyword",
      TokenClass::Name => "name",
      TokenClass::Number => "number",
      TokenClass::String => "string",
      TokenClass::Constant => "constant",
      TokenClass::Operator => "operator",
      TokenClass::Punctuation => "punctuation",
      TokenClass::Function => "function",
      TokenClass::Type => "type",
      TokenClass::Key => "key",
      TokenClass::Parameter => "parameter",
      TokenClass::Variable => "variable",
      TokenClass::Property => "property",
      TokenClass::Error => "error",
    }
  }
}

/// A node of a tolerant parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntaxNode {
  pub kind: &'static str,
  pub start: usize,
  pub end: usize,
}

/// An error of a tolerant parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
  /// What kind of error it is, for a program.
  pub code: &'static str,
  /// The error in words.
  pub message: String,
  pub start: usize,
  pub end: usize,
}

impl Syntax {
  /// Puts the white space and the comments between the tokens, and the nodes in their order.
  fn finish(&mut self, input: &str) {
    let mut tokens = Vec::with_capacity(self.tokens.len() * 2 + 1);
    let mut position = 0;
    self.tokens.sort_by_key(|token| token.start);
    let end = SyntaxToken {
      class: TokenClass::Error,
      start: input.len(),
      end: input.len(),
    };
    for token in self.tokens.iter().copied().chain(std::iter::once(end)) {
      if token.start < position {
        continue;
      }
      trivia(input, position, token.start, &mut tokens);
      if token.end > token.start {
        tokens.push(token);
      }
      position = token.end.max(token.start);
    }
    self.tokens = tokens;
    // a node stands before the nodes inside it; of two nodes of the same place, the one made
    // later holds the other
    let mut order: Vec<usize> = (0..self.nodes.len()).collect();
    order.sort_by(|&a, &b| {
      let (x, y) = (&self.nodes[a], &self.nodes[b]);
      x.start.cmp(&y.start).then(y.end.cmp(&x.end)).then(b.cmp(&a))
    });
    self.nodes = order.into_iter().map(|index| self.nodes[index]).collect();
    self.diagnostics.sort_by_key(|diagnostic| diagnostic.start);
  }
}

/// Writes the white space and the comments of `input[start..end]` as tokens.
fn trivia(input: &str, start: usize, end: usize, tokens: &mut Vec<SyntaxToken>) {
  let mut position = start;
  while position < end {
    let rest = &input[position..end];
    let (class, len) = if rest.starts_with("//") {
      (TokenClass::Comment, rest.find('\n').unwrap_or(rest.len()))
    } else if let Some(comment) = rest.strip_prefix("/*") {
      (TokenClass::Comment, comment.find("*/").map_or(rest.len(), |len| len + 4))
    } else {
      (TokenClass::Whitespace, rest.find('/').filter(|&len| len > 0).unwrap_or(rest.len()).max(1))
    };
    let len = (position + len..=end).find(|&at| input.is_char_boundary(at)).unwrap_or(end) - position;
    tokens.push(SyntaxToken {
      class,
      start: position,
      end: position + len,
    });
    position += len;
  }
}

/// Returns the class of a token of the kind `kind`.
fn class_of(kind: T) -> TokenClass {
  match kind {
    T::Name => TokenClass::Name,
    T::NameDateTime => TokenClass::Function,
    T::BuiltInTypeName => TokenClass::Type,
    T::Numeric => TokenClass::Number,
    T::String => TokenClass::String,
    T::Boolean | T::Null => TokenClass::Constant,
    T::And
    | T::Between
    | T::BetweenAnd
    | T::Context
    | T::Else
    | T::Every
    | T::External
    | T::For
    | T::Function
    | T::If
    | T::In
    | T::Instance
    | T::As
    | T::List
    | T::Not
    | T::Of
    | T::Or
    | T::Range
    | T::Return
    | T::Satisfies
    | T::Some
    | T::Then => TokenClass::Keyword,
    T::Div | T::Eq | T::Exp | T::Ge | T::Gt | T::Le | T::Lt | T::Minus | T::Mul | T::Ne | T::Plus | T::Dot | T::Descendant | T::Ellipsis | T::RightArrow | T::At => {
      TokenClass::Operator
    }
    T::Colon | T::Comma | T::LeftBrace | T::LeftBracket | T::LeftParen | T::RightBrace | T::RightBracket | T::RightParen => TokenClass::Punctuation,
    T::Eof | T::Other => TokenClass::Error,
  }
}

/// Returns `true` for white space of an expression.
fn is_space(ch: char) -> bool {
  ch.is_whitespace() || matches!(ch, '\u{180E}' | '\u{200B}' | '\u{FEFF}')
}

/// Returns `true` when a token of the kind `kind` starts an operand.
fn starts_operand(kind: T) -> bool {
  matches!(
    kind,
    T::At
      | T::Every
      | T::For
      | T::LeftBrace
      | T::Null
      | T::Function
      | T::If
      | T::Some
      | T::Numeric
      | T::String
      | T::Boolean
      | T::Eq
      | T::Ne
      | T::Lt
      | T::Le
      | T::Gt
      | T::Ge
      | T::Minus
      | T::Name
      | T::NameDateTime
      | T::LeftParen
      | T::LeftBracket
      | T::RightBracket
  )
}

/// Returns what is expected where a token of the kind `kind` is missing: the token and what it
/// is for, to stand after "Expected" in the message of the error.
fn expected(kind: T) -> &'static str {
  match kind {
    T::RightParen => "`)` to close the `(`",
    T::RightBracket => "`]` to close the `[`",
    T::RightBrace => "`}` to close the context",
    T::LeftParen => "`(`",
    T::Colon => "`:` after the name",
    T::Ellipsis => "`..` between the endpoints of the interval",
    T::Then => "`then` after the condition of `if`",
    T::Else => "`else` to complete the `if` expression",
    T::In => "`in` after the variable name",
    T::Return => "`return` to complete the `for` expression",
    T::Satisfies => "`satisfies` to complete the `some` or `every` expression",
    T::Of => "`of` after `instance`",
    T::BetweenAnd => "`and` after the lower bound of `between`",
    T::Lt => "`<` to open the type",
    T::Gt => "`>` to close the type",
    T::RightArrow => "`->` before the result type of the function type",
    _ => "another token",
  }
}

/// Returns a token of the kind `kind` as it is written, for the tokens that are written one way.
fn written(kind: T) -> &'static str {
  match kind {
    T::RightParen => ")",
    T::RightBracket => "]",
    T::RightBrace => "}",
    T::LeftParen => "(",
    T::Colon => ":",
    T::Ellipsis => "..",
    T::Then => "then",
    T::Else => "else",
    T::In => "in",
    T::Return => "return",
    T::Satisfies => "satisfies",
    T::Of => "of",
    T::BetweenAnd => "and",
    T::Lt => "<",
    T::Gt => ">",
    T::RightArrow => "->",
    _ => "",
  }
}

/// Returns the name of the node of a binary operator of the level `level`.
fn operator_name(level: u8) -> &'static str {
  match level {
    OR => "Disjunction",
    AND => "Conjunction",
    COMPARISON => "Comparison",
    ADDITION => "Addition",
    MULTIPLICATION => "Multiplication",
    _ => "Exponentiation",
  }
}

/// How an expression was made, for the start rules that take only some expressions.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Made {
  /// A list, a function definition or a context, alone.
  Boxed,
  /// A number, a string, a boolean, an `@` literal or a date and time literal, alone.
  SimpleLiteral,
  /// An addition, subtraction, multiplication, division or exponentiation.
  Arithmetic,
  /// An arithmetic negation.
  Negation,
  /// Any other textual expression.
  Textual,
}

/// The start rules of the grammar that a text can be parsed as ([crate::parse_start]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartRule {
  /// An `expression` (grammar rule 1).
  Expression,
  /// `unary tests` (grammar rule 15).
  UnaryTests,
  /// A `simple expression` of S-FEEL (DMN 1.7 Beta 1, clause 9.2, rule 3): arithmetic and
  /// comparisons of names, qualified names and simple literals, as the decision tables and
  /// invocations of conformance level 2 have them (clause 9.5). It means what it means in FEEL.
  SimpleExpression,
  /// `simple expressions` of S-FEEL (clause 9.2, rule 4): simple expressions separated by `,`, the
  /// list of their values.
  SimpleExpressions,
  /// `textual expressions` (grammar rule 3): textual expressions separated by `,`, the list of
  /// their values.
  TextualExpressions,
}

impl StartRule {
  /// The start token of the lexer for the rule.
  pub(crate) fn token(self) -> TokenType {
    if self == StartRule::UnaryTests {
      TokenType::StartUnaryTests
    } else {
      TokenType::StartExpression
    }
  }
}

/// What a simple expression is expected as, in the message of a construct S-FEEL has not.
const SIMPLE: &str = "a simple expression (S-FEEL, DMN 9.2)";

/// Whether the invocation of `callee` with `parameters` is a date time literal of S-FEEL
/// (clause 9.2, rule 34): `date`, `time`, `duration`, `days and time duration` or `years and
/// months duration` of one string literal.
fn simple_temporal(callee: &AstNode, parameters: &AstNode) -> bool {
  matches!(callee, AstNode::Name(name) if matches!(name.as_str(), "date" | "time" | "duration" | "days and time duration" | "years and months duration"))
    && matches!(parameters, AstNode::PositionalParameters(items) if matches!(items.as_slice(), [AstNode::String(_)]))
}

/// A piece of the text in a message: in backticks, cut when it is long.
fn quoted(text: &str) -> String {
  match text.char_indices().nth(24) {
    Some((cut, _)) => format!("`{}…`", &text[..cut]),
    None => format!("`{text}`"),
  }
}

/// Where the parser is when it reads a token.
#[derive(Clone, Copy, PartialEq, Eq)]
enum At {
  /// Where an operand or a certain token is expected.
  Start,
  /// After a complete operand: an operator or the end of the construct follows.
  AfterOperand,
}

/// The abstract type that a type name names in DMN 1.7 (Figure 10-26): `comparable`, `temporal`,
/// `duration`, and `context`, the context type without entries (`context<>`).
fn abstract_type(name: &Name) -> Option<FeelType> {
  Some(match name.as_str() {
    "comparable" => FeelType::Comparable,
    "temporal" => FeelType::Temporal,
    "duration" => FeelType::Duration,
    "context" => FeelType::Context(Default::default()),
    _ => return None,
  })
}

/// The hand-written parser.
///
/// The functions that call each other for a nested expression keep their stack frames small:
/// a node is built in place (`&mut AstNode`), and every construct has its own function.
pub struct Parser<'parser, 'scope> {
  /// Parsing scope.
  scope: &'parser ParsingScope<'scope>,
  /// Parsed input.
  input: &'parser str,
  /// The lexer.
  lexer: Lexer<'parser, 'scope>,
  /// The start rule.
  start: Option<TokenType>,
  /// The token the parser looks at, read from the lexer and not taken yet.
  token: Option<TokenValue>,
  /// The kind of `token`.
  kind: T,
  /// How the expression parsed last was made.
  made: Made,
  /// The text is a list of expressions separated by `,` ([StartRule::SimpleExpressions],
  /// [StartRule::TextualExpressions]).
  expressions: bool,
  /// The text is S-FEEL: a construct that S-FEEL has not is an error ([StartRule::SimpleExpression]).
  simple: bool,
  /// The expression being parsed ends the middle operand of a `between`: an `and` after a
  /// complete operand is the `and` of that `between`.
  between: bool,
  /// The number of expressions that are open.
  nesting: usize,
  /// Where the token the parser looks at starts and ends in the input.
  token_start: usize,
  token_end: usize,
  /// Where the token taken last starts and ends.
  last_start: usize,
  last_end: usize,
  /// What a tolerant parse records; a strict parse has none and stops at the first error.
  syntax: Option<Box<Syntax>>,
  /// The token and the part of a name that the parser asks for now (for [TextEnd]).
  expected_token: T,
  expected_role: TokenClass,
  /// The token log: every token read from the lexer, with its start offset in the input.
  #[cfg(test)]
  token_log: Option<Vec<(TokenType, TokenValue, usize)>>,
}

#[cfg(test)]
thread_local! {
  /// The lowest address of the stack seen while parsing (a measurement of the tests).
  pub(crate) static STACK_LOW: std::cell::Cell<usize> = const { std::cell::Cell::new(usize::MAX) };
}

/// Replaces the node `lhs` with a node made of it.
fn wrap(lhs: &mut AstNode, node: impl FnOnce(Box<AstNode>) -> AstNode) {
  let inner = Box::new(std::mem::replace(lhs, AstNode::Null));
  *lhs = node(inner);
}

impl<'parser, 'scope> Parser<'parser, 'scope> {
  /// Creates a new parser. The parser has no trace: `_trace` is kept for the callers of the
  /// table-driven parser that it replaced.
  pub fn new(scope: &'parser ParsingScope<'scope>, start_token_type: TokenType, input: &'parser str) -> Self {
    Self {
      scope,
      input,
      lexer: Lexer::new(scope, start_token_type.clone(), input),
      start: Some(start_token_type),
      token: None,
      kind: T::Other,
      made: Made::Textual,
      expressions: false,
      simple: false,
      between: false,
      nesting: 0,
      token_start: 0,
      token_end: 0,
      last_start: 0,
      last_end: 0,
      syntax: None,
      expected_token: T::Other,
      expected_role: TokenClass::Name,
      #[cfg(test)]
      token_log: None,
    }
  }

  /// Parses the text as `rule` asks; the start token of the parser is [StartRule::token].
  pub fn with_rule(mut self, rule: StartRule) -> Self {
    self.expressions = matches!(rule, StartRule::SimpleExpressions | StartRule::TextualExpressions);
    self.simple = matches!(rule, StartRule::SimpleExpression | StartRule::SimpleExpressions);
    self
  }

  /// Reads the text as a name that is being declared (see [crate::parse_name]).
  pub fn with_whole_names(mut self) -> Self {
    self.lexer.set_whole_names();
    self
  }

  /// Reads every name of the text as a member of a value, as in the predicate of a filter: the
  /// keys below the keys of the contexts in scope are names too. For the tests of the index of
  /// those keys.
  #[cfg(test)]
  pub fn with_member_names(mut self) -> Self {
    self.lexer.enter_filter();
    self
  }

  /// Records every token read from the lexer (see [Parser::token_log]).
  #[cfg(test)]
  pub fn with_token_log(mut self) -> Self {
    self.token_log = Some(vec![]);
    self
  }

  /// Returns the recorded tokens.
  #[cfg(test)]
  pub fn token_log(&mut self) -> Vec<(TokenType, TokenValue, usize)> {
    self.token_log.take().unwrap_or_default()
  }

  /// Parses the input.
  pub fn parse(&mut self) -> Result<AstNode> {
    let node = self.start_rule()?;
    // the end of the input
    self.look(At::AfterOperand)?;
    if self.kind == T::Eof { Ok(node) } else { self.error() }
  }

  /// Parses the input and never fails: returns the tokens, the nodes with their places and
  /// every error. After an error the parser goes on as if the missing part were there, and
  /// passes a token that fits nowhere.
  pub fn parse_tolerant(mut self) -> Syntax {
    self.syntax = Some(Box::default());
    let mut parsed = self.start_rule().map(|_| ());
    while parsed.is_ok() {
      parsed = self.rest();
      if parsed.is_ok() && self.kind == T::Eof {
        break;
      }
    }
    if let Err(reason) = parsed {
      // the parser gave up: the rest of the input is read for its tokens only
      let (start, end) = (self.last_end, self.input.len());
      self.diagnostic("failed", reason.to_string().trim_start_matches("<ParserError> ").to_string(), start, end);
      self.token = None;
      while !matches!(self.look(At::Start), Ok(T::Eof) | Err(_)) {
        self.bump();
      }
    }
    let mut syntax = *self.syntax.take().unwrap_or_default();
    syntax.finish(self.input);
    syntax
  }

  /// What follows the expression in a tolerant parse: nothing, or tokens that fit nowhere. One
  /// such token is passed, and an expression after it is parsed.
  fn rest(&mut self) -> Result<()> {
    if self.look(At::AfterOperand)? == T::Eof {
      return Ok(());
    }
    self.unexpected("unexpected", "")?;
    let start = self.token_start;
    self.bump();
    self.note("Error", start);
    if starts_operand(self.look(At::Start)?) {
      self.expression(LOWEST)?;
    }
    Ok(())
  }

  /// Parses what the start rule asks for.
  fn start_rule(&mut self) -> Result<AstNode> {
    // the start token of the lexer
    let (_token_type, _token_value) = self.lexer.next_token()?;
    #[cfg(test)]
    if let Some(log) = &mut self.token_log {
      log.push((_token_type, _token_value, self.lexer.token_start()));
    }
    let start = self.start.take();
    if self.expressions {
      return self.start_expressions();
    }
    let node = match start {
      Some(TokenType::StartExpression) => self.expression(LOWEST)?,
      Some(TokenType::StartBoxedExpression) => self.start_boxed_expression()?,
      Some(TokenType::StartTextualExpression) => self.textual_expression()?,
      Some(TokenType::StartUnaryTests) => self.start_unary_tests()?,
      Some(TokenType::StartRangeLiteral) => self.start_range_literal()?,
      _ => return self.error(),
    };
    Ok(node)
  }

  //----------------------------------------------------------------------------------------------
  // Tokens
  //----------------------------------------------------------------------------------------------

  /// Returns the syntax error.
  #[cold]
  fn error<V>(&self) -> Result<V> {
    Err(err_syntax_error(self.input))
  }

  /// Looks at the next token: reads it from the lexer unless that was done.
  fn look(&mut self, at: At) -> Result<T> {
    if self.token.is_none() {
      self.read(at)?;
    }
    Ok(self.kind)
  }

  /// Reads the next token from the lexer.
  #[inline(never)]
  fn read(&mut self, at: At) -> Result<()> {
    self.lexer.set_after_operand(at == At::AfterOperand);
    #[allow(unused_mut)]
    let (mut token_type, mut token_value) = if self.syntax.is_some() { self.next_token_tolerant()? } else { self.lexer.next_token()? };
    // the places of the token: only a tolerant parse reads them
    if self.syntax.is_some() {
      self.token_start = self.lexer.token_start();
      // the lexer passes the white space after a name of one word
      self.token_end = self.token_start + self.input[self.token_start..self.lexer.position()].trim_end_matches(is_space).len();
    }
    // the `and` after the middle operand of a `between`
    if at == At::AfterOperand && self.between && matches!(token_value, TokenValue::And) {
      token_type = TokenType::BetweenAnd;
      token_value = TokenValue::BetweenAnd;
    }
    self.kind = kind_of(&token_value);
    #[cfg(test)]
    if let Some(log) = &mut self.token_log {
      log.push((token_type.clone(), token_value.clone(), self.lexer.token_start()));
    }
    let _ = token_type;
    if let Some(syntax) = &mut self.syntax
      && self.kind != T::Eof
    {
      syntax.tokens.push(SyntaxToken {
        class: class_of(self.kind),
        start: self.token_start,
        end: self.token_end,
      });
    }
    self.token = Some(token_value);
    Ok(())
  }

  /// Reads the next token from the lexer in a tolerant parse: a character that starts no token
  /// is passed, and a string or a comment that is not closed or has an escape that is none is
  /// taken up to the end of its line or of the input. Each is an error of the record.
  fn next_token_tolerant(&mut self) -> Result<(TokenType, TokenValue)> {
    loop {
      let read = self.lexer.next_token();
      let start = self.lexer.token_start();
      let rest = &self.input[start..];
      match read {
        Ok((_, TokenValue::YyUndef)) | Err(_) if rest.starts_with('"') => {
          // up to the closing quote, the end of the line or the end of the input
          let mut end = start + 1;
          let mut escaped = false;
          let mut closed = false;
          for (offset, ch) in rest.char_indices().skip(1) {
            end = start + offset;
            if matches!(ch, '\u{000A}'..='\u{000D}') {
              break;
            }
            end += ch.len_utf8();
            if ch == '"' && !escaped {
              closed = true;
              break;
            }
            escaped = ch == '\\' && !escaped;
          }
          let (code, message) = if closed {
            ("string-escape", "Invalid escape sequence in this string.")
          } else {
            ("string-open", "Unterminated string: the closing `\"` is missing.")
          };
          self.diagnostic(code, message.to_string(), start, end);
          self.lexer.set_position(end);
          let text = &self.input[start + 1..if closed { end - 1 } else { end }];
          return Ok((TokenType::String, TokenValue::String(text.to_string())));
        }
        Ok((_, TokenValue::YyUndef)) => {
          let (code, message, len) = if rest.starts_with("/*") {
            ("comment-open", "Unterminated comment: the closing `*/` is missing.".to_string(), rest.len())
          } else {
            let character = rest.chars().next();
            let message = character.map_or_else(String::new, |ch| format!("Unexpected character `{ch}`: it is not part of the FEEL syntax."));
            ("character", message, character.map_or(0, char::len_utf8))
          };
          if len == 0 {
            return Ok((TokenType::YyEof, TokenValue::YyEof));
          }
          self.diagnostic(code, message, start, start + len);
          if let Some(syntax) = &mut self.syntax {
            let class = if code == "comment-open" { TokenClass::Comment } else { TokenClass::Error };
            syntax.tokens.push(SyntaxToken { class, start, end: start + len });
          }
          self.lexer.set_position(start + len);
        }
        other => return other,
      }
    }
  }

  /// Adds an error to the record of a tolerant parse; one error per place.
  fn diagnostic(&mut self, code: &'static str, message: String, start: usize, end: usize) {
    if let Some(syntax) = &mut self.syntax
      && syntax.diagnostics.last().is_none_or(|last| last.start != start)
    {
      if syntax.diagnostics.len() < MAX_DIAGNOSTICS {
        syntax.diagnostics.push(Diagnostic { code, message, start, end });
      } else {
        syntax.truncated = true;
      }
    }
  }

  /// The token the parser looks at is not what the grammar takes here: the syntax error of a
  /// strict parse; in a tolerant parse an error of the record, and the parser goes on.
  ///
  /// The message says what was expected and what stands there instead: `what` is the expected
  /// part, as it reads after "Expected".
  fn unexpected(&mut self, code: &'static str, what: &str) -> Result<()> {
    if self.syntax.is_none() {
      return self.error();
    }
    if self.kind == T::Eof
      && let Some(syntax) = &mut self.syntax
      && syntax.end.is_none()
    {
      // the first thing that is missing where the text ends
      syntax.end = Some(TextEnd {
        code,
        token: if code == "expected" { written(self.expected_token) } else { "" },
        role: self.expected_role,
        names: self.scope.local_names(),
      });
    }
    let (start, end) = if self.kind == T::Eof {
      (self.input.len(), self.input.len())
    } else {
      (self.token_start, self.token_end)
    };
    let found = quoted(&self.input[start..end]);
    // an expression is missing after the token before it
    let after = if code == "expected-expression" && self.last_end > self.last_start {
      format!(" after {}", quoted(&self.input[self.last_start..self.last_end]))
    } else {
      String::new()
    };
    let message = match code {
      "unexpected" => format!("Unexpected {found}: an operator or the end of the expression is expected here."),
      _ if self.input.chars().all(is_space) => "The expression is empty.".to_string(),
      _ if self.kind == T::Eof => format!("Expected {what}{after}, but the expression ends here."),
      _ => format!("Expected {what}{after}, found {found}."),
    };
    self.diagnostic(code, message, start, end);
    Ok(())
  }

  /// The token the parser looks at is neither a `,` nor the `closing` bracket of a list: the
  /// syntax error of a strict parse. A tolerant parse records the error and passes the tokens up
  /// to the next `,` or closing bracket that belongs to the list; returns `true` when it is a `,`.
  fn skip_to(&mut self, closing: T, what: &str) -> Result<bool> {
    self.unexpected("expected", what)?;
    let start = self.token_start;
    let mut open = 0usize;
    loop {
      match self.kind {
        T::Eof => break,
        T::LeftParen | T::LeftBracket | T::LeftBrace => open += 1,
        T::RightParen | T::RightBracket | T::RightBrace if open == 0 => break,
        T::RightParen | T::RightBracket | T::RightBrace => open -= 1,
        T::Comma if open == 0 => break,
        _ => {}
      }
      self.bump();
      self.look(At::Start)?;
    }
    if self.last_end > start {
      self.note("Error", start);
    }
    let _ = closing;
    Ok(self.kind == T::Comma)
  }

  /// Records a node of a tolerant parse: it starts at `start` and ends with the token taken last.
  fn note(&mut self, kind: &'static str, start: usize) {
    if let Some(syntax) = &mut self.syntax {
      let end = self.last_end.max(start);
      syntax.nodes.push(SyntaxNode { kind, start, end });
    }
  }

  /// Gives the token the parser looks at its part in the expression (a tolerant parse).
  fn role(&mut self, class: TokenClass) {
    if let Some(syntax) = &mut self.syntax
      && self.token.is_some()
      && let Some(token) = syntax.tokens.last_mut()
    {
      token.class = class;
    }
  }

  /// Takes the token the parser looks at.
  fn take(&mut self) -> TokenValue {
    self.last_start = self.token_start;
    self.last_end = self.token_end;
    self.token.take().unwrap_or(TokenValue::YyEmpty)
  }

  /// Passes the token the parser looks at.
  fn bump(&mut self) {
    self.last_start = self.token_start;
    self.last_end = self.token_end;
    self.token = None;
  }

  /// Passes the next token, which must be of the kind `kind`.
  fn expect(&mut self, kind: T, at: At) -> Result<()> {
    if self.look(at)? == kind {
      self.bump();
      Ok(())
    } else {
      self.expected_token = kind;
      self.unexpected("expected", expected(kind))
    }
  }

  /// Takes the next token, which must be a name.
  #[inline(never)]
  fn name(&mut self, at: At) -> Result<Name> {
    if self.look(at)? == T::Name
      && let TokenValue::Name(name) = self.take()
    {
      Ok(name)
    } else {
      let what = match self.expected_role {
        TokenClass::Property => "a name after `.`",
        TokenClass::Variable => "a variable name",
        TokenClass::Parameter => "a parameter name",
        TokenClass::Type => "the name of a type",
        _ => "a name",
      };
      self.unexpected("expected-name", what)?;
      Ok(Name::from(""))
    }
  }

  /// Takes the next token, which must be a name, in the part `class` of the expression.
  fn name_as(&mut self, at: At, class: TokenClass) -> Result<Name> {
    if self.look(at)? == T::Name {
      self.role(class);
    }
    self.expected_role = class;
    let name = self.name(at);
    self.expected_role = TokenClass::Name;
    name
  }

  //----------------------------------------------------------------------------------------------
  // Start rules
  //----------------------------------------------------------------------------------------------

  /// A boxed expression alone: a list, a function definition or a context.
  fn start_boxed_expression(&mut self) -> Result<AstNode> {
    self.enter()?;
    let node = match self.look(At::Start)? {
      T::LeftBracket => self.list_or_interval(false)?,
      T::Function => self.function_definition()?,
      T::LeftBrace => self.context()?,
      _ => return self.error(),
    };
    self.nesting -= 1;
    Ok(node)
  }

  /// A textual expression: any expression but a boxed expression alone.
  fn textual_expression(&mut self) -> Result<AstNode> {
    let node = self.expression(LOWEST)?;
    if self.made == Made::Boxed { self.error() } else { Ok(node) }
  }

  /// Expressions separated by `,`: the list of their values. Of textual expressions (grammar
  /// rule 3) an item is no boxed expression alone; of simple expressions (S-FEEL rule 4) an item
  /// is a simple expression.
  fn start_expressions(&mut self) -> Result<AstNode> {
    let mut items = vec![];
    loop {
      self.look(At::Start)?;
      let start = self.token_start;
      self.push(&mut items)?;
      if self.made == Made::Boxed && !self.simple {
        self.refuse(
          "a textual expression",
          "a list, a context or a function definition alone",
          start,
          ": write it in parentheses",
        )?;
      }
      if self.kind != T::Comma {
        break;
      }
      self.bump();
    }
    Ok(AstNode::List(items))
  }

  /// The construct from `start` to the token taken last is not what the start rule takes: the
  /// syntax error of a strict parse; in a tolerant parse an error of the record, and the parser
  /// goes on. One error is recorded for a construct: of the constructs in it, none.
  #[cold]
  #[inline(never)]
  fn refuse(&mut self, expected: &str, found: &str, start: usize, hint: &str) -> Result<()> {
    if self.syntax.is_none() {
      return self.error();
    }
    let end = self.last_end.max(start);
    let message = format!("Expected {expected}, found {found} {}{hint}.", quoted(&self.input[start..end]));
    if let Some(syntax) = &mut self.syntax {
      syntax
        .diagnostics
        .retain(|inner| !(inner.code == "not-allowed" && inner.start >= start && inner.end <= end));
    }
    self.diagnostic("not-allowed", message, start, end);
    Ok(())
  }

  /// The operand from `start`, `node`, that the token `kind` starts, in a simple expression: an
  /// error when S-FEEL has no such operand (clause 9.2).
  #[cold]
  #[inline(never)]
  fn simple_operand(&mut self, kind: T, start: usize, node: &AstNode) -> Result<()> {
    let found = match (kind, node) {
      (T::Numeric, AstNode::Numeric(_, _, _, exponent)) if !exponent.is_empty() => "a number with an exponent",
      (T::Null, _) => "the null literal",
      (T::At, _) => "an `@` literal",
      (T::LeftParen | T::LeftBracket | T::RightBracket, AstNode::Range(..)) => "an interval",
      (T::LeftParen, _) => "parentheses",
      (T::LeftBracket, _) => "a list",
      (T::LeftBrace, _) => "a context",
      (T::Function, _) => "a function definition",
      (T::If, _) => "an if expression",
      (T::For, _) => "a for expression",
      (T::Some | T::Every, _) => "a quantified expression",
      (T::Lt | T::Le | T::Gt | T::Ge | T::Eq | T::Ne, _) => "a unary test",
      (T::NameDateTime, AstNode::FunctionInvocation(callee, parameters)) if !simple_temporal(callee, parameters) => match callee.as_ref() {
        AstNode::Name(name) if name.as_str() == "date and time" => "a date and time",
        _ => "a temporal literal of other than one string literal",
      },
      _ => return Ok(()),
    };
    self.refuse(SIMPLE, found, start, "")
  }

  /// The segments of a qualified name after its first name, which a dot follows.
  fn qualified_name(&mut self, first: Name) -> Result<AstNode> {
    let mut segments = vec![AstNode::QualifiedNameSegment(first)];
    while self.look(At::AfterOperand)? == T::Dot {
      self.bump();
      self.lexer.set_after_dot();
      segments.push(AstNode::QualifiedNameSegment(self.name(At::Start)?));
    }
    Ok(AstNode::QualifiedName(segments))
  }

  /// Unary tests: `-`, `not(` tests `)` or tests.
  fn start_unary_tests(&mut self) -> Result<AstNode> {
    self.lexer.set_unary_tests();
    let mut items = vec![];
    match self.look(At::Start)? {
      T::Not => {
        self.bump();
        self.expect(T::LeftParen, At::Start)?;
        loop {
          self.push(&mut items)?;
          match self.kind {
            T::Comma => self.bump(),
            T::RightParen => break self.bump(),
            _ => {
              self.unexpected("expected", "`,` or `)` in the list of tests")?;
              break;
            }
          }
        }
        return Ok(AstNode::NegatedList(items));
      }
      T::Minus => {
        let start = self.token_start;
        self.bump();
        if self.look(At::AfterOperand)? == T::Eof {
          return Ok(AstNode::Irrelevant);
        }
        self.enter()?;
        let mut node = AstNode::Neg(Box::new(self.expression(NEGATION)?));
        self.made = Made::Negation;
        self.note("Negation", start);
        self.infix(&mut node, start, LOWEST, false)?;
        self.nesting -= 1;
        items.push(node);
      }
      _ => items.push(self.expression(LOWEST)?),
    }
    while self.kind == T::Comma {
      self.bump();
      self.push(&mut items)?;
    }
    Ok(AstNode::ExpressionList(items))
  }

  /// A range literal.
  fn start_range_literal(&mut self) -> Result<AstNode> {
    let closed = match self.look(At::Start)? {
      T::LeftParen | T::RightBracket => false,
      T::LeftBracket => true,
      _ => return self.error(),
    };
    self.bump();
    let (start, has_start) = if self.look(At::Start)? == T::Ellipsis {
      if closed {
        return self.error();
      }
      (AstNode::IntervalStart(AstNode::Null.into(), IntervalType::OpenedUndef), false)
    } else {
      let endpoint = self.range_literal_endpoint()?;
      (
        AstNode::IntervalStart(Box::new(endpoint), if closed { IntervalType::Closed } else { IntervalType::Opened }),
        true,
      )
    };
    self.expect(T::Ellipsis, At::Start)?;
    let end = match self.look(At::Start)? {
      T::RightParen | T::LeftBracket if has_start => {
        self.bump();
        AstNode::IntervalEnd(AstNode::Null.into(), IntervalType::OpenedUndef)
      }
      _ => {
        let endpoint = self.range_literal_endpoint()?;
        let interval_type = match self.look(At::Start)? {
          T::RightParen | T::LeftBracket => IntervalType::Opened,
          T::RightBracket => IntervalType::Closed,
          _ => return self.error(),
        };
        self.bump();
        AstNode::IntervalEnd(Box::new(endpoint), interval_type)
      }
    };
    Ok(AstNode::Range(Box::new(start), Box::new(end)))
  }

  /// An endpoint of a range literal: a number with an optional sign, a string or a date and
  /// time literal.
  fn range_literal_endpoint(&mut self) -> Result<AstNode> {
    match self.look(At::Start)? {
      T::Numeric => self.numeric(),
      T::Minus => {
        self.bump();
        if self.look(At::Start)? != T::Numeric {
          return self.error();
        }
        Ok(AstNode::Neg(Box::new(self.numeric()?)))
      }
      T::String => self.string().map(AstNode::String),
      T::At => {
        self.bump();
        if self.look(At::Start)? != T::String {
          return self.error();
        }
        self.string().map(AstNode::At)
      }
      T::NameDateTime => {
        let TokenValue::NameDateTime(name) = self.take() else { return self.error() };
        self.expect(T::LeftParen, At::Start)?;
        let parameters = match self.look(At::Start)? {
          T::Name => {
            let parameter_name = self.name(At::Start)?;
            self.expect(T::Colon, At::Start)?;
            if self.look(At::Start)? != T::String {
              return self.error();
            }
            let value = self.string().map(AstNode::String)?;
            AstNode::NamedParameters(vec![AstNode::NamedParameter(Box::new(AstNode::ParameterName(parameter_name)), Box::new(value))])
          }
          T::String => AstNode::PositionalParameters(vec![self.string().map(AstNode::String)?]),
          _ => return self.error(),
        };
        self.expect(T::RightParen, At::Start)?;
        Ok(AstNode::FunctionInvocation(Box::new(AstNode::Name(name)), Box::new(parameters)))
      }
      _ => self.error(),
    }
  }

  //----------------------------------------------------------------------------------------------
  // Expressions
  //----------------------------------------------------------------------------------------------

  /// Counts an open expression.
  fn enter(&mut self) -> Result<()> {
    self.nesting += 1;
    #[cfg(test)]
    {
      let marker = 0u8;
      let address = &marker as *const u8 as usize;
      STACK_LOW.with(|low| low.set(low.get().min(address)));
    }
    if self.nesting > MAX_NESTING { Err(err_nested_too_deeply(MAX_NESTING)) } else { Ok(()) }
  }

  /// An expression whose operators bind tighter than the level `level`.
  fn expression(&mut self, level: u8) -> Result<AstNode> {
    self.expression_until(level, false)
  }

  /// An expression (see [Parser::expression]). With `end_endpoint` it is an endpoint of an
  /// interval that a `[` after it ends.
  fn expression_until(&mut self, level: u8, end_endpoint: bool) -> Result<AstNode> {
    self.enter()?;
    self.look(At::Start)?;
    let start = self.token_start;
    let mut node = self.operand()?;
    self.infix(&mut node, start, level, end_endpoint)?;
    self.nesting -= 1;
    Ok(node)
  }

  /// An expression in a box (see [Parser::expression]): the caller holds a pointer, not the node,
  /// while the expressions inside it are parsed.
  #[inline(never)]
  fn boxed(&mut self, level: u8) -> Result<Box<AstNode>> {
    Ok(Box::new(self.expression_until(level, false)?))
  }

  /// An expression in a box that is no part of the expressions around it (see [Parser::inner]).
  #[inline(never)]
  fn boxed_inner(&mut self) -> Result<Box<AstNode>> {
    let between = std::mem::replace(&mut self.between, false);
    let node = self.boxed(LOWEST);
    self.between = between;
    node
  }

  /// An expression as the next item of `items`.
  #[inline(never)]
  fn push(&mut self, items: &mut Vec<AstNode>) -> Result<()> {
    items.push(self.expression_until(LOWEST, false)?);
    Ok(())
  }

  /// An expression that is no part of the expressions around it: it is not the end of the
  /// middle operand of a `between`.
  fn inner(&mut self) -> Result<AstNode> {
    let between = std::mem::replace(&mut self.between, false);
    let node = self.expression(LOWEST);
    self.between = between;
    node
  }

  /// The operators after the operand `lhs` that bind tighter than the level `level`, with their
  /// operands: `lhs` becomes the whole expression. With `end_endpoint` the expression is an
  /// endpoint of an interval that a `[` after it ends.
  #[inline(never)]
  fn infix(&mut self, lhs: &mut AstNode, start: usize, level: u8, end_endpoint: bool) -> Result<()> {
    // in a simple expression: `lhs` is a name or a qualified name so far (S-FEEL rule 15), kept
    // as the path grows so that a long path is read in linear time
    let mut names = self.simple && matches!(lhs, AstNode::Name(_) | AstNode::QualifiedName(_));
    loop {
      let binary: fn(Box<AstNode>, Box<AstNode>) -> AstNode;
      let operator_level;
      match self.look(At::AfterOperand)? {
        T::Dot | T::Descendant => {
          names = names && self.kind == T::Dot;
          self.path(lhs)?;
          self.note("Path", start);
          if self.simple && !names {
            self.refuse(SIMPLE, "a path", start, "")?;
          }
          continue;
        }
        T::LeftBracket => {
          if end_endpoint {
            break;
          }
          self.filter(lhs)?;
          names = false;
          self.note("Filter", start);
          if self.simple {
            self.refuse(SIMPLE, "a filter", start, "")?;
          }
          continue;
        }
        T::LeftParen => {
          self.invocation(lhs)?;
          names = false;
          self.note("Invocation", start);
          if self.simple && !matches!(lhs, AstNode::FunctionInvocation(callee, parameters) if simple_temporal(callee, parameters)) {
            self.refuse(SIMPLE, "a function invocation", start, "")?;
          }
          continue;
        }
        T::Instance => {
          self.instance_of(lhs)?;
          names = false;
          self.note("InstanceOf", start);
          if self.simple {
            self.refuse(SIMPLE, "an instance of test", start, "")?;
          }
          continue;
        }
        T::As => {
          self.as_type(lhs)?;
          names = false;
          self.note("As", start);
          if self.simple {
            self.refuse(SIMPLE, "an `as` conversion", start, "")?;
          }
          continue;
        }
        T::Between => {
          if COMPARISON <= level {
            break;
          }
          self.between(lhs)?;
          names = false;
          self.note("Between", start);
          if self.simple {
            self.refuse(SIMPLE, "a between test", start, "")?;
          }
          continue;
        }
        T::In => {
          if COMPARISON <= level {
            break;
          }
          self.comparison_in(lhs)?;
          names = false;
          self.note("In", start);
          if self.simple {
            self.refuse(SIMPLE, "an in test", start, "")?;
          }
          continue;
        }
        T::Or => (binary, operator_level) = (AstNode::Or, OR),
        T::And => (binary, operator_level) = (AstNode::And, AND),
        T::Eq => (binary, operator_level) = (AstNode::Eq, COMPARISON),
        T::Ne => (binary, operator_level) = (AstNode::Nq, COMPARISON),
        T::Lt => (binary, operator_level) = (AstNode::Lt, COMPARISON),
        T::Le => (binary, operator_level) = (AstNode::Le, COMPARISON),
        T::Gt => (binary, operator_level) = (AstNode::Gt, COMPARISON),
        T::Ge => (binary, operator_level) = (AstNode::Ge, COMPARISON),
        T::Plus => (binary, operator_level) = (AstNode::Add, ADDITION),
        T::Minus => (binary, operator_level) = (AstNode::Sub, ADDITION),
        T::Mul => (binary, operator_level) = (AstNode::Mul, MULTIPLICATION),
        T::Div => (binary, operator_level) = (AstNode::Div, MULTIPLICATION),
        T::Exp => (binary, operator_level) = (AstNode::Exp, EXPONENTIATION),
        _ => break,
      }
      // the binary operators are left associative
      if operator_level <= level {
        break;
      }
      self.binary(lhs, binary, operator_level)?;
      names = false;
      self.note(operator_name(operator_level), start);
      if self.simple && operator_level <= AND {
        self.refuse(SIMPLE, if operator_level == OR { "a disjunction" } else { "a conjunction" }, start, "")?;
      }
    }
    Ok(())
  }

  /// A binary operator and its right operand. The parser looks at the operator.
  #[inline(never)]
  fn binary(&mut self, lhs: &mut AstNode, binary: fn(Box<AstNode>, Box<AstNode>) -> AstNode, operator_level: u8) -> Result<()> {
    self.bump();
    let rhs = self.boxed(operator_level)?;
    wrap(lhs, |lhs| binary(lhs, rhs));
    self.made = if operator_level >= ADDITION { Made::Arithmetic } else { Made::Textual };
    Ok(())
  }

  /// A path or a descendant: `.` or `...` and a name.
  #[inline(never)]
  fn path(&mut self, lhs: &mut AstNode) -> Result<()> {
    let descendant = self.kind == T::Descendant;
    self.bump();
    self.lexer.set_after_dot();
    let name = Box::new(AstNode::Name(self.name_as(At::Start, TokenClass::Property)?));
    wrap(lhs, |lhs| if descendant { AstNode::Descendant(lhs, name) } else { AstNode::Path(lhs, name) });
    self.made = Made::Textual;
    Ok(())
  }

  /// A filter: `[`, an expression and `]`.
  #[inline(never)]
  fn filter(&mut self, lhs: &mut AstNode) -> Result<()> {
    self.bump();
    self.lexer.enter_filter();
    let filter = self.boxed_inner();
    self.lexer.leave_filter();
    let filter = filter?;
    self.expect(T::RightBracket, At::AfterOperand)?;
    wrap(lhs, |lhs| AstNode::Filter(lhs, filter));
    self.made = Made::Textual;
    Ok(())
  }

  /// An invocation: `(` and the parameters.
  #[inline(never)]
  fn invocation(&mut self, lhs: &mut AstNode) -> Result<()> {
    self.bump();
    let parameters = Box::new(self.parameters()?);
    wrap(lhs, |lhs| AstNode::FunctionInvocation(lhs, parameters));
    self.made = Made::Textual;
    Ok(())
  }

  /// `as` and a type (DMN 1.7 Beta 1, grammar rule 51).
  #[inline(never)]
  fn as_type(&mut self, lhs: &mut AstNode) -> Result<()> {
    self.bump();
    self.lexer.set_type_name();
    let target_type = Box::new(self.type_expression()?);
    wrap(lhs, |lhs| AstNode::As(lhs, target_type));
    self.made = Made::Textual;
    Ok(())
  }

  /// `instance of` and a type.
  #[inline(never)]
  fn instance_of(&mut self, lhs: &mut AstNode) -> Result<()> {
    self.bump();
    self.expect(T::Of, At::Start)?;
    self.lexer.set_type_name();
    let expected_type = Box::new(self.type_expression()?);
    wrap(lhs, |lhs| AstNode::InstanceOf(lhs, expected_type));
    self.made = Made::Textual;
    Ok(())
  }

  /// `between`, the middle operand, `and` and the last operand.
  #[inline(never)]
  fn between(&mut self, lhs: &mut AstNode) -> Result<()> {
    self.bump();
    let between = std::mem::replace(&mut self.between, true);
    let middle = self.boxed(LOWEST);
    self.between = between;
    let middle = middle?;
    self.expect(T::BetweenAnd, At::AfterOperand)?;
    let rhs = self.boxed(COMPARISON)?;
    wrap(lhs, |lhs| AstNode::Between(lhs, middle, rhs));
    self.made = Made::Textual;
    Ok(())
  }

  /// `in` and its right side: an operand, or tests in parentheses.
  #[inline(never)]
  fn comparison_in(&mut self, lhs: &mut AstNode) -> Result<()> {
    self.bump();
    let rhs = if self.look(At::Start)? == T::LeftParen {
      let start = self.token_start;
      self.bump();
      let between = std::mem::replace(&mut self.between, false);
      let mut first = self.expression(LOWEST)?;
      match self.kind {
        T::Comma => {
          let mut items = vec![first];
          while self.kind == T::Comma {
            self.bump();
            self.push(&mut items)?;
          }
          self.expect(T::RightParen, At::AfterOperand)?;
          self.between = between;
          // the tests in parentheses are no operand: no operator of the right side follows them
          let tests = Box::new(AstNode::ExpressionList(items));
          wrap(lhs, |lhs| AstNode::In(lhs, tests));
          self.made = Made::Textual;
          return Ok(());
        }
        T::RightParen => self.bump(),
        T::Ellipsis => first = self.interval_end(first, IntervalType::Opened, start)?,
        _ => self.unexpected("expected", "`,`, `)` or `..` after the expression")?,
      }
      self.between = between;
      // the operators after the operand in parentheses: an open expression, as the operand
      // without parentheses is
      self.enter()?;
      self.infix(&mut first, start, COMPARISON, false)?;
      self.nesting -= 1;
      first
    } else {
      self.expression(COMPARISON)?
    };
    let rhs = Box::new(rhs);
    wrap(lhs, |lhs| AstNode::In(lhs, rhs));
    self.made = Made::Textual;
    Ok(())
  }

  /// An operand: a literal, a name, a parenthesized expression, an interval, a boxed
  /// expression, an expression that starts with a keyword, or an operand after a sign or a
  /// comparison operator.
  fn operand(&mut self) -> Result<AstNode> {
    self.made = Made::Textual;
    let kind = self.look(At::Start)?;
    let start = self.token_start;
    let node = self.operand_of(kind, start);
    if self.simple
      && let Ok(node) = &node
    {
      self.simple_operand(kind, start, node)?;
    }
    node
  }

  /// The operand that the token `kind` at `start` starts ([Parser::operand]).
  fn operand_of(&mut self, kind: T, start: usize) -> Result<AstNode> {
    match kind {
      T::Numeric => {
        self.made = Made::SimpleLiteral;
        let node = self.numeric();
        self.note("Number", start);
        node
      }
      T::String => {
        self.made = Made::SimpleLiteral;
        let node = self.string().map(AstNode::String);
        self.note("String", start);
        node
      }
      T::Boolean => {
        self.made = Made::SimpleLiteral;
        let node = if let TokenValue::Boolean(value) = self.take() {
          Ok(AstNode::Boolean(value))
        } else {
          self.error()
        };
        self.note("Boolean", start);
        node
      }
      T::At => self.at_literal(),
      T::Null => {
        self.bump();
        self.note("Null", start);
        Ok(AstNode::Null)
      }
      T::Name => {
        let node = self.name(At::Start).map(AstNode::Name);
        self.note("Name", start);
        node
      }
      T::NameDateTime => self.date_time_literal(),
      T::LeftParen => self.parenthesized(),
      T::LeftBracket => self.list_or_interval(true),
      T::RightBracket => self.interval_from_right_bracket(),
      T::LeftBrace => self.context(),
      T::Function => self.function_definition(),
      T::If => self.if_expression(),
      T::For => self.for_expression(),
      T::Some | T::Every => self.quantified_expression(),
      T::Minus => self.negation(),
      T::Lt => self.unary_comparison(AstNode::UnaryLt),
      T::Le => self.unary_comparison(AstNode::UnaryLe),
      T::Gt => self.unary_comparison(AstNode::UnaryGt),
      T::Ge => self.unary_comparison(AstNode::UnaryGe),
      T::Eq => self.unary_comparison(AstNode::UnaryEq),
      T::Ne => self.unary_comparison(AstNode::UnaryNe),
      _ => {
        // in a tolerant parse the operand is missing: nothing is taken
        self.unexpected("expected-expression", "an expression")?;
        let start = self.last_end;
        self.note("Error", start);
        Ok(AstNode::Null)
      }
    }
  }

  /// Takes the number the parser looks at.
  #[inline(never)]
  fn numeric(&mut self) -> Result<AstNode> {
    let TokenValue::Numeric(before, after, sign, exponent) = self.take() else {
      return self.error();
    };
    Ok(AstNode::Numeric(before, after, sign, exponent))
  }

  /// Takes the string the parser looks at.
  #[inline(never)]
  fn string(&mut self) -> Result<String> {
    let TokenValue::String(value) = self.take() else { return self.error() };
    Ok(value)
  }

  /// `@` and a string.
  #[inline(never)]
  fn at_literal(&mut self) -> Result<AstNode> {
    let start = self.token_start;
    self.bump();
    if self.look(At::Start)? != T::String {
      self.unexpected("expected", "a string after `@`")?;
      return Ok(AstNode::Null);
    }
    let node = self.string().map(AstNode::At);
    self.made = Made::SimpleLiteral;
    self.note("Temporal", start);
    node
  }

  /// A date and time literal: the name of its function, `(` and the parameters.
  #[inline(never)]
  fn date_time_literal(&mut self) -> Result<AstNode> {
    let start = self.token_start;
    self.role(TokenClass::Function);
    let TokenValue::NameDateTime(name) = self.take() else { return self.error() };
    self.expect(T::LeftParen, At::Start)?;
    let parameters = self.parameters()?;
    self.made = Made::SimpleLiteral;
    self.note("Temporal", start);
    Ok(AstNode::FunctionInvocation(Box::new(AstNode::Name(name)), Box::new(parameters)))
  }

  /// An arithmetic negation: its operand takes only what binds tighter than the arithmetic.
  #[inline(never)]
  fn negation(&mut self) -> Result<AstNode> {
    let start = self.token_start;
    self.bump();
    let operand = self.boxed(NEGATION)?;
    self.made = Made::Negation;
    self.note("Negation", start);
    Ok(AstNode::Neg(operand))
  }

  /// A unary comparison: its operand takes the arithmetic after it.
  #[inline(never)]
  fn unary_comparison(&mut self, unary: fn(Box<AstNode>) -> AstNode) -> Result<AstNode> {
    let start = self.token_start;
    self.bump();
    let operand = self.boxed(COMPARISON)?;
    self.made = Made::Textual;
    self.note("UnaryTest", start);
    Ok(unary(operand))
  }

  /// An expression in parentheses, or an interval that starts with `(`.
  #[inline(never)]
  fn parenthesized(&mut self) -> Result<AstNode> {
    let start = self.token_start;
    self.bump();
    let between = std::mem::replace(&mut self.between, false);
    let mut node = self.expression(LOWEST)?;
    match self.kind {
      T::RightParen => {
        self.bump();
        self.note("Parentheses", start);
      }
      T::Ellipsis => node = self.interval_end(node, IntervalType::Opened, start)?,
      _ => self.unexpected("expected", "`)` to close the `(`")?,
    }
    self.between = between;
    self.made = Made::Textual;
    Ok(node)
  }

  /// `if`, a condition, `then`, an expression, `else` and an expression.
  #[inline(never)]
  fn if_expression(&mut self) -> Result<AstNode> {
    let start = self.token_start;
    self.bump();
    let between = std::mem::replace(&mut self.between, false);
    let condition = self.boxed(LOWEST)?;
    self.expect(T::Then, At::AfterOperand)?;
    let then = self.boxed(LOWEST)?;
    self.expect(T::Else, At::AfterOperand)?;
    self.between = between;
    let otherwise = self.boxed(LOWEST)?;
    self.made = Made::Textual;
    self.note("If", start);
    Ok(AstNode::If(condition, then, otherwise))
  }

  /// The rest of an interval after its start endpoint: `..`, the end endpoint and the closing
  /// bracket. The parser looks at the `..`.
  #[inline(never)]
  fn interval_end(&mut self, start: AstNode, start_type: IntervalType, from: usize) -> Result<AstNode> {
    self.expect(T::Ellipsis, At::AfterOperand)?;
    let start = Box::new(AstNode::IntervalStart(Box::new(start), start_type));
    let end = Box::new(self.expression_until(LOWEST, true)?);
    let end_type = match self.kind {
      T::RightParen | T::LeftBracket => {
        self.bump();
        IntervalType::Opened
      }
      T::RightBracket => {
        self.bump();
        IntervalType::Closed
      }
      _ => {
        self.unexpected("expected", "`)`, `[` or `]` to close the interval")?;
        IntervalType::Opened
      }
    };
    self.note("Interval", from);
    Ok(AstNode::Range(start, Box::new(AstNode::IntervalEnd(end, end_type))))
  }

  /// An interval that starts with `]`. The parser looks at the `]`.
  #[inline(never)]
  fn interval_from_right_bracket(&mut self) -> Result<AstNode> {
    let start = self.token_start;
    self.bump();
    let between = std::mem::replace(&mut self.between, false);
    let interval = self.interval_after_right_bracket(start)?;
    self.between = between;
    self.made = Made::Textual;
    Ok(interval)
  }

  /// An interval after the `]` that starts it. A `[` after its start endpoint is a filter: the
  /// interval ends only after the `..` and the end endpoint.
  fn interval_after_right_bracket(&mut self, from: usize) -> Result<AstNode> {
    let start = self.expression_until(LOWEST, false)?;
    self.interval_end(start, IntervalType::Opened, from)
  }

  /// A list or an interval that starts with `[`; only a list without `interval`.
  #[inline(never)]
  fn list_or_interval(&mut self, interval: bool) -> Result<AstNode> {
    let start = self.token_start;
    self.bump();
    let first = if self.look(At::Start)? == T::RightBracket {
      let inner_start = self.token_start;
      self.bump();
      // an operand after `[]`: the `]` starts an interval, the first item of the list
      match self.look(At::AfterOperand)? {
        T::At | T::Every | T::For | T::LeftBrace | T::Null | T::Function | T::If | T::Some | T::Numeric | T::String | T::Boolean | T::Name | T::NameDateTime => {
          let between = std::mem::replace(&mut self.between, false);
          let mut first = self.interval_after_right_bracket(inner_start)?;
          self.infix(&mut first, inner_start, LOWEST, false)?;
          self.between = between;
          first
        }
        _ => {
          self.made = Made::Boxed;
          self.note("List", start);
          return Ok(AstNode::List(vec![]));
        }
      }
    } else {
      self.inner()?
    };
    let between = std::mem::replace(&mut self.between, false);
    let node = match self.kind {
      T::Ellipsis if interval => {
        let node = self.interval_end(first, IntervalType::Closed, start)?;
        self.made = Made::Textual;
        node
      }
      _ => {
        let mut items = vec![first];
        loop {
          while self.kind == T::Comma {
            self.bump();
            self.push(&mut items)?;
          }
          if self.kind == T::RightBracket || !self.skip_to(T::RightBracket, "`,` or `]` in the list")? {
            break;
          }
        }
        self.expect(T::RightBracket, At::AfterOperand)?;
        self.made = Made::Boxed;
        self.note("List", start);
        AstNode::List(items)
      }
    };
    self.between = between;
    Ok(node)
  }

  /// The parameters of an invocation after its `(`.
  #[inline(never)]
  fn parameters(&mut self) -> Result<AstNode> {
    let between = std::mem::replace(&mut self.between, false);
    let parameters = match self.look(At::Start)? {
      T::RightParen => {
        self.bump();
        AstNode::PositionalParameters(vec![])
      }
      T::Name => {
        let start = self.token_start;
        let name = self.name(At::Start)?;
        if self.look(At::AfterOperand)? == T::Colon {
          self.named_parameters(name)?
        } else {
          self.note("Name", start);
          self.enter()?;
          let mut first = AstNode::Name(name);
          self.infix(&mut first, start, LOWEST, false)?;
          self.nesting -= 1;
          self.positional_parameters(first)?
        }
      }
      _ => {
        let first = self.expression(LOWEST)?;
        self.positional_parameters(first)?
      }
    };
    self.between = between;
    Ok(parameters)
  }

  /// The named parameters after the name of the first one. The parser looks at the `:`.
  #[inline(never)]
  fn named_parameters(&mut self, mut name: Name) -> Result<AstNode> {
    if let Some(syntax) = &mut self.syntax
      && let Some(token) = syntax.tokens.iter_mut().rev().find(|token| token.class == TokenClass::Name)
    {
      // the name before the `:` the parser looks at
      token.class = TokenClass::Parameter;
    }
    let mut items = vec![];
    loop {
      self.expect(T::Colon, At::Start)?;
      let value = self.boxed(LOWEST)?;
      items.push(AstNode::NamedParameter(Box::new(AstNode::ParameterName(name)), value));
      match self.kind {
        T::Comma => {
          self.bump();
          name = self.name_as(At::Start, TokenClass::Parameter)?;
        }
        T::RightParen => break self.bump(),
        _ => {
          self.unexpected("expected", "`,` or `)` in the argument list")?;
          break;
        }
      }
    }
    Ok(AstNode::NamedParameters(items))
  }

  /// The positional parameters after the first one.
  #[inline(never)]
  fn positional_parameters(&mut self, first: AstNode) -> Result<AstNode> {
    let mut items = vec![first];
    loop {
      while self.kind == T::Comma {
        self.bump();
        self.push(&mut items)?;
      }
      if self.kind == T::RightParen || !self.skip_to(T::RightParen, "`,` or `)` in the argument list")? {
        break;
      }
    }
    self.expect(T::RightParen, At::AfterOperand)?;
    Ok(AstNode::PositionalParameters(items))
  }

  /// A context. The parser looks at its `{`.
  #[inline(never)]
  fn context(&mut self) -> Result<AstNode> {
    let start = self.token_start;
    self.bump();
    self.lexer.push_to_scope();
    // an entry can read an entry written after it: the keys of several tokens are names in scope
    // before the entries are parsed
    // the place of the `{` from the lexer: the parser keeps the places of tokens only for a tolerant parse
    for key in crate::lexer::literal_keys(self.input, self.lexer.token_start()) {
      self.lexer.add_name_to_scope(key);
    }
    let between = std::mem::replace(&mut self.between, false);
    let mut entries = vec![];
    if self.look(At::Start)? == T::RightBrace {
      self.bump();
    } else {
      loop {
        let key = self.look(At::Start)?;
        let entry_start = self.token_start;
        let name = match key {
          T::Name => self.name_as(At::Start, TokenClass::Key)?,
          T::String => {
            self.role(TokenClass::Key);
            Name::from(self.string()?)
          }
          _ => {
            self.unexpected("expected", "a key (a name or a string) for the context entry")?;
            Name::from("")
          }
        };
        self.expect(T::Colon, At::Start)?;
        // a key that is a string in another form than FEEL reads a name is read by that name
        let read_as = if rheon_feel::names::is_in_name_form(name.as_str()) {
          name.clone()
        } else {
          Name::from(rheon_feel::names::name_form(name.as_str()))
        };
        // an entry does not read its own key (DMN 1.6 clause 10.3.2.6)
        self.lexer.remove_name_from_scope(&read_as);
        let value = self.boxed(LOWEST)?;
        // the key is a name for the entries after it, and a member name from here on
        self.lexer.add_name_to_scope(read_as.clone());
        self.lexer.add_parsed_name(read_as);
        entries.push(AstNode::ContextEntry(Box::new(AstNode::ContextEntryKey(name)), value));
        self.note("Entry", entry_start);
        match self.kind {
          T::Comma => self.bump(),
          T::RightBrace => break self.bump(),
          _ => {
            self.unexpected("expected", "`,` or `}` in the context")?;
            break;
          }
        }
      }
    }
    self.lexer.pop_from_scope();
    self.between = between;
    self.made = Made::Boxed;
    self.note("Context", start);
    Ok(AstNode::Context(entries))
  }

  /// A function definition. The parser looks at its `function`.
  #[inline(never)]
  fn function_definition(&mut self) -> Result<AstNode> {
    let start = self.token_start;
    self.bump();
    self.expect(T::LeftParen, At::Start)?;
    self.scope.push_default();
    let parameters = Box::new(self.formal_parameters()?);
    let external = self.look(At::Start)? == T::External;
    if external {
      self.bump();
    }
    let body = self.boxed(LOWEST)?;
    self.scope.pop();
    self.made = Made::Boxed;
    self.note("Function", start);
    Ok(AstNode::FunctionDefinition(parameters, Box::new(AstNode::FunctionBody(body, external))))
  }

  /// The formal parameters of a function definition after their `(`.
  #[inline(never)]
  fn formal_parameters(&mut self) -> Result<AstNode> {
    let between = std::mem::replace(&mut self.between, false);
    let mut parameters = vec![];
    // a parameter is a name being declared: it holds `as` and a type (FEEL 1.6 names)
    self.lexer.set_declared_name();
    if self.look(At::Start)? == T::RightParen {
      self.bump();
    } else {
      loop {
        let name = self.name_as(At::Start, TokenClass::Parameter)?;
        let parameter_type = if self.look(At::AfterOperand)? == T::Colon {
          self.bump();
          self.lexer.set_type_name();
          self.type_expression()?
        } else {
          AstNode::FeelType(FeelType::Any)
        };
        self.scope.set_entry_name(name.clone());
        parameters.push(AstNode::FormalParameter(Box::new(AstNode::ParameterName(name)), Box::new(parameter_type)));
        match self.look(At::AfterOperand)? {
          T::Comma => {
            self.bump();
            self.lexer.set_declared_name();
          }
          T::RightParen => break self.bump(),
          _ => {
            self.unexpected("expected", "`,` or `)` in the argument list")?;
            break;
          }
        }
      }
    }
    self.between = between;
    Ok(AstNode::FormalParameters(parameters))
  }

  /// A `for` expression. The parser looks at its `for`.
  #[inline(never)]
  fn for_expression(&mut self) -> Result<AstNode> {
    let start = self.token_start;
    self.bump();
    self.lexer.push_to_scope();
    self.lexer.add_name_to_scope(Name::from("partial"));
    let contexts = Box::new(self.iteration_contexts()?);
    let body = self.boxed(LOWEST)?;
    self.lexer.pop_from_scope();
    self.made = Made::Textual;
    self.note("For", start);
    Ok(AstNode::For(contexts, Box::new(AstNode::EvaluatedExpression(body))))
  }

  /// The iteration contexts of a `for` expression and its `return`.
  #[inline(never)]
  fn iteration_contexts(&mut self) -> Result<AstNode> {
    let between = std::mem::replace(&mut self.between, false);
    let mut contexts = vec![];
    loop {
      self.lexer.set_till_in();
      let name = self.name_as(At::Start, TokenClass::Variable)?;
      let variable = Box::new(AstNode::Name(name.clone()));
      self.lexer.add_name_to_scope(name);
      self.expect(T::In, At::Start)?;
      let start = self.boxed(LOWEST)?;
      contexts.push(if self.kind == T::Ellipsis {
        self.bump();
        let end = self.boxed(LOWEST)?;
        AstNode::IterationContextInterval(variable, start, end)
      } else {
        AstNode::IterationContextSingle(variable, start)
      });
      if self.kind != T::Comma {
        break;
      }
      self.bump();
    }
    self.expect(T::Return, At::AfterOperand)?;
    self.between = between;
    Ok(AstNode::IterationContexts(contexts))
  }

  /// A `some` or `every` expression. The parser looks at its first keyword.
  #[inline(never)]
  fn quantified_expression(&mut self) -> Result<AstNode> {
    let every = self.kind == T::Every;
    let start = self.token_start;
    self.bump();
    self.lexer.push_to_scope();
    let contexts = Box::new(self.quantified_contexts()?);
    let body = self.boxed(LOWEST)?;
    self.lexer.pop_from_scope();
    self.made = Made::Textual;
    self.note("Quantified", start);
    let satisfies = Box::new(AstNode::Satisfies(body));
    Ok(if every {
      AstNode::Every(contexts, satisfies)
    } else {
      AstNode::Some(contexts, satisfies)
    })
  }

  /// The variables of a `some` or `every` expression with their lists, and its `satisfies`.
  #[inline(never)]
  fn quantified_contexts(&mut self) -> Result<AstNode> {
    let between = std::mem::replace(&mut self.between, false);
    let mut contexts = vec![];
    loop {
      self.lexer.set_till_in();
      let name = self.name_as(At::Start, TokenClass::Variable)?;
      let variable = Box::new(AstNode::Name(name.clone()));
      self.lexer.add_name_to_scope(name);
      self.expect(T::In, At::Start)?;
      let domain = self.boxed(LOWEST)?;
      // an iteration context, as of `for` (DMN 1.7 Beta 1, grammar rule 46): a list, or `a..b`
      contexts.push(if self.kind == T::Ellipsis {
        self.bump();
        let end = self.boxed(LOWEST)?;
        AstNode::IterationContextInterval(variable, domain, end)
      } else {
        AstNode::QuantifiedContext(variable, domain)
      });
      if self.kind != T::Comma {
        break;
      }
      self.bump();
    }
    self.expect(T::Satisfies, At::AfterOperand)?;
    self.between = between;
    Ok(AstNode::QuantifiedContexts(contexts))
  }

  /// A type. The lexer was told that a type is expected.
  #[inline(never)]
  fn type_expression(&mut self) -> Result<AstNode> {
    self.enter()?;
    let kind = self.look(At::Start)?;
    let start = self.token_start;
    let node = match kind {
      T::BuiltInTypeName => {
        self.role(TokenClass::Type);
        let TokenValue::BuiltInTypeName(name) = self.take() else { return self.error() };
        AstNode::FeelType((&name).into())
      }
      T::Name => {
        let name = self.name_as(At::Start, TokenClass::Type)?;
        // the abstract types of DMN 1.7 (Figure 10-26), and `context` without entries: decided
        // where the name is read, so a name of the model or of the context wins
        match abstract_type(&name) {
          Some(feel_type) if self.look(At::AfterOperand)? != T::Dot => AstNode::AbstractType(Box::new(self.qualified_name(name)?), Box::new(AstNode::FeelType(feel_type))),
          _ => self.qualified_name(name)?,
        }
      }
      T::List | T::Range => self.list_or_range_type()?,
      T::Context => self.context_type()?,
      T::Function => self.function_type()?,
      _ => {
        self.unexpected("expected-type", "a type")?;
        AstNode::Null
      }
    };
    self.nesting -= 1;
    self.note("Type", start);
    Ok(node)
  }

  /// `list<` or `range<`, a type and `>`.
  fn list_or_range_type(&mut self) -> Result<AstNode> {
    let list = self.kind == T::List;
    self.bump();
    self.expect(T::Lt, At::Start)?;
    self.lexer.set_type_name();
    let between = std::mem::replace(&mut self.between, false);
    let item_type = Box::new(self.type_expression()?);
    self.expect(T::Gt, At::AfterOperand)?;
    self.between = between;
    Ok(if list { AstNode::ListType(item_type) } else { AstNode::RangeType(item_type) })
  }

  /// `context<`, the entries with their types and `>`.
  fn context_type(&mut self) -> Result<AstNode> {
    self.bump();
    self.expect(T::Lt, At::Start)?;
    let between = std::mem::replace(&mut self.between, false);
    let mut entries = vec![];
    // `context<>`, the context type without entries: every context (DMN 1.7 Beta 1, rule 54)
    if self.look(At::Start)? == T::Gt {
      self.bump();
      self.between = between;
      return Ok(AstNode::ContextType(entries));
    }
    loop {
      let name = self.name(At::Start)?;
      self.expect(T::Colon, At::Start)?;
      self.lexer.set_type_name();
      let entry_type = Box::new(self.type_expression()?);
      entries.push(AstNode::ContextTypeEntry(Box::new(AstNode::ContextTypeEntryKey(name)), entry_type));
      match self.look(At::AfterOperand)? {
        T::Comma => self.bump(),
        T::Gt => break self.bump(),
        _ => {
          self.unexpected("expected", "`,` or `>` in the type")?;
          break;
        }
      }
    }
    self.between = between;
    Ok(AstNode::ContextType(entries))
  }

  /// `function<`, the types of the parameters, `>`, `->` and the type of the result.
  fn function_type(&mut self) -> Result<AstNode> {
    self.bump();
    self.expect(T::Lt, At::Start)?;
    self.lexer.set_type_name();
    let between = std::mem::replace(&mut self.between, false);
    let mut parameter_types = vec![];
    if self.look(At::Start)? == T::Gt {
      self.bump();
    } else {
      loop {
        parameter_types.push(self.type_expression()?);
        match self.look(At::AfterOperand)? {
          T::Comma => {
            self.bump();
            self.lexer.set_type_name();
          }
          T::Gt => break self.bump(),
          _ => {
            self.unexpected("expected", "`,` or `>` in the type")?;
            break;
          }
        }
      }
    }
    self.expect(T::RightArrow, At::Start)?;
    self.lexer.set_type_name();
    self.between = between;
    let result_type = Box::new(self.type_expression()?);
    Ok(AstNode::FunctionType(Box::new(AstNode::ParameterTypes(parameter_types)), result_type))
  }
}

#[cfg(test)]
mod stack_tests {
  use super::*;

  /// Prints the stack one level of nesting takes, for some kinds of nesting.
  #[test]
  fn stack_per_level() {
    let n = 200;
    let cases = [
      ("parentheses", format!("{}1{}", "(".repeat(n), ")".repeat(n)), n),
      ("sum in parentheses", format!("{}1{}", "(1+".repeat(n), ")".repeat(n)), 2 * n),
      ("list", format!("{}1{}", "[".repeat(n), "]".repeat(n)), n),
      ("context", format!("{}1{}", "{a:".repeat(n), "}".repeat(n)), n),
      ("negation", format!("{}1", "-".repeat(n)), n),
      ("if", format!("{}1", "if true then 1 else ".repeat(n)), n),
      ("invocation", format!("{}1{}", "f(".repeat(n), ")".repeat(n)), n),
      ("for", format!("{}x", "for x in [1] return ".repeat(n)), n),
      ("function", format!("{}1", "function(a) ".repeat(n)), n),
      ("filter", format!("{}1{}", "a[".repeat(n), "]".repeat(n)), n),
      ("between", format!("{}1{}", "1 between ".repeat(n), " and 1".repeat(n)), n),
      ("unary comparison", format!("{}1", "< ".repeat(n)), n),
    ];
    for (name, input, levels) in cases {
      let handle = std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
          let scope = ParsingScope::new(std::iter::empty());
          let top = 0u8;
          let top = &top as *const u8 as usize;
          STACK_LOW.with(|low| low.set(usize::MAX));
          let result = Parser::new(&scope, TokenType::StartExpression, &input).parse();
          assert!(result.is_ok(), "{name}");
          let low = STACK_LOW.with(|low| low.get());
          println!("STACK {name}: {} bytes per level of {levels}", (top - low) / levels);
        })
        .unwrap();
      handle.join().unwrap();
    }
  }
}
