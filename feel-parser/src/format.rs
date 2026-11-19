//! A formatter of `FEEL` text: one predictable layout for every expression.
//!
//! The text is parsed with the tolerant parse, which keeps every token (comments included) and
//! the place of every part. A part that fits the rest of its line is written on it, with one
//! space where the grammar reads better with one. A part that does not fit is broken in the way
//! of its kind, and the parts inside it are indented by [INDENT] spaces:
//!
//! - a list, a context, the arguments of an invocation, a filter and an expression in
//!   parentheses: every item on a line of its own between the brackets; a context of two or
//!   more entries is always written this way, whatever its length;
//! - `if`: `then` and `else` start a line; `for`, `some` and `every`: `return` or `satisfies`
//!   starts a line, and so does every variable after the first;
//! - a chain of one kind of operator (`and`, `or`, a comparison, `+ -`, `* /`, `**`): every
//!   operator starts a line.
//!
//! The layout depends only on the tokens and the width, never on the layout of the input, so
//! formatting a formatted text changes nothing. Names, numbers, strings, comments and
//! parentheses are written as they stand in the input. The result is checked before it is
//! returned: it must parse to the same tree as the input.

use crate::AstNode;
use crate::parser::{Diagnostic, Parser, Syntax, TokenClass};
use crate::scope::ParsingScope;
use crate::token::TokenType;

/// The spaces a nested part is indented by.
pub const INDENT: usize = 2;

/// The width of a line unless the caller names another.
pub const DEFAULT_WIDTH: usize = 80;

/// The deepest tree whose text is formatted: the comparison of the trees is recursive.
const MAX_DEPTH: usize = 300;

/// What formatting a text gave.
#[derive(Debug, PartialEq)]
pub enum Formatted {
  /// The formatted text.
  Text(String),
  /// The text has syntax errors and is not formatted.
  Errors(Vec<Diagnostic>),
  /// The text is not formatted: it is nested too deeply to be checked, or the check failed.
  Unchanged,
}

/// How a token stands to the tokens beside it.
#[derive(Clone, Copy, PartialEq)]
enum Role {
  /// A space on both sides, unless the token beside it says otherwise.
  Plain,
  /// An opening bracket: no space after it.
  Open,
  /// An opening bracket that belongs to what stands before it (the `(` of an invocation, the
  /// `[` of a filter, the `<` of a type): no space before it and none after it.
  Attached,
  /// A closing bracket: no space before it.
  Close,
  /// A comma or a colon: no space before it.
  Follows,
  /// A dot, `..` or `...`: no space on either side.
  Tight,
  /// A sign or `@`: no space after it.
  Prefix,
}

struct Token {
  start: usize,
  end: usize,
  role: Role,
  /// A comment that runs to the end of its line: a line break follows it.
  line_comment: bool,
  comment: bool,
}

enum Child {
  Token(usize),
  Node(usize),
}

struct Node {
  kind: &'static str,
  children: Vec<Child>,
  /// The first and the last token of the node.
  first: usize,
  last: usize,
}

/// How a node that does not fit its line is broken.
#[derive(Clone, Copy, PartialEq)]
enum Break {
  /// Not at its own tokens; the parts inside it break by their kind.
  None,
  /// After the opening bracket, after every comma and before the closing bracket.
  Brackets,
  /// Before `then`, `else`, `return` and `satisfies`, and after every comma.
  Keywords,
  /// Before every operator of the chain.
  Chain,
  /// After the colon of a context entry, when the value is not a bracketed part.
  Entry,
}

fn break_of(kind: &str) -> Break {
  match kind {
    "List" | "Context" | "Parentheses" | "Invocation" | "Filter" => Break::Brackets,
    "If" | "For" | "Quantified" => Break::Keywords,
    "Disjunction" | "Conjunction" | "Comparison" | "Addition" | "Multiplication" | "Exponentiation" => Break::Chain,
    "Entry" => Break::Entry,
    _ => Break::None,
  }
}

struct Formatter<'a> {
  input: &'a str,
  tokens: Vec<Token>,
  nodes: Vec<Node>,
  /// For each token, the length of the flat text of the tokens up to and with it, and the
  /// number of tokens up to and with it that keep a part from standing on one line: a comment
  /// that runs to the end of its line, and the `{` of a context of two or more entries.
  flat: Vec<usize>,
  line_comments: Vec<usize>,
  width: usize,
  out: String,
  /// The column after the last character written, and the indentation of the line.
  column: usize,
  line_indent: usize,
  /// The token written last on this line, if any.
  previous: Option<usize>,
  /// A line comment was written: the next token starts a line.
  must_break: bool,
}

impl<'a> Formatter<'a> {
  /// Builds the tree of the nodes and tokens of a text without syntax errors.
  fn new(input: &'a str, syntax: &Syntax, width: usize) -> Self {
    let tokens: Vec<Token> = syntax
      .tokens
      .iter()
      .filter(|token| token.class != TokenClass::Whitespace)
      .map(|token| {
        let text = &input[token.start..token.end];
        let comment = token.class == TokenClass::Comment;
        let role = match text {
          _ if comment => Role::Plain,
          "(" | "[" | "{" => Role::Open,
          ")" | "]" | "}" => Role::Close,
          "," | ":" => Role::Follows,
          "." | ".." | "..." => Role::Tight,
          "@" => Role::Prefix,
          _ => Role::Plain,
        };
        Token {
          start: token.start,
          end: token.end,
          role,
          line_comment: comment && text.starts_with("//"),
          comment,
        }
      })
      .collect();
    // the nodes stand each before the nodes inside it: a stack of the open ones
    let mut nodes = vec![Node {
      kind: "Root",
      children: vec![],
      first: 0,
      last: tokens.len().saturating_sub(1),
    }];
    let mut ends = vec![usize::MAX];
    let mut open = vec![0];
    let mut next = 0;
    for (index, token) in tokens.iter().enumerate() {
      // a node that ended before this token: its last token is the one before
      while open.len() > 1 && ends[*open.last().unwrap_or(&0)] <= token.start {
        if let Some(closed) = open.pop() {
          nodes[closed].last = index - 1;
        }
      }
      while next < syntax.nodes.len() && syntax.nodes[next].start <= token.start {
        let node = &syntax.nodes[next];
        next += 1;
        if node.end <= node.start || node.end <= token.start {
          continue;
        }
        while open.len() > 1 && ends[*open.last().unwrap_or(&0)] <= node.start {
          if let Some(closed) = open.pop() {
            nodes[closed].last = index - 1;
          }
        }
        let created = nodes.len();
        nodes.push(Node {
          kind: node.kind,
          children: vec![],
          first: index,
          last: index,
        });
        ends.push(node.end);
        nodes[*open.last().unwrap_or(&0)].children.push(Child::Node(created));
        open.push(created);
      }
      nodes[*open.last().unwrap_or(&0)].children.push(Child::Token(index));
    }
    // the nodes still open hold the last token
    for node in open {
      nodes[node].last = tokens.len().saturating_sub(1);
    }
    let mut formatter = Self {
      input,
      tokens,
      nodes,
      flat: vec![],
      line_comments: vec![],
      width,
      out: String::with_capacity(input.len() + input.len() / 8),
      column: 0,
      line_indent: 0,
      previous: None,
      must_break: false,
    };
    formatter.assign_roles();
    // a context of two or more entries has every entry on a line of its own
    let mut always_broken = vec![false; formatter.tokens.len()];
    for node in &formatter.nodes {
      let entries = node
        .children
        .iter()
        .filter(|child| matches!(child, Child::Node(entry) if formatter.nodes[*entry].kind == "Entry"))
        .count();
      if node.kind == "Context" && entries >= 2 {
        always_broken[node.first] = true;
      }
    }
    let (mut length, mut comments) = (0, 0);
    for (index, &broken) in always_broken.iter().enumerate() {
      if index > 0 && formatter.space_between(index - 1, index) {
        length += 1;
      }
      length += formatter.tokens[index].end - formatter.tokens[index].start;
      comments += usize::from(formatter.tokens[index].line_comment || broken);
      formatter.flat.push(length);
      formatter.line_comments.push(comments);
    }
    formatter
  }

  fn text(&self, token: usize) -> &'a str {
    &self.input[self.tokens[token].start..self.tokens[token].end]
  }

  /// The tokens that are children of the node itself, comments left out.
  fn own_tokens(&self, node: usize) -> Vec<usize> {
    self.nodes[node]
      .children
      .iter()
      .filter_map(|child| match child {
        Child::Token(token) if !self.tokens[*token].comment => Some(*token),
        _ => None,
      })
      .collect()
  }

  /// Gives the tokens whose place depends on the node they stand in their roles.
  fn assign_roles(&mut self) {
    for node in 0..self.nodes.len() {
      let own = self.own_tokens(node);
      let kind = self.nodes[node].kind;
      let first_with = |formatter: &Self, text: &str| own.iter().copied().find(|&token| formatter.text(token) == text);
      match kind {
        // the sign of a negation
        "Negation" => {
          if let Some(&sign) = own.first() {
            self.tokens[sign].role = Role::Prefix;
          }
        }
        // the brackets of an invocation, of a date and time literal and of a filter belong to
        // what stands before them
        "Invocation" | "Temporal" | "Function" => {
          if let Some(bracket) = first_with(self, "(") {
            self.tokens[bracket].role = Role::Attached;
          }
        }
        "Filter" => {
          if let Some(bracket) = first_with(self, "[") {
            self.tokens[bracket].role = Role::Attached;
          }
        }
        // an interval opens and closes with either kind of bracket
        "Interval" => {
          if let (Some(&opening), Some(&closing)) = (own.first(), own.last()) {
            self.tokens[opening].role = Role::Open;
            self.tokens[closing].role = Role::Close;
          }
        }
        // the angle brackets of a type
        "Type" => {
          for &token in &own {
            match self.text(token) {
              "<" => self.tokens[token].role = Role::Attached,
              ">" => self.tokens[token].role = Role::Close,
              _ => {}
            }
          }
        }
        // `not(` of unary tests
        "Root" => {
          for pair in own.windows(2) {
            if self.text(pair[0]) == "not" && self.text(pair[1]) == "(" {
              self.tokens[pair[1]].role = Role::Attached;
            }
          }
        }
        _ => {}
      }
    }
  }

  /// Whether a space stands between two tokens that follow each other.
  fn space_between(&self, left: usize, right: usize) -> bool {
    !matches!(self.tokens[left].role, Role::Open | Role::Attached | Role::Tight | Role::Prefix)
      && !matches!(self.tokens[right].role, Role::Close | Role::Follows | Role::Tight | Role::Attached)
  }

  /// The length of the tokens `first..=last` on one line.
  fn flat_length(&self, first: usize, last: usize) -> usize {
    self.flat[last] - self.flat[first] + (self.tokens[first].end - self.tokens[first].start)
  }

  /// Whether the node can stand on one line: it holds no comment that runs to the end of a line
  /// and no context of two or more entries.
  fn can_be_flat(&self, node: usize) -> bool {
    let (first, last) = (self.nodes[node].first, self.nodes[node].last);
    let before = if first == 0 { 0 } else { self.line_comments[first - 1] };
    self.line_comments[last] == before
  }

  /// Whether the node fits the rest of the line.
  fn fits(&self, node: usize) -> bool {
    let (first, last) = (self.nodes[node].first, self.nodes[node].last);
    let space = usize::from(self.previous.is_some_and(|previous| self.space_between(previous, first)));
    self.can_be_flat(node) && self.column + space + self.flat_length(first, last) <= self.width
  }

  fn newline(&mut self, indent: usize) {
    let kept = self.out.trim_end_matches(' ').len();
    self.out.truncate(kept);
    self.out.push('\n');
    self.out.extend(std::iter::repeat_n(' ', indent));
    self.column = indent;
    self.line_indent = indent;
    self.previous = None;
    self.must_break = false;
  }

  fn write(&mut self, token: usize) {
    if self.must_break {
      self.newline(self.line_indent);
    }
    if self.previous.is_some_and(|previous| self.space_between(previous, token)) {
      self.out.push(' ');
      self.column += 1;
    }
    let text = self.text(token);
    self.out.push_str(text);
    // a comment of several lines keeps its lines: the column is the length of its last line
    self.column = match text.rfind('\n') {
      Some(at) => text.len() - at - 1,
      None => self.column + text.chars().count(),
    };
    self.previous = Some(token);
    self.must_break = self.tokens[token].line_comment;
  }

  /// Writes a node: on the line when it fits, broken by its kind otherwise.
  fn node(&mut self, node: usize, indent: usize) {
    if self.fits(node) {
      for token in self.nodes[node].first..=self.nodes[node].last {
        self.write(token);
      }
    } else {
      self.broken(node, indent);
    }
  }

  /// Writes a node that does not fit its line.
  fn broken(&mut self, node: usize, indent: usize) {
    let kind = self.nodes[node].kind;
    let how = break_of(kind);
    let own = self.own_tokens(node);
    // the brackets that hold the items of the node
    let opening = own.iter().copied().find(|&token| matches!(self.tokens[token].role, Role::Open | Role::Attached));
    let closing = own.iter().copied().rev().find(|&token| self.tokens[token].role == Role::Close);
    let inner = indent + INDENT;
    for position in 0..self.nodes[node].children.len() {
      match self.nodes[node].children[position] {
        Child::Token(token) => {
          let text = self.text(token);
          let comment = self.tokens[token].comment;
          match how {
            Break::Brackets if Some(token) == closing => self.newline(indent),
            Break::Keywords if !comment && matches!(text, "then" | "else" | "return" | "satisfies") => self.newline(indent),
            Break::Chain if !comment => self.newline(inner),
            _ => {}
          }
          self.write(token);
          match how {
            Break::Brackets if Some(token) == opening || (!comment && text == ",") => self.newline(inner),
            Break::Keywords if !comment && text == "," => self.newline(inner + INDENT),
            _ => {}
          }
        }
        Child::Node(child) => match how {
          // the left operand of a chain that is a chain of the same kind goes on with it
          Break::Chain if position == 0 && self.nodes[child].kind == kind => self.broken(child, indent),
          // the value of an entry that is no bracketed part and does not fit stands below its key
          Break::Entry if !self.fits(child) && break_of(self.nodes[child].kind) != Break::Brackets && self.nodes[child].kind != "Function" => {
            self.newline(inner);
            self.node(child, inner);
          }
          // a bracketed value closes below its key; the body of a function below `function`
          Break::Entry => self.node(child, indent),
          Break::None if kind == "Function" || kind == "Root" => self.node(child, indent),
          // a bracketed part after a keyword, or one that a path or an operator of one operand
          // continues, closes at the indentation of its line
          Break::Keywords | Break::None if break_of(self.nodes[child].kind) == Break::Brackets => self.node(child, indent),
          _ => self.node(child, inner),
        },
      }
    }
  }
}

/// Formats a text: an expression, or unary tests with `unary_tests`. `width` is the most
/// characters of a line that the layout aims at; a part that can not be broken may run past it.
///
/// `scope` makes the parsing scope of a parse: every parse has its own, since a parse adds the
/// names of the text to its scope.
pub(crate) fn format<'s>(scope: &dyn Fn() -> ParsingScope<'s>, input: &str, unary_tests: bool, width: usize) -> Formatted {
  let start = || if unary_tests { TokenType::StartUnaryTests } else { TokenType::StartExpression };
  let syntax = Parser::new(&scope(), start(), input).parse_tolerant();
  if !syntax.diagnostics.is_empty() {
    return Formatted::Errors(syntax.diagnostics);
  }
  let mut formatter = Formatter::new(input, &syntax, width.max(1));
  if formatter.tokens.is_empty() {
    return Formatted::Text(String::new());
  }
  // a text that is nested more deeply than an expression may be is left as it is, before the
  // layout follows its nesting: a chain of operators is as deep as it is long
  let parse = |text: &str| -> Option<AstNode> { Parser::new(&scope(), start(), text).parse().ok().filter(|node| node.depth() <= MAX_DEPTH) };
  let Some(before) = parse(input) else {
    return Formatted::Unchanged;
  };
  formatter.node(0, 0);
  let formatted = formatter.out;
  // the formatted text must be the same expression
  match (Some(before), parse(&formatted)) {
    (Some(before), Some(after)) if before == after => Formatted::Text(formatted),
    _other => {
      #[cfg(test)]
      if std::env::var_os("RHEON_FORMAT_DEBUG").is_some() {
        eprintln!(
          "FORMAT-DIFF input {input:?}\n  formatted {formatted:?}\n  trees {:?}",
          _other.0.is_some() && _other.1.is_some()
        );
      }
      Formatted::Unchanged
    }
  }
}
