//! # FEEL string values

use crate::values::Value;
use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

/// A FEEL string.
///
/// Copying the value copies a pointer: the text is shared between all copies and is never
/// changed while it is shared. The only operation that changes a string is [FeelString::concat],
/// which appends in place when the string is not shared and allocates a new text otherwise.
///
/// Equality, order and hash are those of the text (byte order, as for [String]).
#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FeelString(Arc<String>);

impl FeelString {
  /// Returns the text of this string.
  pub fn as_str(&self) -> &str {
    self.0.as_str()
  }

  /// Converts this string into a [String], without a copy when the text is not shared.
  pub fn into_string(self) -> String {
    Arc::unwrap_or_clone(self.0)
  }

  /// FEEL string addition (DMN 1.6 §10.3.2.5): returns this string followed by `rhs`.
  ///
  /// When this string is not shared, `rhs` is appended in place. Otherwise the result is
  /// allocated once, at exactly the length of both texts, and the shared text is left unchanged.
  pub fn concat(mut self, rhs: &str) -> FeelString {
    if let Some(text) = Arc::get_mut(&mut self.0) {
      text.push_str(rhs);
      return self;
    }
    let mut text = String::with_capacity(self.0.len() + rhs.len());
    text.push_str(&self.0);
    text.push_str(rhs);
    Self(Arc::new(text))
  }
}

impl Deref for FeelString {
  type Target = str;

  fn deref(&self) -> &Self::Target {
    self.as_str()
  }
}

impl AsRef<str> for FeelString {
  fn as_ref(&self) -> &str {
    self.as_str()
  }
}

impl Borrow<str> for FeelString {
  fn borrow(&self) -> &str {
    self.as_str()
  }
}

impl fmt::Display for FeelString {
  /// Writes the text, as [String] does.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Display::fmt(self.as_str(), f)
  }
}

impl fmt::Debug for FeelString {
  /// Writes the quoted and escaped text, as [String] does.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Debug::fmt(self.as_str(), f)
  }
}

impl From<String> for FeelString {
  /// Moves the text into a new string, without a copy.
  fn from(value: String) -> Self {
    Self(Arc::new(value))
  }
}

impl From<&String> for FeelString {
  fn from(value: &String) -> Self {
    Self(Arc::new(value.clone()))
  }
}

impl From<&str> for FeelString {
  fn from(value: &str) -> Self {
    Self(Arc::new(value.to_string()))
  }
}

impl From<FeelString> for String {
  /// Converts the string into its text, without a copy when the text is not shared.
  fn from(value: FeelString) -> Self {
    value.into_string()
  }
}

impl From<FeelString> for Value {
  fn from(value: FeelString) -> Self {
    Value::String(value)
  }
}

impl PartialEq<str> for FeelString {
  fn eq(&self, other: &str) -> bool {
    self.as_str() == other
  }
}

impl PartialEq<&str> for FeelString {
  fn eq(&self, other: &&str) -> bool {
    self.as_str() == *other
  }
}

impl PartialEq<String> for FeelString {
  fn eq(&self, other: &String) -> bool {
    self.as_str() == other.as_str()
  }
}
