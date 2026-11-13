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
mod context;
mod errors;
mod index;
mod lalr;
mod lexer;
mod parser;
mod scope;

#[cfg(test)]
mod tests;

pub use ast::AstNode;
pub use closure::ClosureBuilder;
pub use scope::ParsingScope;

use crate::errors::*;
use crate::lalr::TokenType;
use crate::parser::Parser;
use dsntk_common::Result;
use rheon_feel::{FeelScope, Name};

/// Parses the input in a parsing scope over the contexts of `scope` (borrowed, not copied;
/// nothing changes `scope` while parsing).
fn parse(scope: &FeelScope, start_token_type: TokenType, input: &str, trace: bool) -> Result<AstNode> {
  let frames = scope.frames();
  let parsing_scope = ParsingScope::new(&frames);
  Parser::new(&parsing_scope, start_token_type, input, trace).parse()
}

/// Parses an `expression` as defined in grammar rule `1`.
pub fn parse_expression(scope: &FeelScope, input: &str, trace: bool) -> Result<AstNode> {
  parse(scope, TokenType::StartExpression, input, trace)
}

/// Parses a `textual expression` as defined in grammar rule `2`.
pub fn parse_textual_expression(scope: &FeelScope, input: &str, trace: bool) -> Result<AstNode> {
  parse(scope, TokenType::StartTextualExpression, input, trace)
}

/// Parses `textual expressions` as defined in grammar rule `3`.
pub fn parse_textual_expressions(scope: &FeelScope, input: &str, trace: bool) -> Result<AstNode> {
  parse(scope, TokenType::StartTextualExpressions, input, trace)
}

/// Parses a `simple expression` as defined in grammar rule `5`.
pub fn parse_simple_expression(scope: &FeelScope, input: &str, trace: bool) -> Result<AstNode> {
  parse(scope, TokenType::StartSimpleExpression, input, trace)
}

/// Parses `simple expressions` as defined in grammar rule `6`.
pub fn parse_simple_expressions(scope: &FeelScope, input: &str, trace: bool) -> Result<AstNode> {
  parse(scope, TokenType::StartSimpleExpressions, input, trace)
}

/// Parses `unary tests` as defined in grammar rule `15`.
pub fn parse_unary_tests(scope: &FeelScope, input: &str, trace: bool) -> Result<AstNode> {
  parse(scope, TokenType::StartUnaryTests, input, trace)
}

/// Parses a `simple value` as defined in grammar rule `17`.
pub fn parse_simple_value(scope: &FeelScope, input: &str, trace: bool) -> Result<AstNode> {
  parse(scope, TokenType::StartSimpleValue, input, trace)
}

/// Parses a `name` as defined grammar rule `25`.
pub fn parse_name(scope: &FeelScope, input: &str, trace: bool) -> Result<Name> {
  if let AstNode::Name(name) = &parse(scope, TokenType::StartTextualExpression, input, trace)? {
    Ok(name.clone())
  } else {
    Err(err_not_a_feel_name(input))
  }
}

/// Parses the longest valid `name` as defined in grammar rule `25`.
pub fn parse_longest_name(input: &str) -> Result<Name> {
  parse_name(&Default::default(), input, false)
}

/// Parses a `boxed expression` as defined in grammar rule `53`.
pub fn parse_boxed_expression(scope: &FeelScope, input: &str, trace: bool) -> Result<AstNode> {
  parse(scope, TokenType::StartBoxedExpression, input, trace)
}

/// Parses a `context` as defined in grammar rule `59`.
pub fn parse_context(scope: &FeelScope, input: &str, trace: bool) -> Result<AstNode> {
  parse(scope, TokenType::StartBoxedExpression, input, trace)
}

/// Parses a `range literal` as defined in grammar rule `66`.
pub fn parse_range_literal(scope: &FeelScope, input: &str, trace: bool) -> Result<AstNode> {
  parse(scope, TokenType::StartRangeLiteral, input, trace)
}
