//! The patterns of the regex built-ins are read as XML Schema Part 2 Appendix F regular expressions
//! with the additions of XQuery 1.0 and XPath 2.0 Functions and Operators §7.6.1.

use rheon_feel_regex::{RegexError, is_match, replace, split};

fn matches(input: &str, pattern: &str) -> bool {
  is_match(input, pattern, "").unwrap_or_else(|error| panic!("{pattern:?}: {error:?}"))
}

fn invalid(pattern: &str) -> bool {
  matches!(is_match("a", pattern, ""), Err(RegexError::InvalidPattern(_)))
}

#[test]
fn the_syntax_of_the_engine_is_no_pattern() {
  for pattern in [
    r"\b",
    r"\Bx",
    r"\A",
    r"\z",
    r"\Z",
    "(?i)a",
    "(?=a)",
    "(?!a)",
    "(?<=a)b",
    "(?<n>a)",
    r"\x41",
    r"\u0041",
    r"\Qa\E",
    "a{",
    "a}",
    "a{,2}",
    "a**",
    "a+*",
    "(?:a)",
    r"\e",
    "[[:alpha:]]",
    r"\k<n>",
  ] {
    assert!(invalid(pattern), "{pattern:?} should be invalid");
  }
}

#[test]
fn the_classes_have_the_sets_of_xml_schema() {
  // \s: the four XML space characters only
  assert!(matches(" \t\n\r", r"^\s+$"));
  assert!(!matches("\u{A0}", r"^\s$"));
  assert!(!matches("\u{2003}", r"^\s$"));
  // \w: every character but punctuation, separators and others
  assert!(!matches("aé5_", r"^\w+$"));
  assert!(matches("aé5", r"^\w+$"));
  assert!(!matches("_", r"^\w$"));
  assert!(matches("+", r"^\w$"));
  // \d: decimal digits of every script
  assert!(matches("٣", r"^\d$"));
  // \i and \c: the XML name characters
  assert!(matches("_:a", r"^\i+$"));
  assert!(!matches("1", r"^\i$"));
  assert!(matches("a-1.", r"^\c+$"));
  assert!(!matches(" ", r"^\c$"));
  // `.`: every character but the line ends, or every character with `s`
  assert!(!matches("\n", "^.$"));
  assert!(!matches("\r", "^.$"));
  assert!(is_match("\n", "^.$", "s").unwrap());
  // categories and blocks
  assert!(matches("A", r"^\p{Lu}$"));
  assert!(matches("a", r"^\P{Lu}$"));
  assert!(matches("a", r"^\p{IsBasicLatin}$"));
  assert!(!matches("é", r"^\p{IsBasicLatin}$"));
  assert!(invalid(r"\p{IsNoSuchBlock}"));
  assert!(invalid(r"\p{Latin}"));
}

#[test]
fn character_classes() {
  assert!(matches("b", "^[a-z-[aeiou]]$"));
  assert!(!matches("e", "^[a-z-[aeiou]]$"));
  assert!(matches("1", "^[^a-z-[aeiou]]$"));
  assert!(matches("-", "^[-a]$"));
  assert!(matches("-", "^[a-]$"));
  assert!(matches(" ", r"^[\s]$"));
  assert!(matches("x", r"^[^\s]$"));
  assert!(matches("]", r"^[\]]$"));
  assert!(invalid("[a-z"));
  assert!(invalid("[z-a]"));
  assert!(invalid("[]"));
  assert!(invalid(r"[\s-z]"));
  // a '-' that starts no range is a character (XML Schema 1.1; TCK 1111 `[0-9-.]`)
  assert!(matches("-", "^[0-9-.]$"));
  assert!(matches("-", "^[a-c-e]$"));
}

#[test]
fn back_references_and_quantifiers() {
  assert!(matches("abab", r"^(ab)\1$"));
  assert!(!matches("abac", r"^(ab)\1$"));
  // as many digits as name a closed group
  assert!(matches("aa0", r"^(a)\10$"));
  assert!(!matches("a0", r"^(a)\10$"));
  assert!(invalid(r"(a)\2"));
  assert!(invalid(r"(a\1)"));
  assert!(matches("aaa", "^a{3}$"));
  assert!(matches("aaa", "^a{2,}$"));
  assert!(matches("aa", "^a{1,3}?$"));
  assert!(invalid("a{3,2}"));
  // back-references in replace and split too
  assert_eq!(Ok("x-x".to_string()), replace("aa-bb", r"(.)\1", "x", ""));
  assert_eq!(Ok(vec!["a".to_string(), "b".to_string()]), split("a11b", r"(1)\1"));
}

#[test]
fn the_input_is_matched_as_it_is() {
  assert!(matches("a\rb", "a\rb"));
  assert!(matches("x\ry", "\r"));
  assert!(!matches("a\r\nb", "a\nb"));
  assert!(matches("a\r\nb", "a\r\nb"));
}

#[test]
fn the_set_operators_of_the_engine_are_characters() {
  // `&&` and `~~` in a class are plain characters in XML Schema
  assert!(matches("&", "^[a&&b]$"));
  assert!(matches("~", "^[a~~b]$"));
  assert!(!matches("c", "^[a&&b]$"));
}

#[test]
fn review_of_f8() {
  // a back-reference to a group that matched nothing matches the zero-length string (F&O §7.6.1)
  assert!(matches("b", r"^(a)?\1b$"));
  assert!(matches("b", r"(a)|b\1"));
  assert!(matches("aab", r"^(a)?\1b$"));
  // a quantifier may follow ^ and $, which are atoms (F&O §7.6.1)
  assert!(matches("a", "^*a"));
  assert!(matches("a", "a$?"));
  // leading zeros do not count against the repetition limit
  assert!(matches("aaa", "^a{0000000003}$"));
  // an unescaped '-' is no endpoint of a range
  for pattern in ["[+--]", "[!--]", "[--a]"] {
    assert!(invalid(pattern), "{pattern:?} should be invalid");
  }
  assert!(matches("-", r"^[+-\-]$"));
  // the block names of XML Schema 1.0, private use and the surrogates included
  assert!(matches("\u{E000}", r"^\p{IsPrivateUse}$"));
  assert!(matches("\u{10300}", r"^\p{IsOldItalic}$"));
  assert!(!matches("a", r"\p{IsHighSurrogates}"));
  assert!(matches("a", r"^\P{IsHighSurrogates}$"));
  // deeply nested subtractions are a limit, not a crash
  let deep = format!("[a{}{}", "-[a".repeat(5000), "]".repeat(5001));
  assert!(matches!(is_match("a", &deep, ""), Err(RegexError::LimitExceeded(_))));
  // with x, the pattern is quoted as written
  match is_match("a", "[ a", "x") {
    Err(RegexError::InvalidPattern(error)) => assert!(error.to_string().contains("'[ a'"), "{error}"),
    other => panic!("{other:?}"),
  }
}
