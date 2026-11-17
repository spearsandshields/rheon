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
    Self(Arc::from(value.trim()))
  }
}

impl From<&String> for Name {
  /// Converts a reference to [String] into [Name].
  fn from(value: &String) -> Self {
    Self(Arc::from(value.trim()))
  }
}

impl From<&str> for Name {
  /// Converts a reference to [str] into [Name].
  fn from(value: &str) -> Self {
    Self(Arc::from(value.trim()))
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
  /// Converts [Name] to its `JSON` representation.
  fn jsonify(&self) -> String {
    self.0.as_ref().to_string()
  }
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
      return Self(Arc::from(first.as_ref().trim()));
    }
    let mut result = String::new();
    let mut current;
    let mut prev = false;
    for (index, part) in std::iter::once(first).chain(parts).enumerate() {
      let part = part.as_ref().trim();
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
