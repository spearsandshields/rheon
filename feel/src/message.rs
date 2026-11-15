//! # Bounded messages
//!
//! Null reasons, errors and warnings quote values, types, names and input text. A quoted argument
//! goes through [arg]: when its text is at most 120 characters, it is quoted whole; otherwise as a
//! preview, its first 120 characters, `…` and its size when it has one (`[1, 2, 3, …, 32, … (1000
//! items)`). A number whose plain text is longer than 40 characters is quoted in scientific
//! notation (`1E+50`), never cut. Integers, characters and fixed text are not arguments, and a
//! nested message (an error or a reason) is quoted as it is.
//!
//! Every message is built by [compose], which keeps at most 1,000 characters and marks a cut with
//! `…`. Formatting stops at the cap and a preview stops at its 120 characters, so a message
//! allocates a bounded amount, never a whole value or type: the type of a value is quoted as
//! [TypeOf](crate::values::TypeOf), which writes the type from the value and builds at most a
//! bounded number of type nodes. Its time is at most linear in the quoted values (the size of a
//! cut string counts its characters, and the type of a list checks every item).

use std::fmt::{self, Display, Write};

/// Characters of an argument quoted whole; a longer one is quoted as a preview of this length.
pub(crate) const PREVIEW_CHARS: usize = 120;

/// Characters of a message at most, the `…` of a cut included.
const MAX_CHARS: usize = 1_000;

/// The size of a quoted argument, shown after its preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
  /// The number of items of a list.
  Items(usize),
  /// The number of entries of a context or a context type.
  Entries(usize),
  /// The number of characters of a text.
  Characters(usize),
}

impl Display for Size {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let (count, one, many) = match self {
      Size::Items(count) => (count, "item", "items"),
      Size::Entries(count) => (count, "entry", "entries"),
      Size::Characters(count) => (count, "character", "characters"),
    };
    write!(f, "{count} {}", if *count == 1 { one } else { many })
  }
}

/// Something a message may quote.
pub trait MessageArg {
  /// Writes the whole text of the argument; a preview stops it when it has enough.
  fn write_message(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;

  /// The size shown after a preview, if the argument has one.
  fn size(&self) -> Option<Size> {
    None
  }

  /// Whether the text is known to be at most 120 characters without writing it (a number, a
  /// short string): such an argument is written straight, with no preview to watch it.
  fn is_short(&self) -> bool {
    false
  }
}

/// An argument quoted in a message (see [arg]).
pub struct Arg<'a, T: ?Sized>(&'a T);

/// Quotes `value` in a message: its text when that is at most 120 characters, else a preview.
pub fn arg<T: MessageArg + ?Sized>(value: &T) -> Arg<'_, T> {
  Arg(value)
}

impl<T: MessageArg + ?Sized> Display for Arg<'_, T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if self.0.is_short() {
      return self.0.write_message(f);
    }
    // the preview is the start of the text either way, so it is written as it comes
    let mut preview = Preview { f, chars: 0, full: false };
    if write!(preview, "{}", Whole(self.0)).is_ok() {
      return Ok(());
    }
    if !preview.full {
      // the formatter refused the text
      return Err(fmt::Error);
    }
    f.write_str("…")?;
    if let Some(size) = self.0.size() {
      write!(f, " ({size})")?;
    }
    Ok(())
  }
}

/// The whole text of an argument, as [Display].
struct Whole<'a, T: ?Sized>(&'a T);

impl<T: MessageArg + ?Sized> Display for Whole<'_, T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.0.write_message(f)
  }
}

/// Passes the first [PREVIEW_CHARS] characters on and refuses the next one.
struct Preview<'a, 'b> {
  f: &'a mut fmt::Formatter<'b>,
  chars: usize,
  /// Whether a character beyond the preview was refused.
  full: bool,
}

impl Write for Preview<'_, '_> {
  fn write_str(&mut self, s: &str) -> fmt::Result {
    let left = PREVIEW_CHARS - self.chars;
    // a text of at most `left` bytes has at most `left` characters
    if s.len() <= left {
      self.chars += s.chars().count();
      return self.f.write_str(s);
    }
    match s.char_indices().nth(left) {
      None => {
        self.chars += s.chars().count();
        self.f.write_str(s)
      }
      Some((end, _)) => {
        self.chars = PREVIEW_CHARS;
        self.f.write_str(&s[..end])?;
        self.full = true;
        Err(fmt::Error)
      }
    }
  }
}

/// The message written by `args`, at most 1,000 characters: a longer one keeps its first 999
/// characters and `…`, and formatting stops there.
pub fn compose(args: fmt::Arguments<'_>) -> String {
  if let Some(text) = args.as_str()
    && text.len() <= MAX_CHARS
  {
    return text.to_string();
  }
  let mut message = Capped {
    text: String::with_capacity(INITIAL_CAPACITY),
    chars: None,
    cut: false,
  };
  // an error is the cut (or a failing argument, whose text so far is kept)
  let _ = fmt::write(&mut message, args);
  message.text
}

/// Bytes reserved for a message before it is written: most messages fit.
const INITIAL_CAPACITY: usize = 64;

/// Keeps at most [MAX_CHARS] characters; a cut replaces the last kept one with `…`.
struct Capped {
  text: String,
  /// The characters of `text`, counted once its bytes could pass the cap (a text of at most
  /// [MAX_CHARS] bytes has at most that many characters).
  chars: Option<usize>,
  cut: bool,
}

impl Write for Capped {
  fn write_str(&mut self, s: &str) -> fmt::Result {
    if self.cut {
      return Err(fmt::Error);
    }
    if self.chars.is_none() && self.text.len() + s.len() <= MAX_CHARS {
      self.text.push_str(s);
      return Ok(());
    }
    let mut chars = self.chars.unwrap_or_else(|| self.text.chars().count());
    for c in s.chars() {
      if chars == MAX_CHARS {
        self.text.pop();
        self.text.push('…');
        self.cut = true;
        self.chars = Some(chars);
        return Err(fmt::Error);
      }
      self.text.push(c);
      chars += 1;
    }
    self.chars = Some(chars);
    Ok(())
  }
}

impl<T: MessageArg + ?Sized> MessageArg for &T {
  fn write_message(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    (**self).write_message(f)
  }

  fn size(&self) -> Option<Size> {
    (**self).size()
  }

  fn is_short(&self) -> bool {
    (**self).is_short()
  }
}

impl MessageArg for str {
  fn write_message(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(self)
  }

  fn size(&self) -> Option<Size> {
    Some(Size::Characters(self.chars().count()))
  }

  fn is_short(&self) -> bool {
    self.len() <= PREVIEW_CHARS
  }
}

impl MessageArg for String {
  fn write_message(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(self)
  }

  fn size(&self) -> Option<Size> {
    self.as_str().size()
  }

  fn is_short(&self) -> bool {
    self.as_str().is_short()
  }
}

impl MessageArg for serde_json::Value {
  fn write_message(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Display::fmt(self, f)
  }

  fn size(&self) -> Option<Size> {
    match self {
      serde_json::Value::Array(items) => Some(Size::Items(items.len())),
      serde_json::Value::Object(entries) => Some(Size::Entries(entries.len())),
      serde_json::Value::String(text) => text.size(),
      _ => None,
    }
  }
}
