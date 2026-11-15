use dsntk_common::DsntkError;
use rheon_feel_regex::{RegexError, is_match, replace};

#[test]
fn _0001() {
  assert!(is_match("Mr. B. Obama", "B. Obama", "iq").unwrap());
}

#[test]
fn _0002() {
  assert!(!is_match("Mr. B, Obama", "B. OBAMA", "iq").unwrap());
}

#[test]
fn _0003() {
  assert!(!is_match("abcd", ".*", "q").unwrap());
}

#[test]
fn _0004() {
  assert!(is_match("hello world", r#"\p{ IsBasicLatin}+"#, "x").unwrap());
}

#[test]
fn _0005() {
  assert!(is_match("hello world", r#" hello[ ]world"#, "x").unwrap());
}

#[test]
fn _0006() {
  assert!(!is_match("i", r#"[A-Z]-[OI]"#, "i").unwrap());
}

#[test]
fn back_reference_with_caseless_flag() {
  // TCK 1111 K2-MatchesFunc-17
  assert!(is_match("aA", r"(a)\1", "i").unwrap());
}

#[test]
fn a_character_that_is_no_flag_is_an_invalid_argument() {
  let reason = |flag: char| DsntkError::new("FeelRegexError", &format!("invalid regex flag: '{flag}'"));
  assert_eq!(Err(RegexError::InvalidArgument(reason(' '))), is_match("a", "a", " i"));
  assert_eq!(Err(RegexError::InvalidArgument(reason('?'))), replace("a", "a", "b", "?"));
  // the flags are read before the pattern
  assert_eq!(Err(RegexError::InvalidArgument(reason('I'))), is_match("a", "[", "I"));
}
