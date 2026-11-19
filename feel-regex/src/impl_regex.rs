use crate::errors::{Limit, RegexError, err_invalid_pattern, err_zero_length_match};
use crate::flags::Flags;
use crate::replacement::Replacement;
use crate::unicode_blocks::replace_block_names;
use fancy_regex::Regex;
use regex_syntax::ast;

/// At most this many compiled patterns of each kind are kept in a [Patterns].
const MAX_KEPT: usize = 8;

/// Compiled patterns, kept by a caller that matches the same patterns again and again (the
/// rows of a list): a pattern in it is compiled once. It holds at most [MAX_KEPT] patterns of
/// `matches` and as many of `replace` and `split`, each within the size limits of its engine;
/// a further pattern is compiled for its use and not kept. It lives as long as its owner.
#[derive(Default)]
pub struct Patterns {
  /// By engine source, the patterns of `matches`.
  searches: Vec<(String, Regex)>,
  /// By engine source, the patterns of `replace` and `split`.
  others: Vec<(String, regex::Regex)>,
}

/// Whether `input` matches `pattern` with `flags` (XQuery 1.0 and XPath 2.0 F&O §7.6.2
/// `fn:matches`, back-references included); `""` is no flags.
pub fn is_match(input: &str, pattern: &str, flags: &str) -> Result<bool, RegexError> {
  is_match_with(&mut Patterns::default(), input, pattern, flags)
}

/// [is_match] with the compiled patterns of `patterns`.
pub fn is_match_with(patterns: &mut Patterns, input: &str, pattern: &str, flags: &str) -> Result<bool, RegexError> {
  let flags = Flags::parse(flags)?;
  if flags.plain(pattern) {
    return Ok(input.contains(pattern));
  }
  search(patterns, input, &flags.source(pattern, translate), pattern)
}

/// Whether `input` matches the backtracking engine's `source` (back-references and look-around
/// included), or which limit the engine hit, or that the pattern is invalid (quoting `quoted`, the
/// pattern as the caller wrote it).
fn search(patterns: &mut Patterns, input: &str, source: &str, quoted: &str) -> Result<bool, RegexError> {
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
  let input = fix_input(input);
  if let Some((_, regex)) = patterns.searches.iter().find(|(kept, _)| kept == source) {
    return regex.is_match(&input).map_err(classify);
  }
  let regex = Regex::new(source).map_err(classify)?;
  let answer = regex.is_match(&input).map_err(classify);
  if patterns.searches.len() < MAX_KEPT {
    patterns.searches.push((source.to_string(), regex));
  }
  answer
}

/// Whether a pattern nests groups deeper than the parser's limit.
fn is_nest_limit(error: &regex_syntax::Error) -> bool {
  matches!(error, regex_syntax::Error::Parse(error) if matches!(error.kind(), ast::ErrorKind::NestLimitExceeded(_)))
}

/// The engine source `source` of a `replace` or `split` pattern compiled by the automata engine
/// (no back-references), or which limit it hit, or that the pattern is invalid (quoting `quoted`,
/// the pattern as the caller wrote it), or that it matches the zero-length string. F&O §7.6.3 and
/// §7.6.4 raise FORX0003 "if the pattern matches a zero-length string, that is, if the expression
/// fn:matches("", $pattern, $flags) returns true", whatever the input.
fn compile(patterns: &mut Patterns, source: &str, quoted: &str) -> Result<regex::Regex, RegexError> {
  if let Some((_, regex)) = patterns.others.iter().find(|(kept, _)| kept == source) {
    return Ok(regex.clone());
  }
  let regex = regex::Regex::new(source).map_err(|error| match error {
    regex::Error::CompiledTooBig(_) => RegexError::LimitExceeded(Limit::CompiledSize),
    // the error names no kind; the parser with the engine's defaults tells a nesting limit apart,
    // and it runs only on this error path
    regex::Error::Syntax(_) => match ast::parse::Parser::new().parse(source) {
      Err(error) if matches!(error.kind(), ast::ErrorKind::NestLimitExceeded(_)) => RegexError::LimitExceeded(Limit::Nesting),
      _ => RegexError::InvalidPattern(err_invalid_pattern(quoted)),
    },
    _ => RegexError::InvalidPattern(err_invalid_pattern(quoted)),
  })?;
  if regex.is_match("") {
    return Err(RegexError::InvalidArgument(err_zero_length_match()));
  }
  if patterns.others.len() < MAX_KEPT {
    // a compiled pattern is shared by its copies
    patterns.others.push((source.to_string(), regex.clone()));
  }
  Ok(regex)
}

/// Replaces every match of `pattern` with `flags` in `input` with `replacement` (F&O §7.6.3
/// `fn:replace`); `""` is no flags.
pub fn replace(input: &str, pattern: &str, replacement: &str, flags: &str) -> Result<String, RegexError> {
  replace_with(&mut Patterns::default(), input, pattern, replacement, flags)
}

/// [replace] with the compiled patterns of `patterns`.
pub fn replace_with(patterns: &mut Patterns, input: &str, pattern: &str, replacement: &str, flags: &str) -> Result<String, RegexError> {
  let flags = Flags::parse(flags)?;
  let regex = compile(patterns, &flags.source(pattern, str::to_string), pattern)?;
  let replacement = Replacement::parse(replacement, regex.captures_len() - 1, flags.literal())?;
  Ok(regex.replace_all(input, &replacement).into_owned())
}

/// Splits `input` at every match of `delimiter` (F&O §7.6.4 `fn:tokenize` without flags). "If
/// $input is the empty sequence, or if $input is the zero-length string, the result is the empty
/// sequence"; otherwise a separator at the start or the end of the input, and two adjacent
/// separators, give a zero-length string.
pub fn split(input: &str, delimiter: &str) -> Result<Vec<String>, RegexError> {
  split_with(&mut Patterns::default(), input, delimiter)
}

/// [split] with the compiled patterns of `patterns`.
pub fn split_with(patterns: &mut Patterns, input: &str, delimiter: &str) -> Result<Vec<String>, RegexError> {
  let regex = compile(patterns, delimiter, delimiter)?;
  if input.is_empty() {
    return Ok(Vec::new());
  }
  Ok(regex.split(input).map(String::from).collect())
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

#[cfg(test)]
mod tests {
  use super::*;

  /// [is_match] by the engine, whatever the pattern.
  fn by_engine(input: &str, pattern: &str, flags: &str) -> Result<bool, RegexError> {
    let source = Flags::parse(flags)?.source(pattern, translate);
    search(&mut Patterns::default(), input, &source, pattern)
  }

  #[test]
  fn a_plain_pattern_is_matched_as_the_engine_matches_it() {
    let patterns = [
      "a",
      "open",
      "Order 66",
      "a_b",
      "x@y",
      "1,2",
      "k: v",
      "a;b",
      "a=b",
      "wow!",
      "it's",
      "say \"hi\"",
      "a/b",
      "50%",
      "#1",
      "żółć",
      "東京",
      " ",
    ];
    let inputs = [
      "",
      "a",
      "open",
      "reopened",
      "Order 66 placed",
      "a_b",
      "mail x@y.z",
      "1,2,3",
      "k: v",
      "a;b",
      "a=b",
      "wow!",
      "it's",
      "say \"hi\"",
      "a/b/c",
      "50%",
      "#1",
      "zażółć",
      "東京都",
      "a b",
      "line\r\nopen",
      "o\rpen",
      "o\npen",
    ];
    for pattern in patterns {
      for flags in ["", "q"] {
        assert!(Flags::parse(flags).unwrap().plain(pattern), "{pattern:?} {flags:?}");
        for input in inputs {
          assert_eq!(by_engine(input, pattern, flags), is_match(input, pattern, flags), "{input:?} {pattern:?} {flags:?}");
        }
      }
    }
    // with `q` every character stands for itself
    for pattern in ["a.b", "1+1", "[x]", "a|b", "\\d", "^a$", "(", "a-b", "<b>"] {
      assert!(Flags::parse("q").unwrap().plain(pattern), "{pattern:?}");
      for input in ["a.b", "axb", "1+1", "11", "[x]", "x", "a|b", "a", "\\d", "5", "^a$", "(", "a-b", "<b>"] {
        assert_eq!(by_engine(input, pattern, "q"), is_match(input, pattern, "q"), "{input:?} {pattern:?}");
      }
    }
  }

  #[test]
  fn another_pattern_goes_to_the_engine() {
    for (pattern, flags) in [
      ("", ""),
      ("a.b", ""),
      ("a-b", ""),
      ("a&b", ""),
      ("<b>", ""),
      ("a~b", ""),
      ("a\nb", "q"),
      ("a\rb", ""),
      ("a", "i"),
      ("a", "x"),
      ("a", "m"),
      ("a", "s"),
      ("a", "qi"),
      ("a b", "x"),
    ] {
      assert!(!Flags::parse(flags).unwrap().plain(pattern), "{pattern:?} {flags:?}");
    }
  }

  #[test]
  fn a_pattern_is_compiled_once_and_at_most_eight_are_kept() {
    let mut patterns = Patterns::default();
    for _ in 0..3 {
      assert_eq!(Ok(true), is_match_with(&mut patterns, "abc", "^a.c$", ""));
      assert_eq!(Ok("a-c".to_string()), replace_with(&mut patterns, "abc", "b+", "-", ""));
      assert_eq!(Ok(vec!["a".to_string(), "c".to_string()]), split_with(&mut patterns, "abc", "b+"));
    }
    // `replace` and `split` share the pattern they both use
    assert_eq!((1, 1), (patterns.searches.len(), patterns.others.len()));
    for n in 0..20 {
      assert_eq!(Ok(true), is_match_with(&mut patterns, &format!("x{n}"), &format!("^x{n}$"), ""));
      assert_eq!(Ok(String::new()), replace_with(&mut patterns, &format!("x{n}"), &format!("x{n}+"), "", ""));
    }
    assert_eq!((MAX_KEPT, MAX_KEPT), (patterns.searches.len(), patterns.others.len()));
    // an invalid pattern and a plain one are not kept
    let mut patterns = Patterns::default();
    assert!(is_match_with(&mut patterns, "a", "(", "").is_err());
    assert_eq!(Ok(true), is_match_with(&mut patterns, "abc", "b", ""));
    assert!(replace_with(&mut patterns, "a", "b*", "", "").is_err());
    assert_eq!((0, 0), (patterns.searches.len(), patterns.others.len()));
  }
}
