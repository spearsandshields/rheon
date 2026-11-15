//! The replacement string of `replace`, read once before the matches are replaced.
//!
//! XQuery 1.0 and XPath 2.0 F&O §7.6.3, where S is the number of parenthesized sub-expressions
//! and "N is the decimal number formed by taking all the digits that consecutively follow the $
//! character":
//!
//! - "If N=0, then the variable is replaced by the substring matched by the regular expression as
//!   a whole."
//! - "If 1<=N<=S, then the variable is replaced by the substring captured by the Nth
//!   parenthesized sub-expression. If the Nth parenthesized sub-expression was not matched, then
//!   the variable is replaced by the zero-length string."
//! - "If S<N<=9, then the variable is replaced by the zero-length string."
//! - "Otherwise (if N>S and N>9), the last digit of N is taken to be a literal character to be
//!   included "as is" in the replacement string, and the rules are reapplied using the number N
//!   formed by stripping off this last digit."
//!
//! "A literal "$" symbol must be written as "\$"", "A literal "\" symbol must be written as
//! "\\"", and FORX0004 is raised for a `$` "that is not immediately followed by a digit 0-9 and
//! not immediately preceded by a "\"", and for a `\` "that is not part of a "\\" pair, unless it
//! is immediately followed by a "$" character". The string is read from left to right, so in
//! `\\$1` the pair comes first and `$1` refers to a group.
//!
//! With `q` (F&O 3.1 §5.6.4, an extension under DMN 1.6) "the replacement string is used as is".

use crate::errors::{RegexError, err_lone_backslash, err_lone_dollar};
use std::borrow::Cow;

/// A part of a replacement string.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Part {
  /// Text included as is.
  Literal(String),
  /// The substring captured by a group; 0 is the whole match.
  Group(usize),
}

/// A replacement string read for a pattern with a given number of groups.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replacement(Vec<Part>);

impl Replacement {
  /// Reads `text` for a pattern with `groups` parenthesized sub-expressions; with `literal`
  /// (`q`) the text is used as is.
  pub fn parse(text: &str, groups: usize, literal: bool) -> Result<Self, RegexError> {
    if literal {
      return Ok(Self(vec![Part::Literal(text.to_string())]));
    }
    let mut parts = Vec::new();
    let mut literal = String::new();
    let mut chars = text.char_indices().peekable();
    while let Some((position, ch)) = chars.next() {
      match ch {
        '\\' => match chars.next() {
          Some((_, escaped @ ('\\' | '$'))) => literal.push(escaped),
          _ => return Err(RegexError::InvalidArgument(err_lone_backslash())),
        },
        '$' => {
          let digits = text[position + 1..].bytes().take_while(u8::is_ascii_digit).count();
          if digits == 0 {
            return Err(RegexError::InvalidArgument(err_lone_dollar()));
          }
          let digits = &text[position + 1..position + 1 + digits];
          // the longest leading digits whose number is at most S or at most 9; the numbers of
          // longer leading digits never decrease, and the first digit is always at most 9
          let bound = groups.max(9);
          let mut number = 0_usize;
          let mut taken = 0;
          for digit in digits.bytes() {
            let next = number.saturating_mul(10).saturating_add(usize::from(digit - b'0'));
            if taken > 0 && next > bound {
              break;
            }
            number = next;
            taken += 1;
          }
          if number <= groups {
            if !literal.is_empty() {
              parts.push(Part::Literal(std::mem::take(&mut literal)));
            }
            parts.push(Part::Group(number));
          }
          literal.push_str(&digits[taken..]);
          for _ in 0..digits.len() {
            chars.next();
          }
        }
        other => literal.push(other),
      }
    }
    if !literal.is_empty() {
      parts.push(Part::Literal(literal));
    }
    Ok(Self(parts))
  }
}

impl regex::Replacer for &Replacement {
  fn replace_append(&mut self, captures: &regex::Captures<'_>, output: &mut String) {
    for part in &self.0 {
      match part {
        Part::Literal(text) => output.push_str(text),
        Part::Group(group) => output.push_str(captures.get(*group).map_or("", |capture| capture.as_str())),
      }
    }
  }

  fn no_expansion(&mut self) -> Option<Cow<'_, str>> {
    match self.0.as_slice() {
      [] => Some(Cow::Borrowed("")),
      [Part::Literal(text)] => Some(Cow::Borrowed(text)),
      _ => None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn parts(text: &str, groups: usize) -> Vec<Part> {
    Replacement::parse(text, groups, false).unwrap().0
  }

  fn literal(text: &str) -> Part {
    Part::Literal(text.to_string())
  }

  #[test]
  fn digits_name_a_group_while_their_number_is_at_most_the_groups_or_9() {
    assert_eq!(vec![Part::Group(0)], parts("$0", 0));
    assert_eq!(vec![Part::Group(0)], parts("$00", 0));
    assert_eq!(vec![literal("a"), Part::Group(1), literal("b")], parts("a$1b", 1));
    assert_eq!(Vec::<Part>::new(), parts("$5", 1));
    assert_eq!(vec![literal("0")], parts("$50", 1));
    assert_eq!(vec![Part::Group(1), literal("0")], parts("$10", 1));
    assert_eq!(vec![Part::Group(10)], parts("$10", 10));
    assert_eq!(vec![Part::Group(2), literal("3")], parts("$23", 5));
    assert_eq!(vec![Part::Group(12), Part::Group(1)], parts("$12$1", 12));
    let digits = "9".repeat(100);
    assert_eq!(vec![literal(&"9".repeat(99))], parts(&format!("${digits}"), 3));
    assert_eq!(vec![Part::Group(99), literal(&"9".repeat(98))], parts(&format!("${digits}"), 100));
    // the number saturates instead of overflowing
    assert_eq!(vec![Part::Group(usize::MAX)], parts(&format!("${digits}"), usize::MAX));
  }

  #[test]
  fn a_backslash_escapes_a_dollar_or_a_backslash() {
    assert_eq!(vec![literal("$")], parts(r"\$", 0));
    assert_eq!(vec![literal(r"\")], parts(r"\\", 0));
    assert_eq!(vec![literal(r"\"), Part::Group(1)], parts(r"\\$1", 1));
    assert_eq!(vec![literal("$1")], parts(r"\$1", 1));
    assert_eq!(vec![literal("é€")], parts("é€", 0));
  }

  #[test]
  fn a_lone_dollar_or_backslash_is_invalid() {
    for text in ["$", "$$", "${1}", "$a", "a$", r"\\$"] {
      assert!(matches!(Replacement::parse(text, 1, false), Err(RegexError::InvalidArgument(_))), "{text}");
    }
    for text in [r"\", r"\x", r"\\\", r"a\1"] {
      assert!(matches!(Replacement::parse(text, 1, false), Err(RegexError::InvalidArgument(_))), "{text}");
    }
  }

  #[test]
  fn q_takes_the_text_as_is() {
    assert_eq!(vec![literal(r"$1\x$")], Replacement::parse(r"$1\x$", 1, true).unwrap().0);
  }
}
