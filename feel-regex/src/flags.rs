//! The flags of the regex built-ins and the pattern they make for an engine.
//!
//! DMN 1.6 Table 75 note 2: the flags "SHALL conform to the syntax and constraints specified in
//! clause 7.6 of XQuery 1.0 and XPath 2.0 Functions and Operators", whose §7.6.1.1 defines `s`,
//! `m`, `i` and `x`. `q` is the flag of XPath and XQuery Functions and Operators 3.1 §5.6.2, kept
//! as an extension.

use crate::errors::{RegexError, err_invalid_flag};
use crate::utils::is_special_character;

/// The options that a flags string switches on.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Flags {
  /// `i`: case-insensitive mode.
  case_insensitive: bool,
  /// `m`: multi-line mode.
  multi_line: bool,
  /// `s`: dot-all mode.
  dot_all: bool,
  /// `x`: whitespace outside character classes is removed from the pattern.
  free_spacing: bool,
  /// `q`: every character of the pattern represents itself.
  literal: bool,
}

impl Flags {
  /// The options of `flags`. F&O §7.6.1.1: "The presence of a letter within the string indicates
  /// that the option is on; its absence indicates that the option is off. Letters may appear in
  /// any order and may be repeated. If there are characters present that are not defined here as
  /// flags, then an error is raised [err:FORX0001]." So `""` switches every option off, and no
  /// character is trimmed.
  pub fn parse(flags: &str) -> Result<Self, RegexError> {
    let mut options = Self::default();
    for flag in flags.chars() {
      match flag {
        'i' => options.case_insensitive = true,
        'm' => options.multi_line = true,
        's' => options.dot_all = true,
        'x' => options.free_spacing = true,
        'q' => options.literal = true,
        other => return Err(RegexError::InvalidArgument(err_invalid_flag(other))),
      }
    }
    Ok(options)
  }

  /// Whether every character of the pattern represents itself (`q`), and so does every
  /// character of a replacement string.
  pub fn literal(&self) -> bool {
    self.literal
  }

  /// Whether `pattern` with these flags matches exactly the inputs that contain it as it is
  /// written, so no engine is needed: there is no flag, or only `q`; the pattern is not empty;
  /// with `q` any character stands for itself, and without it the pattern has only letters,
  /// digits, spaces and punctuation that no pattern syntax gives a meaning; and it has no line
  /// end, the one thing that an input is changed for before it is matched.
  pub fn plain(&self, pattern: &str) -> bool {
    if self.case_insensitive || self.multi_line || self.dot_all || self.free_spacing || pattern.is_empty() {
      return false;
    }
    pattern.chars().all(|ch| match ch {
      '\r' | '\n' => false,
      _ if self.literal => true,
      ' ' | '_' | '@' | ',' | ':' | ';' | '=' | '!' | '\'' | '"' | '/' | '%' | '#' => true,
      ch => ch.is_alphanumeric(),
    })
  }

  /// The pattern as an engine reads it, the options it supports as a leading `(?ims)` group.
  ///
  /// With `q` (F&O 3.1 §5.6.2) "all characters in the regular expression are treated as
  /// representing themselves", and "If it is used together with the m, s, or x flag, that flag
  /// has no effect": the pattern is quoted, and only `i` applies. Otherwise `x` removes the
  /// whitespace first, and `translate` turns the result into the engine's syntax.
  pub fn source(&self, pattern: &str, translate: impl FnOnce(&str) -> String) -> String {
    let mut source = String::with_capacity(pattern.len() + 8);
    if self.literal {
      if self.case_insensitive {
        source.push_str("(?i)");
      }
      for ch in pattern.chars() {
        if is_special_character(ch) {
          source.push('\\');
        }
        source.push(ch);
      }
      return source;
    }
    if self.case_insensitive || self.multi_line || self.dot_all {
      source.push_str("(?");
      for (on, letter) in [(self.case_insensitive, 'i'), (self.multi_line, 'm'), (self.dot_all, 's')] {
        if on {
          source.push(letter);
        }
      }
      source.push(')');
    }
    if self.free_spacing {
      source.push_str(&translate(&without_whitespace(pattern)));
    } else {
      source.push_str(&translate(pattern));
    }
    source
  }
}

/// `pattern` without the whitespace that `x` removes. F&O §7.6.1.1: "whitespace characters (#x9,
/// #xA, #xD and #x20) in the regular expression are removed prior to matching with one exception:
/// whitespace characters within character class expressions (charClassExpr) are not removed."
///
/// The whitespace after a backslash outside a class is removed as well, so a backslash escapes
/// the next character that stays: `"hello\ sworld"` reads `hello\sworld`, as the clause's example
/// expects, and an escaped bracket opens or closes no class. Inside a class every character stays,
/// and a subtraction (`[a-z-[aeiou]]`) is a class nested in it.
fn without_whitespace(pattern: &str) -> String {
  let mut output = String::with_capacity(pattern.len());
  // the nesting depth of character class expressions
  let mut depth = 0_usize;
  let mut escaped = false;
  for ch in pattern.chars() {
    let removed = depth == 0 && matches!(ch, '\t' | '\n' | '\r' | ' ');
    if escaped {
      if !removed {
        output.push(ch);
        escaped = false;
      }
      continue;
    }
    match ch {
      _ if removed => continue,
      '\\' => escaped = true,
      '[' => depth += 1,
      ']' if depth > 0 => depth -= 1,
      _ => {}
    }
    output.push(ch);
  }
  output
}

#[cfg(test)]
mod tests {
  use super::*;

  fn source(pattern: &str, flags: &str) -> String {
    Flags::parse(flags).unwrap().source(pattern, str::to_string)
  }

  #[test]
  fn every_character_is_a_flag() {
    assert_eq!(Ok(Flags::default()), Flags::parse(""));
    for flags in [" ", " i", "i ", "I", "g", "u", "\n"] {
      assert!(matches!(Flags::parse(flags), Err(RegexError::InvalidArgument(_))), "{flags:?}");
    }
    assert_eq!(Flags::parse("i"), Flags::parse("iii"));
  }

  #[test]
  fn the_options_lead_the_pattern() {
    assert_eq!("a", source("a", ""));
    assert_eq!("(?ims)a", source("a", "smi"));
    assert_eq!("(?s)a", source("a", "ss"));
  }

  #[test]
  fn x_keeps_the_whitespace_of_character_classes() {
    assert_eq!("helloworld", source("hello world", "x"));
    assert_eq!("hello[ ]world", source("hello[ ]world", "x"));
    assert_eq!(r"hello\sworld", source("hello\\\n sworld", "x"));
    assert_eq!("ab", source(" a\t\r\nb ", "x"));
    assert_eq!(r"a\[b", source(r"a\ [ b", "x"));
    assert_eq!(r"[\] ]", source(r"[\] ]", "x"));
    assert_eq!("[a- z-[ b ]]c", source("[a- z-[ b ]] c", "x"));
    assert_eq!(r"\p{IsBasicLatin}", source(r"\p{ Is Basic Latin }", "x"));
    assert_eq!("a#b", source("a # b", "x"));
    assert_eq!("\\", source("\\ ", "x"));
  }

  #[test]
  fn q_quotes_the_pattern_and_keeps_only_i() {
    assert_eq!(r"a\.b\[c\]", source("a.b[c]", "q"));
    assert_eq!(r"(?i)a\.b", source("a.b", "qi"));
    assert_eq!(r"a b\.", source("a b.", "qsmx"));
    assert_eq!(r"\\p\{L\}", source(r"\p{L}", "q"));
  }

  #[test]
  fn translate_runs_on_the_pattern_without_whitespace() {
    let flags = Flags::parse("x").unwrap();
    assert_eq!("<ab>", flags.source("a b", |pattern| format!("<{pattern}>")));
    let flags = Flags::parse("q").unwrap();
    assert_eq!("a", flags.source("a", |_| unreachable!("q translates nothing")));
  }
}
