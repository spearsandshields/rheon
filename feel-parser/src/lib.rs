#![doc = include_str!("../docs/README.md")]
#![deny(rustdoc::broken_intra_doc_links)]

#[macro_use]
extern crate dsntk_macros;

/// Counts `n` units of parsing work in tests (see `tests::work`); nothing in other builds.
macro_rules! work {
  ($work:ident, $n:expr) => {{
    #[cfg(test)]
    crate::tests::work::count(crate::tests::work::Work::$work, $n);
  }};
}

mod ast;
mod closure;
mod composite;
mod context;
mod errors;
mod format;
mod index;
mod lexer;
mod parser;
mod scope;
mod token;

#[cfg(test)]
mod tests;

pub use ast::AstNode;
pub use closure::ClosureBuilder;
use scope::ParsingScope;

use crate::errors::*;
use crate::parser::Parser;
use crate::token::TokenType;
use dsntk_common::Result;
use rheon_feel::{FeelScope, Name};

/// Parses the input in a parsing scope over the contexts of `scope` (borrowed, not copied;
/// nothing changes `scope` while parsing).
fn parse(scope: &FeelScope, start_token_type: TokenType, input: &str) -> Result<AstNode> {
  let frames = scope.frames();
  let parsing_scope = ParsingScope::new(frames.iter().map(|frame| &**frame));
  Parser::new(&parsing_scope, start_token_type, input).parse()
}

/// Parses an `expression` as defined in grammar rule `1`: the names of `scope` are the names in
/// scope of the text.
pub fn parse_expression(scope: &FeelScope, input: &str) -> Result<AstNode> {
  parse(scope, TokenType::StartExpression, input)
}

/// Parses `unary tests` as defined in grammar rule `15`.
pub fn parse_unary_tests(scope: &FeelScope, input: &str) -> Result<AstNode> {
  parse(scope, TokenType::StartUnaryTests, input)
}

/// Parses the input as the start rule `rule` asks ([StartRule]): an expression, unary tests, a
/// simple expression of S-FEEL, or a list of simple or textual expressions.
pub fn parse_start(scope: &FeelScope, rule: StartRule, input: &str) -> Result<AstNode> {
  let frames = scope.frames();
  let parsing_scope = ParsingScope::new(frames.iter().map(|frame| &**frame));
  Parser::new(&parsing_scope, rule.token(), input).with_rule(rule).parse()
}

/// Contexts with their indexes of names, kept for several parses: a text that is parsed in a
/// context of many names reads an index of them, and a parse of its own builds that index anew
/// ([parse_expression]). See [sharing].
pub struct Shared<'a> {
  contexts: &'a [std::sync::Arc<rheon_feel::context::FeelContext>],
  indexes: &'a [index::FrameIndex<'a>],
}

/// Calls `f` with the indexes of `contexts`, the contexts of a scope bottom first, for the
/// parses that `f` makes in a scope that holds them ([FeelScope::push_shared]).
pub fn sharing<R>(contexts: &[std::sync::Arc<rheon_feel::context::FeelContext>], f: impl FnOnce(&Shared<'_>) -> R) -> R {
  let indexes: Vec<index::FrameIndex<'_>> = contexts.iter().map(|context| index::FrameIndex::new(context)).collect();
  f(&Shared { contexts, indexes: &indexes })
}

impl Shared<'_> {
  /// Parses the input as [parse] does. The indexes kept are used when `scope` holds exactly the
  /// contexts they were made of, shared and not written into; otherwise the text is parsed in
  /// `scope` as it is.
  fn parse(&self, scope: &FeelScope, rule: StartRule, input: &str) -> Result<AstNode> {
    let same = {
      let frames = scope.frames();
      frames.len() == self.contexts.len() && frames.iter().zip(self.contexts).all(|(frame, context)| frame.shares(context))
    };
    if !same {
      return parse_start(scope, rule, input);
    }
    let parsing_scope = ParsingScope::sharing(self.indexes);
    Parser::new(&parsing_scope, rule.token(), input).with_rule(rule).parse()
  }

  /// [parse_start] in `scope`.
  pub fn parse_start(&self, scope: &FeelScope, rule: StartRule, input: &str) -> Result<AstNode> {
    self.parse(scope, rule, input)
  }

  /// [parse_expression] in `scope`.
  pub fn parse_expression(&self, scope: &FeelScope, input: &str) -> Result<AstNode> {
    self.parse(scope, StartRule::Expression, input)
  }

  /// [parse_unary_tests] in `scope`.
  pub fn parse_unary_tests(&self, scope: &FeelScope, input: &str) -> Result<AstNode> {
    self.parse(scope, StartRule::UnaryTests, input)
  }
}

/// Parses a `range literal` as defined in grammar rule `66`: its endpoints are literals, so it
/// has no names in scope.
pub fn parse_range_literal(input: &str) -> Result<AstNode> {
  parse(&FeelScope::default(), TokenType::StartRangeLiteral, input)
}

/// Parses a `context` as defined in grammar rule `59` (a boxed expression: a list or a function
/// definition is taken too).
pub fn parse_context(scope: &FeelScope, input: &str) -> Result<AstNode> {
  parse(scope, TokenType::StartBoxedExpression, input)
}

/// Parses a `name` as defined grammar rule `25`: the text of a name that is being declared (an
/// output of a decision table, an iteration variable of a boxed expression, an element of a
/// model). It is one name with the keywords and symbols it has (`Approved/Declined`, `Profit
/// and loss`), whether a name in scope starts it or not; in an expression such a name is read
/// as one name only when it is in scope.
pub fn parse_name(scope: &FeelScope, input: &str) -> Result<Name> {
  let frames = scope.frames();
  let parsing_scope = ParsingScope::new(frames.iter().map(|frame| &**frame));
  let parsed = Parser::new(&parsing_scope, TokenType::StartTextualExpression, input).with_whole_names().parse();
  if let AstNode::Name(name) = &mut parsed? {
    Ok(std::mem::take(name))
  } else {
    Err(err_not_a_feel_name(input))
  }
}

/// Parses the longest valid `name` as defined in grammar rule `25`.
pub fn parse_longest_name(input: &str) -> Result<Name> {
  parse_name(&Default::default(), input)
}

pub use format::{DEFAULT_WIDTH, Formatted};
pub use parser::{Diagnostic, MAX_DIAGNOSTICS, StartRule, Syntax, SyntaxNode, SyntaxToken, TextEnd, TokenClass};

/// Parses an `expression` (or `unary tests` with `unary_tests`) for an editor: never fails, and
/// returns every token, the nodes with their places and every error.
pub fn parse_tolerant(scope: &FeelScope, input: &str, unary_tests: bool) -> Syntax {
  parse_tolerant_start(scope, input, if unary_tests { StartRule::UnaryTests } else { StartRule::Expression })
}

/// [parse_tolerant] of the start rule `rule`.
pub fn parse_tolerant_start(scope: &FeelScope, input: &str, rule: StartRule) -> Syntax {
  let frames = scope.frames();
  let parsing_scope = ParsingScope::new(frames.iter().map(|frame| &**frame));
  Parser::new(&parsing_scope, rule.token(), input).with_rule(rule).parse_tolerant()
}

/// Formats an `expression` (or `unary tests` with `unary_tests`) in the one layout of the
/// formatter: lines of at most `width` characters where the text can be broken, nested parts
/// indented. A text with syntax errors is not formatted; the formatted text parses to the same
/// tree as the input.
pub fn format(scope: &FeelScope, input: &str, unary_tests: bool, width: usize) -> Formatted {
  format_start(scope, input, if unary_tests { StartRule::UnaryTests } else { StartRule::Expression }, width)
}

/// [format] of the start rule `rule`.
pub fn format_start(scope: &FeelScope, input: &str, rule: StartRule, width: usize) -> Formatted {
  let frames = scope.frames();
  format::format(&|| ParsingScope::new(frames.iter().map(|frame| &**frame)), input, rule, width)
}
