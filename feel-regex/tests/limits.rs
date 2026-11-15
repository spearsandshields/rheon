//! A regex engine that gives up on a valid pattern reports which limit it hit; an invalid pattern
//! stays an invalid pattern, with its text unchanged.

use dsntk_common::DsntkError;
use rheon_feel_regex::{Limit, RegexError, is_match, replace, split};

fn nested(levels: usize) -> String {
  format!("{}a{}", "(".repeat(levels), ")".repeat(levels))
}

fn invalid(pattern: &str) -> RegexError {
  RegexError::InvalidPattern(DsntkError::new("FeelRegexError", &format!("invalid regex pattern: '{pattern}'")))
}

#[test]
fn matches_reports_the_engine_limits() {
  let input = format!("{}!", "a".repeat(40));
  assert_eq!(Err(RegexError::LimitExceeded(Limit::Backtracking)), is_match(&input, r"^(a|a)*\1$", ""));
  assert_eq!(Err(RegexError::LimitExceeded(Limit::Backtracking)), is_match(&input, r"^(a|a)*\1$", "i"));
  assert_eq!(Err(RegexError::LimitExceeded(Limit::CompiledSize)), is_match("a", r"\w{10000}", ""));
  assert_eq!(Err(RegexError::LimitExceeded(Limit::Nesting)), is_match("a", &nested(100), ""));
  // 60 levels are within the limit of the backtracking engine's parser
  assert_eq!(Ok(true), is_match("a", &nested(60), ""));
}

#[test]
fn split_reports_the_engine_limits() {
  assert_eq!(Err(RegexError::LimitExceeded(Limit::CompiledSize)), split("a", r"\w{10000}"));
  assert_eq!(Err(RegexError::LimitExceeded(Limit::Nesting)), split("a", &nested(300)));
  // 100 levels are within the limit of split's engine
  assert_eq!(Ok(vec!["".to_string(), "".to_string()]), split("a", &nested(100)));
}

#[test]
fn replace_reports_the_engine_limits() {
  assert_eq!(Err(RegexError::LimitExceeded(Limit::CompiledSize)), replace("a", r"\w{10000}", "b", ""));
  assert_eq!(Err(RegexError::LimitExceeded(Limit::Nesting)), replace("a", &nested(300), "b", ""));
  // with flags, a limit does not fall back to the pattern without them
  assert_eq!(Err(RegexError::LimitExceeded(Limit::CompiledSize)), replace("a", r"\w{10000}", "b", "i"));
  // also when only the pattern with its flags exceeds the limit: case folding gives `k` the
  // Kelvin sign as a third form
  assert_eq!(Err(RegexError::LimitExceeded(Limit::CompiledSize)), replace("x", "k{100000}", "y", "i"));
  assert_eq!(Ok("x".to_string()), replace("x", "k{100000}", "y", ""));
}

#[test]
fn a_pattern_is_invalid_only_when_the_engine_reads_it_within_its_limits() {
  let pattern = format!("{}[", "(".repeat(300));
  // the backtracking engine's parser stops at its nesting limit, before the unclosed class
  assert_eq!(Err(RegexError::LimitExceeded(Limit::Nesting)), is_match("a", &pattern, ""));
  // the automata engine's parser reads the whole pattern before it checks the nesting
  assert_eq!(Err(invalid(&pattern)), split("a", &pattern));
  assert_eq!(Err(invalid(&pattern)), replace("a", &pattern, "b", ""));
}

#[test]
fn invalid_patterns_stay_invalid() {
  for pattern in ["[a-z", r"(asd)[\1]"] {
    assert_eq!(Err(invalid(pattern)), split("a", pattern));
    assert_eq!(Err(invalid(pattern)), replace("a", pattern, "b", ""));
  }
  for pattern in [r"(.)\3", "[a-z", r"(asd)[\1]", r"\p{ IsBasicLatin}+"] {
    assert_eq!(Err(invalid(pattern)), is_match("a", pattern, ""));
  }
  // the pattern is quoted as the caller wrote it, also with flags
  assert_eq!(Err(invalid("[a-z")), is_match("a", "[a-z", "i"));
  assert_eq!(Err(invalid("[a-z")), replace("a", "[a-z", "b", "i"));
}

#[test]
fn replace_and_split_answer() {
  assert_eq!(Ok("x-y-z".to_string()), replace("x y z", " ", "-", ""));
  assert_eq!(Ok("[b][a]".to_string()), replace("ab", "(a)(b)", "[$2][$1]", ""));
  assert_eq!(Ok("XbX".to_string()), replace("AbA", "a", "X", "i"));
  assert_eq!(Ok("a-b".to_string()), replace("a.b", ".", "-", "q"));
  assert_eq!(Ok(vec!["a".to_string(), "b".to_string(), "c".to_string()]), split("a,b;c", "[,;]"));
}

#[test]
fn replace_and_split_reject_a_pattern_that_matches_the_zero_length_string() {
  let zero_length = || RegexError::InvalidArgument(DsntkError::new("FeelRegexError", "the pattern matches the zero-length string"));
  assert_eq!(Err(zero_length()), replace("abc", "x*", "-", ""));
  assert_eq!(Err(zero_length()), replace("abc", "x *", "-", "x"));
  assert_eq!(Err(zero_length()), split("abc", ""));
  assert_eq!(Err(zero_length()), split("", "a?"));
  // a limit or an invalid pattern comes first
  assert_eq!(Err(RegexError::LimitExceeded(Limit::CompiledSize)), split("", r"\w{10000}"));
  assert_eq!(Err(invalid("(|")), split("", "(|"));
  // a zero-length input has no item
  assert_eq!(Ok(Vec::<String>::new()), split("", ";"));
  assert_eq!(Ok(vec!["".to_string(), "".to_string()]), split(";", ";"));
}
