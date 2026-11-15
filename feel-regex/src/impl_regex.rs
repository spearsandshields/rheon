use crate::errors::{Limit, RegexError, err_invalid_pattern};
use crate::flags::Flags;
use crate::replacement::Replacement;
use crate::unicode_blocks::replace_block_names;
use fancy_regex::Regex;
use regex_syntax::ast;

/// Whether `input` matches `pattern` with `flags` (XQuery 1.0 and XPath 2.0 F&O §7.6.2
/// `fn:matches`, back-references included); `""` is no flags.
pub fn is_match(input: &str, pattern: &str, flags: &str) -> Result<bool, RegexError> {
  let source = Flags::parse(flags)?.source(pattern, translate);
  search(input, &source, pattern)
}

/// Whether `input` matches the backtracking engine's `source` (back-references and look-around
/// included), or which limit the engine hit, or that the pattern is invalid (quoting `quoted`, the
/// pattern as the caller wrote it).
fn search(input: &str, source: &str, quoted: &str) -> Result<bool, RegexError> {
  let classify = |error: fancy_regex::Error| match error {
    fancy_regex::Error::ParseError(_, fancy_regex::ParseError::RecursionExceeded) => RegexError::LimitExceeded(Limit::Nesting),
    fancy_regex::Error::CompileError(error) => match *error {
      fancy_regex::CompileError::InnerError(error) if error.size_limit().is_some() => RegexError::LimitExceeded(Limit::CompiledSize),
      fancy_regex::CompileError::InnerError(error) if error.syntax_error().is_some_and(is_nest_limit) => RegexError::LimitExceeded(Limit::Nesting),
      _ => RegexError::InvalidPattern(err_invalid_pattern(quoted)),
    },
    fancy_regex::Error::RuntimeError(fancy_regex::RuntimeError::StackOverflow) => RegexError::LimitExceeded(Limit::BacktrackingStack),
    // a search that fails at run time always ran out of its backtracking budget
    fancy_regex::Error::RuntimeError(_) => RegexError::LimitExceeded(Limit::Backtracking),
    _ => RegexError::InvalidPattern(err_invalid_pattern(quoted)),
  };
  Regex::new(source).map_err(classify)?.is_match(&fix_input(input)).map_err(classify)
}

/// Whether a pattern nests groups deeper than the parser's limit.
fn is_nest_limit(error: &regex_syntax::Error) -> bool {
  matches!(error, regex_syntax::Error::Parse(error) if matches!(error.kind(), ast::ErrorKind::NestLimitExceeded(_)))
}

/// The engine source `source` compiled by the automata engine (no back-references), or which
/// limit it hit, or that the pattern is invalid (quoting `quoted`, the pattern as the caller wrote
/// it).
fn compile(source: &str, quoted: &str) -> Result<regex::Regex, RegexError> {
  regex::Regex::new(source).map_err(|error| match error {
    regex::Error::CompiledTooBig(_) => RegexError::LimitExceeded(Limit::CompiledSize),
    // the error names no kind; the parser with the engine's defaults tells a nesting limit apart,
    // and it runs only on this error path
    regex::Error::Syntax(_) => match ast::parse::Parser::new().parse(source) {
      Err(error) if matches!(error.kind(), ast::ErrorKind::NestLimitExceeded(_)) => RegexError::LimitExceeded(Limit::Nesting),
      _ => RegexError::InvalidPattern(err_invalid_pattern(quoted)),
    },
    _ => RegexError::InvalidPattern(err_invalid_pattern(quoted)),
  })
}

/// Replaces every match of `pattern` with `flags` in `input` with `replacement` (F&O §7.6.3
/// `fn:replace`); `""` is no flags.
pub fn replace(input: &str, pattern: &str, replacement: &str, flags: &str) -> Result<String, RegexError> {
  let flags = Flags::parse(flags)?;
  let regex = compile(&flags.source(pattern, str::to_string), pattern)?;
  let replacement = Replacement::parse(replacement, regex.captures_len() - 1, flags.literal())?;
  Ok(regex.replace_all(input, &replacement).into_owned())
}

/// Splits `input` at every match of `delimiter`.
pub fn split(input: &str, delimiter: &str) -> Result<Vec<String>, RegexError> {
  Ok(compile(delimiter, delimiter)?.split(input).map(String::from).collect())
}

// Nasty tricks on input.
fn fix_input(s: &str) -> String {
  s.replace("\r\n", "\n").replace('\r', "\n")
}

/// A pattern in the syntax of the backtracking engine: the XSD block names (`\p{IsBasicLatin}`)
/// become their ranges, and a class subtraction (`[a-z-[aeiou]]`) becomes the engine's `--[`.
fn translate(pattern: &str) -> String {
  replace_block_names(pattern).replace("-[", "--[")
}
