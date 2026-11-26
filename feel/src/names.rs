//! `FEEL` name implementation.

use crate::message::{MessageArg, Size};
use dsntk_common::Jsonify;
use std::borrow::Borrow;
use std::fmt;
use std::sync::Arc;

/// `FEEL` name.
///
/// The text of a name is shared: a clone is another handle of the same text, without a copy,
/// so the names of the records of a list can be one text each.
#[derive(Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Clone)]
pub struct Name(Arc<str>);

impl From<Vec<String>> for Name {
  /// Converts a vector of strings into [Name].
  fn from(value: Vec<String>) -> Self {
    Self::join(&value)
  }
}

impl From<Vec<&str>> for Name {
  /// Converts a vector of string references into [Name].
  fn from(value: Vec<&str>) -> Self {
    Self::new(&value)
  }
}

impl From<String> for Name {
  /// Converts a [String] into [Name], without white space at its ends.
  fn from(value: String) -> Self {
    Self(Arc::from(trim(&value)))
  }
}

impl From<&String> for Name {
  /// Converts a reference to [String] into [Name].
  fn from(value: &String) -> Self {
    Self(Arc::from(trim(value)))
  }
}

impl From<&str> for Name {
  /// Converts a reference to [str] into [Name].
  fn from(value: &str) -> Self {
    Self(Arc::from(trim(value)))
  }
}

impl From<Name> for String {
  /// Converts [Name] to its [String] representation.
  fn from(value: Name) -> Self {
    value.0.as_ref().to_string()
  }
}

impl From<&Name> for String {
  /// Converts a reference to [Name] to its [String] representation.
  fn from(value: &Name) -> Self {
    value.0.as_ref().to_string()
  }
}

/// A [Name] can be looked up by its text in hashed and ordered collections (`HashSet<Name>`,
/// `BTreeMap<Name, _>`) without building a [Name]: `Eq`, `Ord` and `Hash` are derived on the
/// single text field, so they agree with those of `str`.
impl Borrow<str> for Name {
  fn borrow(&self) -> &str {
    &self.0
  }
}

impl fmt::Display for Name {
  /// Implements [Display](fmt::Display) trait for [Name].
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl MessageArg for Name {
  fn write_message(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.0)
  }

  fn size(&self) -> Option<Size> {
    self.as_str().size()
  }

  fn is_short(&self) -> bool {
    self.as_str().is_short()
  }
}

impl Jsonify for Name {
  /// The name as a JSON string, escaped as JSON escapes it.
  fn jsonify(&self) -> String {
    crate::values::json_string(self.0.as_ref())
  }
}

/// Returns `true` when `ch` is white space (DMN 1.6 grammar rule 61, with the vertical space
/// characters): it separates the tokens of a name.
pub fn is_white_space(ch: char) -> bool {
  matches!(
    ch,
    '\u{0009}'..='\u{000D}' | '\u{0020}' | '\u{0085}' | '\u{00A0}' | '\u{1680}' | '\u{180E}' | '\u{2000}'..='\u{200B}' | '\u{2028}' | '\u{2029}' | '\u{202F}' | '\u{205F}' | '\u{3000}' | '\u{FEFF}'
  )
}

/// Returns `text` without the white space of FEEL ([is_white_space]) at its ends: the white
/// space that a name does not have. It is not the white space of Rust's `trim`, which leaves
/// U+180E, U+200B and U+FEFF.
pub fn trim(text: &str) -> &str {
  let bytes = text.as_bytes();
  // the usual text: a character of ASCII that is no white space at each end
  if let (Some(&first), Some(&last)) = (bytes.first(), bytes.last())
    && first.is_ascii_graphic()
    && last.is_ascii_graphic()
  {
    return text;
  }
  text.trim_matches(is_white_space)
}

/// Returns `true` when `ch` is an additional name symbol (DMN 1.6 grammar rule 30).
pub fn is_additional_name_symbol(ch: char) -> bool {
  matches!(ch, '.' | '/' | '-' | '\'' | '+' | '*')
}

/// Returns `true` when `text` is written in the form in which FEEL reads a name: its tokens
/// joined by one space, none next to an additional name symbol, none at an end. A name is a
/// list of tokens (DMN 1.6 clause 10.3.1.4), so a text in another form (`Date - Time`,
/// `a  b`) is the same name as its text in this form (`Date-Time`, `a b`).
pub fn is_in_name_form(text: &str) -> bool {
  // a text of ASCII without white space, as most keys are
  if text.bytes().all(|byte| byte > b' ' && byte < 0x80) {
    return true;
  }
  let mut before: Option<char> = None;
  let mut chars = text.chars().peekable();
  while let Some(ch) = chars.next() {
    if is_white_space(ch) {
      let fits = ch == ' '
        && before.is_some_and(|before| !is_white_space(before) && !is_additional_name_symbol(before))
        && chars.peek().is_some_and(|after| !is_white_space(*after) && !is_additional_name_symbol(*after));
      if !fits {
        return false;
      }
    }
    before = Some(ch);
  }
  true
}

/// Returns `text` in the form of [is_in_name_form]: its tokens joined by one space, none next to
/// an additional name symbol.
pub fn name_form(text: &str) -> String {
  let mut form = String::with_capacity(text.len());
  let mut before: Option<char> = None;
  let mut space = false;
  for ch in text.chars() {
    if is_white_space(ch) {
      space = true;
      continue;
    }
    if space
      && let Some(before) = before
      && !is_additional_name_symbol(before)
      && !is_additional_name_symbol(ch)
    {
      form.push(' ');
    }
    space = false;
    form.push(ch);
    before = Some(ch);
  }
  form
}

/// Returns `true` when `text`, written with any white space between its tokens, is the name
/// that `name` is in the form of [is_in_name_form].
pub fn is_name(text: &str, name: &str) -> bool {
  let mut expected = name.chars();
  let mut before: Option<char> = None;
  let mut space = false;
  for ch in text.chars() {
    if is_white_space(ch) {
      space = true;
      continue;
    }
    if space
      && let Some(before) = before
      && !is_additional_name_symbol(before)
      && !is_additional_name_symbol(ch)
      && expected.next() != Some(' ')
    {
      return false;
    }
    space = false;
    if expected.next() != Some(ch) {
      return false;
    }
    before = Some(ch);
  }
  expected.next().is_none()
}

impl Name {
  /// Creates a [Name] from string parts.
  pub fn new(parts: &[&str]) -> Self {
    Self::join(parts)
  }

  /// Creates a [Name] from string parts, like [Name::new], without collecting the parts first.
  pub fn from_parts<S: AsRef<str>>(parts: impl IntoIterator<Item = S>) -> Self {
    Self::join(parts)
  }

  /// Joins trimmed name parts with single spaces, except around additional name symbols.
  fn join<S: AsRef<str>>(parts: impl IntoIterator<Item = S>) -> Self {
    let mut parts = parts.into_iter();
    let Some(first) = parts.next() else {
      return Self::default();
    };
    let mut parts = parts.peekable();
    if parts.peek().is_none() {
      // a name of one part is the part: one text, written once
      return Self(Arc::from(trim(first.as_ref())));
    }
    let mut result = String::new();
    let mut current;
    let mut prev = false;
    for (index, part) in std::iter::once(first).chain(parts).enumerate() {
      let part = trim(part.as_ref());
      current = matches!(part, "." | "/" | "-" | "'" | "+" | "*");
      if index > 0 && !prev && !current && !part.is_empty() {
        result.push(' ');
      }
      result.push_str(part);
      prev = current;
    }
    Self(Arc::from(result))
  }

  /// Extracts a string slice containing the entire [Name].
  pub fn as_str(&self) -> &str {
    &self.0
  }

  /// Returns `true` when this name is empty.
  pub fn is_empty(&self) -> bool {
    self.0.is_empty()
  }
}
