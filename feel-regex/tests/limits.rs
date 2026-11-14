//! A regex engine that gives up on a valid pattern reports which limit it hit; an invalid pattern
//! stays an invalid pattern, with its text unchanged.

use dsntk_common::DsntkError;
use rheon_feel_regex::{Limit, RegexError, is_match, is_match_with_flags, replace, split};

fn nested(levels: usize) -> String {
  format!("{}a{}", "(".repeat(levels), ")".repeat(levels))
}

fn invalid(pattern: &str) -> RegexError {
  RegexError::Invalid(DsntkError::new("FeelRegexError", &format!("invalid regex pattern: '{pattern}'")))
}

#[test]
fn matches_reports_the_engine_limits() {
  let input = format!("{}!", "a".repeat(40));
  assert_eq!(Err(RegexError::LimitExceeded(Limit::Backtracking)), is_match(&input, r"^(a|a)*\1$"));
  assert_eq!(Err(RegexError::LimitExceeded(Limit::Backtracking)), is_match_with_flags(&input, r"^(a|a)*\1$", "i"));
  assert_eq!(Err(RegexError::LimitExceeded(Limit::CompiledSize)), is_match("a", r"\w{10000}"));
  assert_eq!(Err(RegexError::LimitExceeded(Limit::Nesting)), is_match("a", &nested(100)));
  // 60 levels are within the limit of the backtracking engine's parser
  assert_eq!(Ok(true), is_match("a", &nested(60)));
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
  assert_eq!(Err(RegexError::LimitExceeded(Limit::CompiledSize)), replace("a", r"\w{10000}", "b", None));
  assert_eq!(Err(RegexError::LimitExceeded(Limit::Nesting)), replace("a", &nested(300), "b", None));
  // with flags, a limit does not fall back to the pattern without them
  assert_eq!(Err(RegexError::LimitExceeded(Limit::CompiledSize)), replace("a", r"\w{10000}", "b", Some("i")));
  // also when only the pattern with its flags exceeds the limit: case folding gives `k` the
  // Kelvin sign as a third form
  assert_eq!(Err(RegexError::LimitExceeded(Limit::CompiledSize)), replace("x", "k{100000}", "y", Some("i")));
  assert_eq!(Ok("x".to_string()), replace("x", "k{100000}", "y", None));
}

#[test]
fn a_pattern_is_invalid_only_when_the_engine_reads_it_within_its_limits() {
  let pattern = format!("{}[", "(".repeat(300));
  // the backtracking engine's parser stops at its nesting limit, before the unclosed class
  assert_eq!(Err(RegexError::LimitExceeded(Limit::Nesting)), is_match("a", &pattern));
  // the automata engine's parser reads the whole pattern before it checks the nesting
  assert_eq!(Err(invalid(&pattern)), split("a", &pattern));
  assert_eq!(Err(invalid(&pattern)), replace("a", &pattern, "b", None));
}

#[test]
fn invalid_patterns_stay_invalid() {
  for pattern in ["[a-z", r"(asd)[\1]"] {
    assert_eq!(Err(invalid(pattern)), split("a", pattern));
    assert_eq!(Err(invalid(pattern)), replace("a", pattern, "b", None));
  }
  for pattern in [r"(.)\3", "[a-z", r"(asd)[\1]", r"\p{ IsBasicLatin}+"] {
    assert_eq!(Err(invalid(pattern)), is_match("a", pattern));
  }
  assert_eq!(Err(invalid("(?i)[a-z")), is_match_with_flags("a", "[a-z", "i"));
}

#[test]
fn replace_and_split_answer_as_before() {
  assert_eq!(Ok("x-y-z".to_string()), replace("x y z", " ", "-", None));
  assert_eq!(Ok("[b][a]".to_string()), replace("ab", "(a)(b)", "[$2][$1]", None));
  assert_eq!(Ok("XbX".to_string()), replace("AbA", "a", "X", Some("i")));
  // `q` quotes the pattern, unless a flag other than `i` comes with it
  assert_eq!(Ok("a-b".to_string()), replace("a.b", ".", "-", Some("q")));
  // letters other than the flags are ignored
  assert_eq!(Ok("b".to_string()), replace("a", "a", "b", Some("?")));
  assert_eq!(Ok(vec!["a".to_string(), "b".to_string(), "c".to_string()]), split("a,b;c", "[,;]"));
}
