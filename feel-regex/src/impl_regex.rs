use crate::errors::{Limit, RegexError, err_invalid_pattern_at, err_invalid_pattern_expecting, err_zero_length_match};
use crate::flags::Flags;
use crate::replacement::Replacement;
use crate::xsd::{Level, translate};
use fancy_regex::Regex;
use regex_syntax::ast;

/// At most this many compiled patterns are kept in a [Patterns].
const MAX_KEPT: usize = 16;

/// Compiled patterns, kept by a caller that matches the same patterns again and again (the
/// rows of a list): a pattern in it is read and compiled once. It holds at most 16
/// patterns, each within the size limits of its engine; a further pattern is compiled for its use
/// and not kept. It lives as long as its owner.
#[derive(Default)]
pub struct Patterns {
  kept: Vec<Kept>,
}

/// A pattern as the caller wrote it, with its flags, and what was made of it.
struct Kept {
  pattern: String,
  flags: String,
  regex: Regex,
  /// Whether it matches the zero-length string, once a `replace` or a `split` asked.
  empty: Option<bool>,
}

/// Whether `input` matches `pattern` with `flags` (XQuery 1.0 and XPath 2.0 F&O §7.6.2
/// `fn:matches`, back-references included); `""` is no flags.
pub fn is_match(input: &str, pattern: &str, flags: &str) -> Result<bool, RegexError> {
  is_match_with(&mut Patterns::default(), input, pattern, flags)
}

/// [is_match] with the compiled patterns of `patterns`.
pub fn is_match_with(patterns: &mut Patterns, input: &str, pattern: &str, flags_text: &str) -> Result<bool, RegexError> {
  let flags = Flags::parse(flags_text)?;
  if flags.plain(pattern) {
    return Ok(input.contains(pattern));
  }
  let compiled = compile(patterns, pattern, flags_text, &flags)?;
  compiled.regex(patterns).is_match(input).map_err(runtime)
}

/// The engine's source of `pattern` with `flags`: read by the XSD front end ([translate]), or
/// quoted with `q`.
fn source(pattern: &str, flags: &Flags) -> Result<String, RegexError> {
  flags.try_source(pattern, |read| {
    translate(read, flags.dot_all(), Level::Fo20).map(|translated| translated.source).map_err(|refused| {
      if refused.nesting {
        RegexError::LimitExceeded(Limit::Nesting)
      } else if read.len() == pattern.len() {
        RegexError::InvalidPattern(err_invalid_pattern_at(pattern, refused.at, refused.expected))
      } else {
        // with `x` the place is one in the pattern without its whitespace: the pattern as the
        // caller wrote it is quoted, with what was expected
        RegexError::InvalidPattern(err_invalid_pattern_expecting(pattern, refused.expected))
      }
    })
  })
}

/// Which limit a search hit at run time.
fn runtime(error: fancy_regex::Error) -> RegexError {
  match error {
    fancy_regex::Error::RuntimeError(fancy_regex::RuntimeError::StackOverflow) => RegexError::LimitExceeded(Limit::BacktrackingStack),
    // a search that fails at run time always ran out of its backtracking budget
    _ => RegexError::LimitExceeded(Limit::Backtracking),
  }
}

/// Whether a pattern nests groups deeper than the parser's limit.
fn is_nest_limit(error: &regex_syntax::Error) -> bool {
  matches!(error, regex_syntax::Error::Parse(error) if matches!(error.kind(), ast::ErrorKind::NestLimitExceeded(_)))
}

/// The compiled `pattern` with `flags` (written `flags_text`), kept or read by the front end and
/// compiled, and where it is kept; or which limit the engine hit, or that the pattern is invalid.
/// The patterns kept are found by the text of the pattern and its flags, so a kept pattern is
/// neither read nor translated again.
fn compile(patterns: &mut Patterns, pattern: &str, flags_text: &str, flags: &Flags) -> Result<Compiled, RegexError> {
  if let Some(at) = patterns.kept.iter().position(|kept| kept.pattern == pattern && kept.flags == flags_text) {
    return Ok(Compiled::Kept(at));
  }
  let source = source(pattern, flags)?;
  let regex = Regex::new(&source).map_err(|error| match error {
    fancy_regex::Error::ParseError(_, fancy_regex::ParseError::RecursionExceeded) => RegexError::LimitExceeded(Limit::Nesting),
    fancy_regex::Error::CompileError(error) => match *error {
      fancy_regex::CompileError::InnerError(error) if error.size_limit().is_some() => RegexError::LimitExceeded(Limit::CompiledSize),
      fancy_regex::CompileError::InnerError(error) if error.syntax_error().is_some_and(is_nest_limit) => RegexError::LimitExceeded(Limit::Nesting),
      _ => RegexError::InvalidPattern(err_invalid_pattern_at(pattern, 0, "a pattern the engine can compile")),
    },
    _ => RegexError::InvalidPattern(err_invalid_pattern_at(pattern, 0, "a pattern the engine can compile")),
  })?;
  if patterns.kept.len() < MAX_KEPT {
    patterns.kept.push(Kept {
      pattern: pattern.to_string(),
      flags: flags_text.to_string(),
      regex,
      empty: None,
    });
    return Ok(Compiled::Kept(patterns.kept.len() - 1));
  }
  Ok(Compiled::Owned(regex))
}

/// A compiled pattern: kept in the [Patterns] at an index, or made for one use. A kept regex is
/// used where it is kept, never copied: a copy of a regex starts with no search cache of its own,
/// and builds one at its first search.
enum Compiled {
  Kept(usize),
  Owned(Regex),
}

impl Compiled {
  fn regex<'p>(&'p self, patterns: &'p Patterns) -> &'p Regex {
    match self {
      Compiled::Kept(at) => &patterns.kept[*at].regex,
      Compiled::Owned(regex) => regex,
    }
  }
}

/// [compile] for `replace` and `split`: F&O §7.6.3 and §7.6.4 raise FORX0003 "if the pattern
/// matches a zero-length string, that is, if the expression fn:matches("", $pattern, $flags)
/// returns true", whatever the input. A kept pattern is asked once.
fn compile_nonempty(patterns: &mut Patterns, pattern: &str, flags_text: &str, flags: &Flags) -> Result<Compiled, RegexError> {
  let compiled = compile(patterns, pattern, flags_text, flags)?;
  let empty = match &compiled {
    Compiled::Kept(at) => match patterns.kept[*at].empty {
      Some(empty) => empty,
      None => {
        let empty = patterns.kept[*at].regex.is_match("").map_err(runtime)?;
        patterns.kept[*at].empty = Some(empty);
        empty
      }
    },
    Compiled::Owned(regex) => regex.is_match("").map_err(runtime)?,
  };
  if empty {
    return Err(RegexError::InvalidArgument(err_zero_length_match()));
  }
  Ok(compiled)
}

/// Replaces every match of `pattern` with `flags` in `input` with `replacement` (F&O §7.6.3
/// `fn:replace`); `""` is no flags.
pub fn replace(input: &str, pattern: &str, replacement: &str, flags: &str) -> Result<String, RegexError> {
  replace_with(&mut Patterns::default(), input, pattern, replacement, flags)
}

/// [replace] with the compiled patterns of `patterns`.
pub fn replace_with(patterns: &mut Patterns, input: &str, pattern: &str, replacement: &str, flags_text: &str) -> Result<String, RegexError> {
  let flags = Flags::parse(flags_text)?;
  let compiled = compile_nonempty(patterns, pattern, flags_text, &flags)?;
  let regex = compiled.regex(patterns);
  let replacement = Replacement::parse(replacement, regex.captures_len() - 1, flags.literal())?;
  regex.try_replacen(input, 0, &replacement).map(|replaced| replaced.into_owned()).map_err(runtime)
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
  let compiled = compile_nonempty(patterns, delimiter, "", &Flags::default())?;
  if input.is_empty() {
    return Ok(Vec::new());
  }
  compiled.regex(patterns).split(input).map(|token| token.map(String::from).map_err(runtime)).collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  /// [is_match] by the engine, whatever the pattern.
  fn by_engine(input: &str, pattern: &str, flags: &str) -> Result<bool, RegexError> {
    let mut patterns = Patterns::default();
    let compiled = compile(&mut patterns, pattern, flags, &Flags::parse(flags)?)?;
    compiled.regex(&patterns).is_match(input).map_err(runtime)
  }

  #[test]
  fn only_a_hyphen_before_a_class_inside_a_class_is_a_subtraction() {
    let source = |pattern: &str| translate(pattern, false, Level::Fo20).map(|translated| translated.source);
    assert_eq!(Ok(r"[[a-z]--[aeiou]]".to_string()), source(r"[a-z-[aeiou]]"));
    assert_eq!(Ok(r"[[^a-z]--[aeiou]]x".to_string()), source(r"[^a-z-[aeiou]]x"));
    assert_eq!(Ok(r"[[a-z]--[[^b-d]--[c]]]".to_string()), source(r"[a-z-[^b-d-[c]]]"));
    // outside a class, `-[` is a hyphen and a class
    assert_eq!(Ok(r"[\p{Nd}]{3}\-[0-9]{4}".to_string()), source(r"\d{3}-[0-9]{4}"));
  }

  #[test]
  fn a_hyphen_before_a_class_outside_a_class_matches_a_hyphen() {
    assert!(is_match("555-1234", r"\d{3}-[0-9]{4}", "").unwrap());
    assert!(is_match("a-1", r"a\-[0-9]", "").unwrap());
    assert!(!is_match("a1", r"a-[0-9]", "").unwrap());
    assert!(is_match("b", r"^[a-z-[aeiou]]$", "").unwrap());
    assert!(!is_match("e", r"^[a-z-[aeiou]]$", "").unwrap());
    // a negated class less a class: the characters outside a-z, less the vowels
    assert!(!is_match("e", r"^[^a-z-[aeiou]]$", "").unwrap());
    assert!(!is_match("b", r"^[^a-z-[aeiou]]$", "").unwrap());
    assert!(is_match("1", r"^[^a-z-[aeiou]]$", "").unwrap());
    assert!(!is_match("1", r"^[^a-z-[0-9]]$", "").unwrap());
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
  fn a_pattern_is_compiled_once_and_at_most_sixteen_are_kept() {
    let mut patterns = Patterns::default();
    for _ in 0..3 {
      assert_eq!(Ok(true), is_match_with(&mut patterns, "abc", "^a.c$", ""));
      assert_eq!(Ok("a-c".to_string()), replace_with(&mut patterns, "abc", "b+", "-", ""));
      assert_eq!(Ok(vec!["a".to_string(), "c".to_string()]), split_with(&mut patterns, "abc", "b+"));
    }
    // `replace` and `split` share the pattern they both use
    assert_eq!(2, patterns.kept.len());
    for n in 0..20 {
      assert_eq!(Ok(true), is_match_with(&mut patterns, &format!("x{n}"), &format!("^x{n}$"), ""));
    }
    assert_eq!(MAX_KEPT, patterns.kept.len());
    // an invalid pattern and a plain one are not kept
    let mut patterns = Patterns::default();
    assert!(is_match_with(&mut patterns, "a", "(", "").is_err());
    assert_eq!(Ok(true), is_match_with(&mut patterns, "abc", "b", ""));
    assert_eq!(0, patterns.kept.len());
  }
}
