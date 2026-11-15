//! Trait for converting FEEL artifact into FEEL string, and the writer of separated parts.

use std::fmt;

/// Trait for converting FEEL artifact into FEEL string.
pub trait ToFeelString {
  /// Converts FEEL artifact into FEEL string.
  fn to_feel_string(&self) -> String;
}

/// Writes `items` separated by `, `, each with `write_item`, straight to the formatter: no text is
/// built before it is written, so a writer that stops early stops the work.
pub(crate) fn write_separated<T>(
  f: &mut fmt::Formatter<'_>,
  items: impl IntoIterator<Item = T>,
  mut write_item: impl FnMut(&mut fmt::Formatter<'_>, T) -> fmt::Result,
) -> fmt::Result {
  for (index, item) in items.into_iter().enumerate() {
    if index > 0 {
      f.write_str(", ")?;
    }
    write_item(f, item)?;
  }
  Ok(())
}
