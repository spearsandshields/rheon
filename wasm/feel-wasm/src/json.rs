//! A strict reader of the request text, which converts the JSON of `context` and `input` into
//! FEEL values in one pass, without a tree of JSON values between the text and the values.
//!
//! The reader answers only what it is sure of. [fields] accepts a text only when it is a JSON
//! object as `serde_json::from_str` accepts it (the same grammar, escapes and nesting limit), and
//! [Reader] converts only what converts without an error. Whatever they refuse (`None`,
//! [Refused]) is answered by the path through the tree of JSON values, as before, so every
//! error and its text come from there.
//!
//! The values and the warnings are those of [crate::input::json_into_value] for the same text:
//! - of two entries of an object with the same key, the later one in the text gives the value;
//! - the warnings of the entries of an object come in the order of its keys, not of the text.
//!
//! The reader refuses what it would have to work for to answer the same: a key with white space
//! at an end (the name is the key without it, and of two such keys the later one in the order of
//! the keys gives the entry), and an object whose keys do not come in ascending order when one
//! of its values has a warning.

use crate::input::{DIRECT_ENTRIES, Path, number_value};
use rheon_feel::Name;
use rheon_feel::context::FeelContext;
use rheon_feel::values::Value;
use rheon_feel_number::FeelNumber;
use std::borrow::Cow;

/// A JSON number text with an exponent as `serde_json` hands it on: `e` in lower case and
/// always a sign. A warning quotes this text.
fn exponent_as_serde_json(number: &str) -> Cow<'_, str> {
  match number.find(['e', 'E']) {
    None => Cow::Borrowed(number),
    Some(at) => {
      let (mantissa, exponent) = (&number[..at], &number[at + 1..]);
      let sign = if exponent.starts_with(['+', '-']) { "" } else { "+" };
      if sign.is_empty() && number.as_bytes()[at] == b'e' {
        Cow::Borrowed(number)
      } else {
        Cow::Owned(format!("{mantissa}e{sign}{exponent}"))
      }
    }
  }
}

/// The deepest nesting of arrays and objects that `serde_json` reads is one less than this.
const NESTING_LIMIT: usize = 128;

/// The key by which `serde_json` (with `arbitrary_precision`) hands a number to its tree: an
/// object with this key is read as a number there, so it is left to that path.
const PRIVATE_NUMBER_KEY: &str = "$serde_json::private::Number";

/// The reader does not answer this text.
#[derive(Debug)]
pub(crate) struct Refused;

/// What the reader made of the JSON of `context` or `input`.
pub(crate) enum Field<'a, T> {
  /// The value and the warnings of reading it.
  Read(T, Vec<String>),
  /// The text of the JSON, which is an object or an array that the reader refused: for the
  /// conversion through a tree of JSON values.
  Text(&'a str),
}

/// A request as the reader takes it: the fields of the top-level object in the order of the
/// text, each key with the text of its value, without `context` and `input` where their JSON is
/// an object or an array (for `context`, read when it is an object); of two fields with one key
/// the later one counts.
pub(crate) struct Request<'a> {
  pub(crate) fields: Vec<(&'a str, &'a str)>,
  pub(crate) context: Option<Field<'a, FeelContext>>,
  pub(crate) input: Option<Field<'a, Value>>,
}

/// Reads a request: `None` when the text is not a JSON object that `serde_json` accepts, or a
/// key of the object is written with an escape. The JSON of `context` and `input` is read into
/// values on the way, in the one pass that finds its end.
pub(crate) fn request(text: &str) -> Option<Request<'_>> {
  let bytes = text.as_bytes();
  let mut request = Request {
    fields: vec![],
    context: None,
    input: None,
  };
  let all = top_level(text, |key, at| {
    let nested = matches!(bytes.get(at), Some(b'{' | b'['));
    match key {
      "context" if nested => {
        let mut warnings = vec![];
        let read = if bytes.get(at) == Some(&b'{') {
          Reader::object_at(text, at, &Path::Root(""), &mut warnings).ok()
        } else {
          None
        };
        let (field, end) = match read {
          Some((context, end)) => (Field::Read(context, warnings), end),
          None => {
            let end = skip_value(bytes, at, 1)?;
            (Field::Text(text.get(at..end)?), end)
          }
        };
        request.context = Some(field);
        Some((end, false))
      }
      "input" if nested => {
        let mut warnings = vec![];
        let (field, end) = match Reader::value_at(text, at, &Path::Root("input"), &mut warnings) {
          Ok((value, end)) => (Field::Read(value, warnings), end),
          Err(Refused) => {
            let end = skip_value(bytes, at, 1)?;
            (Field::Text(text.get(at..end)?), end)
          }
        };
        request.input = Some(field);
        Some((end, false))
      }
      _ => {
        // a later field of the key that is no object or array counts instead of an earlier one
        match key {
          "context" => request.context = None,
          "input" => request.input = None,
          _ => {}
        }
        Some((skip_value(bytes, at, 1)?, true))
      }
    }
  })?;
  // an earlier `context` or `input` that was kept as a field does not count after a later one
  // that was read: the later one counts
  request.fields = all;
  if request.context.is_some() {
    request.fields.retain(|(key, _)| *key != "context");
  }
  if request.input.is_some() {
    request.fields.retain(|(key, _)| *key != "input");
  }
  Some(request)
}

/// The fields of a request: each key of the top-level object with the text of its value, in the
/// order of the text. `None` when the text is not a JSON object that `serde_json` accepts, or a
/// key is written with an escape.
#[cfg(test)]
pub(crate) fn fields(text: &str) -> Option<Vec<(&str, &str)>> {
  top_level(text, |_, at| Some((skip_value(text.as_bytes(), at, 1)?, true)))
}

/// Walks the top-level object of a request text. `value` gets each key and the position of its
/// value, and returns the position after the value and whether the field is kept; the kept
/// fields are returned, each key with the text of its value. `None` when the text is no JSON
/// object, a key is written with an escape, or `value` returns `None`.
fn top_level<'a>(text: &'a str, mut value: impl FnMut(&'a str, usize) -> Option<(usize, bool)>) -> Option<Vec<(&'a str, &'a str)>> {
  let bytes = text.as_bytes();
  let mut at = skip_white(bytes, 0);
  if bytes.get(at) != Some(&b'{') {
    return None;
  }
  at = skip_white(bytes, at + 1);
  let mut fields = vec![];
  if bytes.get(at) == Some(&b'}') {
    at += 1;
  } else {
    loop {
      let key_end = skip_key(bytes, at)?;
      let key = text.get(at + 1..key_end - 1)?;
      if key.contains('\\') {
        return None;
      }
      at = skip_white(bytes, key_end);
      if bytes.get(at) != Some(&b':') {
        return None;
      }
      at = skip_white(bytes, at + 1);
      let (end, kept) = value(key, at)?;
      if kept {
        fields.push((key, text.get(at..end)?));
      }
      at = skip_white(bytes, end);
      match bytes.get(at) {
        Some(b',') => at = skip_white(bytes, at + 1),
        Some(b'}') => {
          at += 1;
          break;
        }
        _ => return None,
      }
    }
  }
  (skip_white(bytes, at) == bytes.len()).then_some(fields)
}

/// Returns the position after the white space that starts at `at`.
fn skip_white(bytes: &[u8], mut at: usize) -> usize {
  while let Some(b' ' | b'\n' | b'\t' | b'\r') = bytes.get(at) {
    at += 1;
  }
  at
}

/// Returns the position after the JSON value that starts at `at`, inside `depth` arrays and
/// objects; `None` when no value that `serde_json` accepts starts there.
fn skip_value(bytes: &[u8], at: usize, depth: usize) -> Option<usize> {
  match bytes.get(at)? {
    b'"' => skip_string(bytes, at),
    open @ (b'{' | b'[') => {
      let depth = depth + 1;
      if depth >= NESTING_LIMIT {
        return None;
      }
      let (object, close) = if *open == b'{' { (true, b'}') } else { (false, b']') };
      let mut at = skip_white(bytes, at + 1);
      if bytes.get(at) == Some(&close) {
        return Some(at + 1);
      }
      loop {
        if object {
          at = skip_white(bytes, skip_key(bytes, at)?);
          if bytes.get(at) != Some(&b':') {
            return None;
          }
          at = skip_white(bytes, at + 1);
        }
        at = skip_white(bytes, skip_value(bytes, at, depth)?);
        match bytes.get(at)? {
          b',' => at = skip_white(bytes, at + 1),
          byte if *byte == close => return Some(at + 1),
          _ => return None,
        }
      }
    }
    b't' => bytes.get(at..at + 4).filter(|word| *word == b"true").map(|_| at + 4),
    b'f' => bytes.get(at..at + 5).filter(|word| *word == b"false").map(|_| at + 5),
    b'n' => bytes.get(at..at + 4).filter(|word| *word == b"null").map(|_| at + 4),
    b'-' | b'0'..=b'9' => skip_number(bytes, at),
    _ => None,
  }
}

/// Reads the JSON number that starts at `at` when it is written without an exponent and with
/// at most 18 digits after its leading zeros: returns the position after it and the number,
/// in the one pass over its digits. The number is the one that [number_value] gives for the
/// text, which has no warning for such a number: the same digits with the same scale. `None`
/// for every other text, a number or not.
fn plain_number(bytes: &[u8], mut at: usize) -> Option<(usize, FeelNumber)> {
  let negative = bytes.get(at) == Some(&b'-');
  at += usize::from(negative);
  let first = at;
  let mut coefficient: u64 = 0;
  // the digits after the leading zeros
  let mut significant = 0;
  while let Some(digit @ b'0'..=b'9') = bytes.get(at) {
    coefficient = coefficient.wrapping_mul(10).wrapping_add(u64::from(digit - b'0'));
    significant += usize::from(significant > 0 || *digit != b'0');
    at += 1;
  }
  // no digit follows a leading zero
  if at == first || (at - first > 1 && bytes[first] == b'0') {
    return None;
  }
  let mut scale = 0;
  if bytes.get(at) == Some(&b'.') {
    let point = at;
    at += 1;
    while let Some(digit @ b'0'..=b'9') = bytes.get(at) {
      coefficient = coefficient.wrapping_mul(10).wrapping_add(u64::from(digit - b'0'));
      significant += usize::from(significant > 0 || *digit != b'0');
      at += 1;
    }
    scale = at - point - 1;
    if scale == 0 {
      return None;
    }
  }
  // 18 digits fit in 63 bits, so nothing wrapped
  if significant > 18 || scale > 1000 || matches!(bytes.get(at), Some(b'e' | b'E')) {
    return None;
  }
  let coefficient = i64::try_from(coefficient).ok()?;
  Some((at, FeelNumber::new(if negative { -coefficient } else { coefficient }, scale as i32)))
}

/// Returns the position after the JSON number that starts at `at`.
fn skip_number(bytes: &[u8], mut at: usize) -> Option<usize> {
  let digits = |mut at: usize| {
    let start = at;
    while let Some(b'0'..=b'9') = bytes.get(at) {
      at += 1;
    }
    (at > start).then_some(at)
  };
  if bytes.get(at) == Some(&b'-') {
    at += 1;
  }
  at = match bytes.get(at)? {
    // no digit follows a leading zero
    b'0' => match bytes.get(at + 1) {
      Some(b'0'..=b'9') => return None,
      _ => at + 1,
    },
    b'1'..=b'9' => digits(at)?,
    _ => return None,
  };
  if bytes.get(at) == Some(&b'.') {
    at = digits(at + 1)?;
  }
  if let Some(b'e' | b'E') = bytes.get(at) {
    at += 1;
    if let Some(b'+' | b'-') = bytes.get(at) {
      at += 1;
    }
    at = digits(at)?;
  }
  Some(at)
}

/// Returns the position of the first byte from `at` on that a JSON string cannot hold as it is
/// (a quote, a backslash or a control character), or the length of `bytes`: eight bytes at a
/// time while none of them is one.
pub(crate) fn string_stop(bytes: &[u8], mut at: usize) -> usize {
  const ONES: u64 = 0x0101_0101_0101_0101;
  const HIGHS: u64 = 0x8080_8080_8080_8080;
  // the high bit of each byte of the result is set when the byte of `word` is zero
  let zero = |word: u64| word.wrapping_sub(ONES) & !word & HIGHS;
  while let Some(chunk) = bytes.get(at..at + 8) {
    let word = u64::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7]]);
    // a byte below 0x20, a quote or a backslash
    let stop = (word.wrapping_sub(ONES * 0x20) & !word & HIGHS) | zero(word ^ (ONES * b'"' as u64)) | zero(word ^ (ONES * b'\\' as u64));
    if stop != 0 {
      // a borrow can only set the bits of bytes above the lowest one that stops, so that one is exact
      return at + (stop.trailing_zeros() / 8) as usize;
    }
    at += 8;
  }
  while let Some(byte) = bytes.get(at) {
    if matches!(byte, b'"' | b'\\' | 0x00..=0x1F) {
      break;
    }
    at += 1;
  }
  at
}

/// Returns the position after the JSON string whose opening quote is at `at`: no control
/// character, every escape one of JSON's, and every `\u` surrogate one of a pair.
fn skip_string(bytes: &[u8], at: usize) -> Option<usize> {
  if bytes.get(at) != Some(&b'"') {
    return None;
  }
  let mut at = at + 1;
  loop {
    at = string_stop(bytes, at);
    match bytes.get(at)? {
      b'"' => return Some(at + 1),
      b'\\' => match bytes.get(at + 1)? {
        b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => at += 2,
        b'u' => match hex4(bytes, at + 2)? {
          0xDC00..=0xDFFF => return None,
          0xD800..=0xDBFF => {
            if bytes.get(at + 6) != Some(&b'\\') || bytes.get(at + 7) != Some(&b'u') || !matches!(hex4(bytes, at + 8)?, 0xDC00..=0xDFFF) {
              return None;
            }
            at += 12;
          }
          _ => at += 6,
        },
        _ => return None,
      },
      _ => return None,
    }
  }
}

/// Returns the position after the key of an object whose opening quote is at `at`; `None` also
/// for the [PRIVATE_NUMBER_KEY], however it is written: `serde_json` reads an object with it as
/// a number or refuses the text.
fn skip_key(bytes: &[u8], at: usize) -> Option<usize> {
  let end = skip_string(bytes, at)?;
  let key = bytes.get(at + 1..end - 1)?;
  if key.len() < PRIVATE_NUMBER_KEY.len() {
    return Some(end);
  }
  let private = if key.contains(&b'\\') {
    let text = std::str::from_utf8(bytes.get(at..end)?).ok()?;
    let mut warnings = vec![];
    let mut reader = Reader::new(text, 0, 0, &mut warnings);
    reader.read_string().ok()? == PRIVATE_NUMBER_KEY
  } else {
    key == PRIVATE_NUMBER_KEY.as_bytes()
  };
  (!private).then_some(end)
}

/// The value of the four hexadecimal digits at `at`.
fn hex4(bytes: &[u8], at: usize) -> Option<u32> {
  let mut value = 0;
  for byte in bytes.get(at..at + 4)? {
    value = value * 16 + (*byte as char).to_digit(16)?;
  }
  Some(value)
}

/// The keys of an object as they stand in the text, with what was found out about them. The
/// records of a list mostly have the same keys in the same order: the next object with these
/// keys takes the names (one text for all of them, not one per record) and the order, for one
/// comparison per key.
#[derive(Default)]
struct Shape {
  /// The names of the keys, in the order of the text.
  names: Vec<Name>,
  /// Whether the keys are in ascending order, each one once.
  ascending: bool,
  /// When they are not: for each place of the ascending order, the index of its entry in the
  /// order of the text; of the entries with one key only the last.
  order: Vec<usize>,
  /// The names in ascending order, each once: one list for all the objects with these keys.
  /// Empty for the outermost object, which takes no names.
  sorted: Option<std::sync::Arc<[Name]>>,
}

/// Reads JSON text into FEEL values. It accepts only JSON that `serde_json` accepts: what it
/// does not expect at a position, it refuses.
pub(crate) struct Reader<'a, 'w> {
  text: &'a str,
  at: usize,
  warnings: &'w mut Vec<String>,
  /// The entries of the objects that are being read, those of an inner object above those of
  /// the objects around it: one buffer for all objects of the text.
  entries: Vec<(Name, Value)>,
  /// The number of arrays and objects around the position, those of the request included.
  depth: usize,
  /// The keys of the object read last at each depth, see [Shape].
  shapes: Vec<Shape>,
  /// The depth inside the outermost array or object of the text.
  outermost: usize,
  /// The entries of an object while they are put into the order of their keys.
  unordered: Vec<Option<(Name, Value)>>,
  /// The values of the objects that are being read while their keys are those of the object
  /// before at their depth: without names, an inner object's above those around it.
  values: Vec<Value>,
  /// The values of such an object while they are put into the order of its keys.
  unordered_values: Vec<Option<Value>>,
}

impl<'a, 'w> Reader<'a, 'w> {
  /// A reader of `text` at the position `at`, inside `depth` arrays and objects.
  fn new(text: &'a str, at: usize, depth: usize, warnings: &'w mut Vec<String>) -> Self {
    Reader {
      text,
      at,
      warnings,
      entries: vec![],
      depth,
      shapes: vec![],
      outermost: depth + 1,
      unordered: vec![],
      values: vec![],
      unordered_values: vec![],
    }
  }

  /// Reads the JSON object that starts at `at` in the text of a request, a field of its
  /// top-level object, into a context; returns it with the position after the object. It reads
  /// only JSON that `serde_json` accepts there.
  pub(crate) fn object_at(text: &'a str, at: usize, path: &Path, warnings: &'w mut Vec<String>) -> Result<(FeelContext, usize), Refused> {
    let mut reader = Reader::new(text, at, 1, warnings);
    if reader.peek() != Some(b'{') {
      return Err(Refused);
    }
    let context = reader.read_object(path)?;
    Ok((context, reader.at))
  }

  /// Reads the JSON array or object that starts at `at` in the text of a request, a field of
  /// its top-level object, into a value; returns it with the position after it.
  pub(crate) fn value_at(text: &'a str, at: usize, path: &Path, warnings: &'w mut Vec<String>) -> Result<(Value, usize), Refused> {
    let mut reader = Reader::new(text, at, 1, warnings);
    let value = match reader.peek() {
      Some(b'{') => reader.read_object(path)?.into(),
      Some(b'[') => reader.read_array(path)?,
      _ => return Err(Refused),
    };
    Ok((value, reader.at))
  }

  /// Reads `text`, the whole text of a JSON object, into a context.
  #[cfg(test)]
  pub(crate) fn object(text: &'a str, path: &Path, warnings: &'w mut Vec<String>) -> Result<FeelContext, Refused> {
    let (context, end) = Reader::object_at(text, 0, path, warnings)?;
    if end == text.len() { Ok(context) } else { Err(Refused) }
  }

  fn peek(&self) -> Option<u8> {
    self.text.as_bytes().get(self.at).copied()
  }

  fn skip_white(&mut self) {
    self.at = skip_white(self.text.as_bytes(), self.at);
  }

  /// Passes the byte `byte`, after white space.
  fn expect(&mut self, byte: u8) -> Result<(), Refused> {
    self.skip_white();
    if self.peek() == Some(byte) {
      self.at += 1;
      Ok(())
    } else {
      Err(Refused)
    }
  }

  fn read_value(&mut self, path: &Path) -> Result<Value, Refused> {
    self.skip_white();
    let value = match self.peek().ok_or(Refused)? {
      b'{' => self.read_object(path)?.into(),
      b'[' => self.read_array(path)?,
      b'"' => Value::String(self.read_string()?.into_owned().into()),
      b't' => self.word("true", Value::Boolean(true))?,
      b'f' => self.word("false", Value::Boolean(false))?,
      b'n' => self.word("null", Value::Null(None))?,
      _ if let Some((end, number)) = plain_number(self.text.as_bytes(), self.at) => {
        self.at = end;
        Value::Number(number)
      }
      _ => {
        let end = skip_number(self.text.as_bytes(), self.at).ok_or(Refused)?;
        let number = self.text.get(self.at..end).ok_or(Refused)?;
        self.at = end;
        number_value(&exponent_as_serde_json(number), path, self.warnings).map_err(|_| Refused)?
      }
    };
    self.skip_white();
    Ok(value)
  }

  fn word(&mut self, word: &str, value: Value) -> Result<Value, Refused> {
    if self.text.get(self.at..).is_some_and(|rest| rest.starts_with(word)) {
      self.at += word.len();
      Ok(value)
    } else {
      Err(Refused)
    }
  }

  /// Passes the bracket that opens an array or an object, one level deeper.
  fn enter(&mut self) -> Result<(), Refused> {
    self.depth += 1;
    if self.depth >= NESTING_LIMIT {
      return Err(Refused);
    }
    self.at += 1;
    self.skip_white();
    Ok(())
  }

  /// Passes the bracket that closes an array or an object.
  fn leave(&mut self) {
    self.depth -= 1;
    self.at += 1;
  }

  fn read_array(&mut self, path: &Path) -> Result<Value, Refused> {
    self.enter()?;
    let mut items = vec![];
    if self.peek() == Some(b']') {
      self.leave();
      return Ok(Value::List(items.into()));
    }
    loop {
      items.push(self.read_value(&Path::Index(path, items.len()))?);
      match self.peek() {
        Some(b',') => self.at += 1,
        Some(b']') => {
          self.leave();
          return Ok(Value::List(items.into()));
        }
        _ => return Err(Refused),
      }
    }
  }

  fn read_object(&mut self, path: &Path) -> Result<FeelContext, Refused> {
    self.enter()?;
    if self.peek() == Some(b'}') {
      self.leave();
      return Ok(FeelContext::default());
    }
    let level = self.depth;
    if self.shapes.len() <= level {
      self.shapes.resize_with(level + 1, Shape::default);
    }
    let first_warning = self.warnings.len();
    let first_entry = self.entries.len();
    let first_value = self.values.len();
    // whether the keys so far are those of the object read before at this depth, in its order:
    // while they are, only the values are kept, and the names are those of that object
    let mut same = self.shapes[level].sorted.is_some();
    loop {
      self.skip_white();
      let key = self.read_string()?;
      // an empty key is an error, and a key with white space at an end is another name
      if key == PRIVATE_NUMBER_KEY || key.is_empty() || key.trim().len() != key.len() {
        return Err(Refused);
      }
      self.expect(b':')?;
      let value = self.read_value(&Path::Key(path, &key))?;
      if same {
        let index = self.values.len() - first_value;
        if self.shapes[level].names.get(index).is_some_and(|name| name.as_str() == key) {
          self.values.push(value);
        } else {
          same = false;
          self.name_values(level, first_value);
          self.entries.push((Name::from(&*key), value));
        }
      } else {
        self.entries.push((Name::from(&*key), value));
      }
      match self.peek() {
        Some(b',') => self.at += 1,
        Some(b'}') => {
          self.leave();
          break;
        }
        _ => return Err(Refused),
      }
    }
    if same {
      let shape = &self.shapes[level];
      if self.values.len() - first_value == shape.names.len()
        && let Some(names) = &shape.sorted
      {
        // the keys of the object before, all of them: the names are shared
        let values = if shape.ascending {
          self.values.drain(first_value..).collect()
        } else {
          // the warnings would have to follow the entries into the order of the keys
          if self.warnings.len() > first_warning {
            return Err(Refused);
          }
          self.unordered_values.clear();
          self.unordered_values.extend(self.values.drain(first_value..).map(Some));
          // every index is that of one entry, once
          let values = shape
            .order
            .iter()
            .filter_map(|index| self.unordered_values.get_mut(*index).and_then(Option::take))
            .collect();
          self.unordered_values.clear();
          values
        };
        return Ok(FeelContext::from_shared_names(names.clone(), values));
      }
      // fewer keys than the object before
      self.name_values(level, first_value);
    }
    self.remember_shape(level, first_entry);
    if !self.shapes[level].ascending {
      // the warnings would have to follow the entries into the order of the keys
      if self.warnings.len() > first_warning {
        return Err(Refused);
      }
      self.unordered.clear();
      self.unordered.extend(self.entries.drain(first_entry..).map(Some));
      for index in &self.shapes[level].order {
        // every index is that of one entry, once
        if let Some(entry) = self.unordered.get_mut(*index).and_then(Option::take) {
          self.entries.push(entry);
        }
      }
      self.unordered.clear();
    }
    // the entries are in ascending order of their names now, each name once
    if first_entry == 0 && self.entries.len() > DIRECT_ENTRIES {
      // the outermost object takes the buffer as it is, without a copy of its entries
      return Ok(FeelContext::from_ascending_entries(std::mem::take(&mut self.entries)));
    }
    // the first object with its keys at its depth shares the names with those after it
    if let Some(names) = &self.shapes[level].sorted
      && names.len() == self.entries.len() - first_entry
    {
      let values = self.entries.drain(first_entry..).map(|entry| entry.1).collect();
      return Ok(FeelContext::from_shared_names(names.clone(), values));
    }
    let entries = self.entries.drain(first_entry..);
    Ok(if entries.len() <= DIRECT_ENTRIES {
      let mut context = FeelContext::default();
      for (name, value) in entries {
        context.insert(name, value);
      }
      context
    } else {
      FeelContext::from_ascending_entries(entries)
    })
  }

  /// Moves the values from `first` on, which were kept without names, to the entries, each
  /// with the name of its place in the shape of `level`.
  fn name_values(&mut self, level: usize, first: usize) {
    let names = &self.shapes[level].names;
    self.entries.extend(names.iter().cloned().zip(self.values.drain(first..)));
  }

  /// Remembers the keys of the object whose entries start at `first` as the shape of `level`:
  /// their names, whether they are ascending, and the order that makes them so, which keeps
  /// the last of the entries with one key.
  fn remember_shape(&mut self, level: usize, first: usize) {
    let entries = &self.entries[first..];
    let shape = &mut self.shapes[level];
    shape.names.clear();
    // the outermost object comes once, so no object takes its names
    if level > self.outermost {
      shape.names.extend(entries.iter().map(|entry| entry.0.clone()));
    }
    shape.ascending = entries.windows(2).all(|pair| pair[0].0 < pair[1].0);
    shape.order.clear();
    if !shape.ascending {
      shape.order.extend(0..entries.len());
      // a stable sort: entries with one key stay in the order of the text
      shape.order.sort_by(|a, b| entries[*a].0.cmp(&entries[*b].0));
      let mut kept = 0;
      for place in 1..entries.len() {
        if entries[shape.order[kept]].0 != entries[shape.order[place]].0 {
          kept += 1;
        }
        shape.order[kept] = shape.order[place];
      }
      shape.order.truncate(kept + 1);
    }
    shape.sorted = (level > self.outermost).then(|| {
      if shape.ascending {
        entries.iter().map(|entry| entry.0.clone()).collect()
      } else {
        shape.order.iter().map(|index| entries[*index].0.clone()).collect()
      }
    });
  }

  /// Reads the string whose opening quote is at the position: its text as written when it has
  /// no escape.
  fn read_string(&mut self) -> Result<Cow<'a, str>, Refused> {
    let bytes = self.text.as_bytes();
    if bytes.get(self.at) != Some(&b'"') {
      return Err(Refused);
    }
    let start = self.at + 1;
    let mut at = string_stop(bytes, start);
    if bytes.get(at) == Some(&b'"') {
      self.at = at + 1;
      return self.text.get(start..at).map(Cow::Borrowed).ok_or(Refused);
    }
    // the text up to the first escape, then piece by piece
    let mut string = String::from(self.text.get(start..at).ok_or(Refused)?);
    loop {
      match bytes.get(at).ok_or(Refused)? {
        b'"' => {
          self.at = at + 1;
          return Ok(Cow::Owned(string));
        }
        b'\\' => {
          let escaped = match bytes.get(at + 1).ok_or(Refused)? {
            b'"' => '"',
            b'\\' => '\\',
            b'/' => '/',
            b'b' => '\u{8}',
            b'f' => '\u{c}',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'u' => {
              let mut code = hex4(bytes, at + 2).ok_or(Refused)?;
              at += 4;
              if let 0xD800..=0xDBFF = code {
                if bytes.get(at + 2) != Some(&b'\\') || bytes.get(at + 3) != Some(&b'u') {
                  return Err(Refused);
                }
                let low = hex4(bytes, at + 4).ok_or(Refused)?;
                if !matches!(low, 0xDC00..=0xDFFF) {
                  return Err(Refused);
                }
                code = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                at += 6;
              }
              char::from_u32(code).ok_or(Refused)?
            }
            _ => return Err(Refused),
          };
          string.push(escaped);
          at += 2;
        }
        _ => {
          let piece = at;
          at = string_stop(bytes, at);
          if at == piece {
            return Err(Refused);
          }
          string.push_str(self.text.get(piece..at).ok_or(Refused)?);
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::input::json_object_into_context;
  use serde_json::Value as Json;

  /// A small deterministic generator (xorshift64*).
  /// The second field: whether an object of the text written last has a key twice or a key
  /// with white space at an end.
  struct Rng(u64, bool);

  impl Rng {
    fn next(&mut self) -> u64 {
      self.0 ^= self.0 >> 12;
      self.0 ^= self.0 << 25;
      self.0 ^= self.0 >> 27;
      self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
      (self.next() % n as u64) as usize
    }

    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
      items[self.below(items.len())]
    }
  }

  const KEYS: [&str; 22] = [
    "a",
    "b",
    "c",
    "amount",
    "unit price",
    " a",
    "a ",
    "\\ta",
    "a\\n",
    "  ",
    "",
    "\\u0061",
    "k\\\"q",
    "\\ud83d\\ude00",
    "é",
    "z",
    "A",
    "a b",
    "0",
    "$serde_json::private::Number",
    "\\/",
    "x.y",
  ];
  const STRINGS: [&str; 12] = [
    "",
    "text",
    " spaced ",
    "\\n\\t\\r\\b\\f",
    "\\\"quoted\\\"",
    "\\\\",
    "\\u00e9\\u4e2d",
    "\\ud83d\\ude00",
    "é中😀",
    "\\/",
    "42",
    "a\\u0000b",
  ];
  const NUMBERS: [&str; 27] = [
    "0",
    "-0",
    "1",
    "-1",
    "42",
    "3.14",
    "-2.5e3",
    "1E5",
    "1e-5",
    "1e+5",
    "0.0",
    "0.10",
    "1234567890123456789012345678901234567890",
    "0.1234567890123456789012345678901234567",
    "1e-6200",
    "1e-6177",
    "1e6144",
    "1e6145",
    "1E-6200",
    "5E-6177",
    "1.5E-6176",
    "12345678901234567890123456789012345678E5",
    "12345678901234567890123456789012345678e5",
    "9.999999999999999999999999999999999e6144",
    "1e400000000",
    "-1e-7000",
    "123456789012345678901234567890123456.5",
  ];
  const WHITE: [&str; 5] = ["", "", " ", "\n\t", " \r\n "];

  /// Writes a random JSON value.
  fn value(rng: &mut Rng, depth: usize, out: &mut String) {
    let kinds = if depth >= 4 { 5 } else { 8 };
    match rng.below(kinds) {
      0 => out.push_str(rng.pick(&["true", "false"])),
      1 => out.push_str("null"),
      2 | 3 => out.push_str(rng.pick(&NUMBERS)),
      4 => {
        out.push('"');
        out.push_str(rng.pick(&STRINGS));
        out.push('"');
      }
      5 => {
        out.push('[');
        for index in 0..rng.below(4) {
          if index > 0 {
            out.push(',');
          }
          out.push_str(rng.pick(&WHITE));
          value(rng, depth + 1, out);
          out.push_str(rng.pick(&WHITE));
        }
        out.push(']');
      }
      _ => object(rng, depth + 1, out),
    }
  }

  /// Writes a random JSON object: keys in any order, repeated, with outer spaces and escapes.
  fn object(rng: &mut Rng, depth: usize, out: &mut String) {
    out.push('{');
    out.push_str(rng.pick(&WHITE));
    let entries = if rng.below(8) == 0 { 12 + rng.below(8) } else { rng.below(5) };
    let mut keys: Vec<String> = vec![];
    for index in 0..entries {
      if index > 0 {
        out.push(',');
      }
      out.push_str(rng.pick(&WHITE));
      out.push('"');
      // mostly ordinary keys, so that most objects convert
      let key = if rng.below(4) == 0 {
        rng.pick(&KEYS).to_string()
      } else {
        format!("k{}", rng.below(30))
      };
      // `a` and `\\u0061` are one key
      let same = |a: &str, b: &str| a == b || (a == "a" && b == "\\u0061") || (a == "\\u0061" && b == "a");
      rng.1 |= keys.iter().any(|other| same(other, &key)) || [" a", "a ", "\\ta", "a\\n"].contains(&key.as_str());
      out.push_str(&key);
      keys.push(key);
      out.push('"');
      out.push_str(rng.pick(&WHITE));
      out.push(':');
      out.push_str(rng.pick(&WHITE));
      value(rng, depth, out);
      out.push_str(rng.pick(&WHITE));
    }
    out.push('}');
  }

  /// Damages a text: one to three characters replaced, inserted or removed.
  fn damage(rng: &mut Rng, text: &str) -> String {
    let alphabet: Vec<char> = "{}[]\",:\\ \n\ttfn0123456789.-+eEua\u{1}é".chars().collect();
    let mut chars: Vec<char> = text.chars().collect();
    for _ in 0..1 + rng.below(3) {
      let at = rng.below(chars.len().max(1));
      match rng.below(3) {
        0 if !chars.is_empty() => chars[at] = alphabet[rng.below(alphabet.len())],
        1 if !chars.is_empty() => {
          chars.remove(at);
        }
        _ => chars.insert(at.min(chars.len()), alphabet[rng.below(alphabet.len())]),
      }
    }
    chars.into_iter().collect()
  }

  /// The context and the warnings of the conversion through the tree of JSON values, or its error.
  fn through_the_tree(text: &str) -> Result<(FeelContext, Vec<String>), String> {
    let Json::Object(map) = serde_json::from_str(text).map_err(|e| e.to_string())? else {
      return Err("no object".to_string());
    };
    let mut warnings = vec![];
    json_object_into_context(map, &Path::Root(""), &mut warnings).map(|context| (context, warnings))
  }

  fn cases() -> usize {
    if std::env::var_os("RHEON_FULL_TESTS").is_some() { 400_000 } else { 20_000 }
  }

  /// What the reader reads is what the conversion through the tree reads, with the same warnings
  /// in the same order; it refuses what that conversion refuses, and beyond that only an object
  /// with the private number key, with a key twice (whose first value may not convert), with a
  /// key that has white space at an end, or with a warning.
  #[test]
  fn reader_equals_the_conversion_through_the_tree() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15, false);
    let (mut read, mut refused, mut with_warnings) = (0, 0, 0);
    for case in 0..cases() {
      let mut text = String::new();
      rng.1 = false;
      object(&mut rng, 0, &mut text);
      // a third of the texts are damaged: the reader reads only what is JSON
      let damaged = rng.below(3) == 0;
      if damaged {
        text = damage(&mut rng, &text);
      }
      let mut warnings = vec!["earlier".to_string()];
      let fast = Reader::object(&text, &Path::Root(""), &mut warnings);
      match (fast, through_the_tree(&text)) {
        (Ok(context), Ok((expected, expected_warnings))) => {
          assert_eq!(expected, context, "case {case}: {text}");
          assert_eq!(format!("{expected:?}"), format!("{context:?}"), "case {case}: {text}");
          assert_eq!(expected_warnings, warnings[1..], "case {case}: {text}");
          read += 1;
          with_warnings += usize::from(!expected_warnings.is_empty());
        }
        (Ok(context), Err(error)) => panic!("case {case}: {text}\nread {context:?}\nthe tree refuses: {error}"),
        (Err(Refused), Ok((_, expected_warnings))) => {
          // an entry that a later one with its key replaces is read here and not in the tree
          assert!(
            damaged || text.contains(PRIVATE_NUMBER_KEY) || rng.1 || !expected_warnings.is_empty(),
            "case {case}: refused {text}"
          );
          refused += 1;
        }
        (Err(Refused), Err(_)) => refused += 1,
      }
    }
    println!("{read} read ({with_warnings} with warnings), {refused} refused");
    assert!(
      read > cases() / 4 && refused > cases() / 20 && with_warnings > cases() / 40,
      "{read} read, {with_warnings} with warnings, {refused} refused"
    );
  }

  /// Records whose keys repeat in an order that is not ascending are read as through the tree:
  /// the order of the record before is taken when it fits, and not when the keys, their order
  /// or their number differ, or a key comes twice.
  #[test]
  fn records_with_repeating_key_orders_equal_the_conversion_through_the_tree() {
    let mut rng = Rng(0x0123_4567_89AB_CDEF, false);
    let orders: [&[&str]; 9] = [
      &["c", "a", "b"],
      &["c", "b", "a"],
      &["b", "a", "c"],
      &["a", "b", "c"],
      &["c", "a", "b", "d"],
      &["c", "a", "x"],
      &["c", "a", "a"],
      &["b", "a", "b"],
      &["zeta", "alpha", "mid", "beta", "omega", "delta", "k1", "k10", "k2", "aa", "ab", "b", "a", "y"],
    ];
    for case in 0..cases() / 100 {
      let mut text = String::from("{\"z\":1,\"rows\":[");
      for record in 0..2 + rng.below(40) {
        if record > 0 {
          text.push(',');
        }
        // mostly the order of the record before
        let order = if record > 0 && rng.below(4) > 0 {
          orders[rng.0 as usize % 3]
        } else {
          orders[rng.below(orders.len())]
        };
        text.push('{');
        for (index, key) in order.iter().enumerate() {
          if index > 0 {
            text.push(',');
          }
          text.push_str(&format!("\"{key}\":{}", rng.below(1000)));
        }
        text.push('}');
      }
      text.push_str("],\"a\":{\"q\":1,\"p\":2}}");
      let mut warnings = vec![];
      let read = Reader::object(&text, &Path::Root(""), &mut warnings).unwrap_or_else(|_| panic!("case {case}: refused {text}"));
      let (expected, _) = through_the_tree(&text).unwrap();
      assert_eq!(expected, read, "case {case}: {text}");
      assert_eq!(format!("{expected:?}"), format!("{read:?}"), "case {case}: {text}");
    }
  }

  /// [fields] takes a text only when `serde_json` reads it as an object, and gives the keys and
  /// the values of that object; it takes every object without an escape in a top-level key.
  #[test]
  fn fields_are_those_of_serde_json() {
    let mut rng = Rng(0x2545_F491_4F6C_DD1D, false);
    let alphabet: Vec<char> = "{}[]\",:\\ \n\ttfn0123456789.-+eEua\u{1}é".chars().collect();
    let (mut taken, mut refused, mut requests) = (0, 0, 0);
    for case in 0..cases() {
      let mut text = String::from(rng.pick(&WHITE));
      object(&mut rng, 0, &mut text);
      text.push_str(rng.pick(&WHITE));
      // half of the texts are damaged: characters replaced, inserted or removed
      let damaged = rng.below(2) == 0;
      if damaged {
        let mut chars: Vec<char> = text.chars().collect();
        for _ in 0..1 + rng.below(3) {
          let at = rng.below(chars.len().max(1));
          match rng.below(3) {
            0 if !chars.is_empty() => chars[at] = alphabet[rng.below(alphabet.len())],
            1 if !chars.is_empty() => {
              chars.remove(at);
            }
            _ => chars.insert(at.min(chars.len()), alphabet[rng.below(alphabet.len())]),
          }
        }
        text = chars.into_iter().collect();
      }
      // as the context and the input of a request, which are read where they stand
      let wrapped = format!("{{\"op\":\"evaluate\",\"context\":{text},\"input\":[{text}, 1]}}");
      if let Some(read) = request(&wrapped) {
        let Ok(Json::Object(map)) = serde_json::from_str::<Json>(&wrapped) else {
          panic!("case {case}: took the request {wrapped:?}");
        };
        // the fields that were not read into values are those of the tree
        assert!(read.fields.iter().all(|(key, _)| map.contains_key(*key)), "case {case}: {wrapped:?}");
        requests += 1;
      }
      let tree: Result<Json, _> = serde_json::from_str(&text);
      match fields(&text) {
        Some(fields) => {
          let Ok(Json::Object(map)) = &tree else {
            panic!("case {case}: took {text:?}, serde_json: {tree:?}");
          };
          // the last field of a key is the entry of the tree
          for (key, value) in &fields {
            let last = fields.iter().rev().find(|(other, _)| other == key).map(|(_, value)| *value);
            assert_eq!(map.get(*key), serde_json::from_str::<Json>(last.unwrap()).ok().as_ref(), "case {case}: {key} in {text:?}");
            assert!(serde_json::from_str::<Json>(value).is_ok(), "case {case}: {value:?} in {text:?}");
          }
          assert_eq!(
            map.len(),
            fields.iter().map(|(key, _)| key).collect::<std::collections::BTreeSet<_>>().len(),
            "case {case}: {text:?}"
          );
          taken += 1;
        }
        None => {
          if let Ok(Json::Object(_)) = &tree {
            // refused though it is an object: for an escape in a top-level key, or the private number key
            assert!(text.contains('\\') || text.contains(PRIVATE_NUMBER_KEY), "case {case}: refused {text:?}");
          }
          refused += 1;
        }
      }
    }
    println!("{taken} taken, {refused} refused, {requests} requests taken");
    assert!(requests > cases() / 4, "{requests} requests taken");
    assert!(taken > cases() / 4 && refused > cases() / 10, "{taken} taken, {refused} refused");
  }

  /// [string_stop] finds the first quote, backslash or control character, wherever it stands in
  /// the eight bytes that are looked at together, and whatever stands around it.
  /// A number read in one pass is the number of its text, digit for digit, and no number is
  /// read from a text that is none or that the pass does not take.
  #[test]
  fn plain_numbers_are_the_numbers_of_their_texts() {
    let mut texts: Vec<String> = [
      "0",
      "-0",
      "1",
      "-1",
      "10",
      "0.0",
      "-0.0",
      "0.5",
      "-0.50",
      "1.000",
      "12.5",
      "100",
      "0.001",
      "0.100",
      "123456789012345678",
      "-123456789012345678",
      "999999999999999999",
      "0.123456789012345678",
      "1234567890.12345678",
      "0.000000000000000000000000000001",
      "9223372036854775807",
      // not taken: 19 digits and more, an exponent, and what is no JSON number
      "1234567890123456789",
      "0.1234567890123456789",
      "18446744073709551616",
      "99999999999999999999999999999999999999",
      "1e5",
      "1E5",
      "1.5e-3",
      "0e0",
      "01",
      "-01",
      "00",
      "1.",
      ".5",
      "-",
      "",
      "-.5",
      "+1",
      "1.e1",
      "a",
    ]
    .iter()
    .map(|text| text.to_string())
    .collect();
    let mut rng = Rng(0x2545_F491_4F6C_DD1D, false);
    for _ in 0..20_000 {
      let mut text = String::new();
      if rng.below(4) == 0 {
        text.push('-');
      }
      let whole = if rng.below(8) == 0 { 22 } else { 6 };
      for _ in 0..1 + rng.below(whole) {
        text.push(char::from(b'0' + rng.below(10) as u8));
      }
      if rng.below(2) == 0 {
        text.push('.');
        let fraction = if rng.below(8) == 0 { 22 } else { 5 };
        for _ in 0..rng.below(fraction) {
          text.push(char::from(b'0' + rng.below(10) as u8));
        }
      }
      texts.push(text);
    }
    let (mut taken, mut left) = (0, 0);
    for text in texts {
      // a number ends before what follows it
      for after in ["", ",", "}", " ", "x"] {
        let padded = format!("{text}{after}");
        let read = plain_number(padded.as_bytes(), 0);
        let is_number = skip_number(text.as_bytes(), 0) == Some(text.len());
        match read {
          Some((end, number)) => {
            assert!(is_number, "{text:?} is no JSON number");
            assert_eq!(text.len(), end, "{padded:?}");
            let mut warnings = vec![];
            let expected = number_value(&text, &Path::Root(""), &mut warnings).unwrap_or_else(|e| panic!("{text}: {e}"));
            assert!(warnings.is_empty(), "{text}: {warnings:?}");
            // the same digits and the same scale, not only an equal number
            assert_eq!(format!("{expected:?}"), format!("{:?}", Value::Number(number)), "{text}");
            taken += 1;
          }
          None => left += 1,
        }
      }
    }
    assert!(taken > 50_000 && left > 5_000, "{taken} taken, {left} left");
  }

  #[test]
  fn string_stop_is_the_first_special_byte() {
    let mut rng = Rng(0x1234_5678_9ABC_DEF1, false);
    let pool = [
      b'a', b'z', b' ', b'!', b'#', b'[', b']', 0x7F, 0x80, 0xC3, 0xFF, 0x20, 0x21, 0x23, 0x5B, 0x5D, b'"', b'\\', 0x00, 0x01, 0x1F, 0x0A,
    ];
    for case in 0..cases() {
      let bytes: Vec<u8> = (0..rng.below(40))
        .map(|_| {
          let among = if rng.below(3) == 0 { pool.len() } else { 16 };
          pool[rng.below(among)]
        })
        .collect();
      let from = rng.below(bytes.len() + 1);
      let expected = (from..bytes.len()).find(|at| matches!(bytes[*at], b'"' | b'\\' | 0x00..=0x1F)).unwrap_or(bytes.len());
      assert_eq!(expected, string_stop(&bytes, from), "case {case}: {bytes:?} from {from}");
    }
    assert_eq!(3, string_stop(b"abc", 3));
    assert_eq!(5, string_stop(b"abc", 5));
  }

  /// The nesting limit is that of `serde_json`: 127 arrays or objects inside each other are
  /// read, 128 are not.
  #[test]
  fn nesting_limit_is_that_of_serde_json() {
    for depth in 120..135 {
      for (open, close) in [("[", "]"), ("{\"a\":", "}")] {
        let text = format!("{{\"context\":{}1{}}}", open.repeat(depth), close.repeat(depth));
        let tree: Result<Json, _> = serde_json::from_str(&text);
        assert_eq!(tree.is_ok(), fields(&text).is_some(), "depth {depth} of {open}");
        // the reader of the context counts the same levels
        assert_eq!(tree.is_ok(), request(&text).is_some(), "depth {depth} of {open}");
        let input = text.replace("context", "input");
        assert_eq!(tree.is_ok(), request(&input).is_some(), "depth {depth} of {open}");
        if let Some(read) = request(&text) {
          assert!(matches!(read.context, Some(Field::Read(..)) | Some(Field::Text(_))), "depth {depth} of {open}");
          assert_eq!(open == "{\"a\":", matches!(read.context, Some(Field::Read(..))), "depth {depth} of {open}");
        }
        assert_eq!(depth + 1 < NESTING_LIMIT, tree.is_ok(), "depth {depth} of {open}");
      }
    }
  }

  /// Texts that are no JSON object, each refused as `serde_json` refuses it or reads another value.
  #[test]
  fn fields_refuses_what_is_no_object() {
    for text in [
      "",
      " ",
      "null",
      "[]",
      "1",
      "\"a\"",
      "{",
      "}",
      "{}x",
      "{} {}",
      "{\"a\"}",
      "{\"a\":}",
      "{\"a\":1,}",
      "{,}",
      "{\"a\":1 \"b\":2}",
      "{a:1}",
      "{'a':1}",
      "{\"a\":01}",
      "{\"a\":1.}",
      "{\"a\":.5}",
      "{\"a\":1e}",
      "{\"a\":+1}",
      "{\"a\":-}",
      "{\"a\":0x10}",
      "{\"a\":NaN}",
      "{\"a\":Infinity}",
      "{\"a\":tru}",
      "{\"a\":True}",
      "{\"a\":nul}",
      "{\"a\":\"\\x\"}",
      "{\"a\":\"\\u12\"}",
      "{\"a\":\"\\ud800\"}",
      "{\"a\":\"\\ud800\\u0041\"}",
      "{\"a\":\"\\udc00\"}",
      "{\"a\":\"\\ud800x\"}",
      "{\"a\":\"\t\"}",
      "{\"a\":\"\n\"}",
      "{\"a\":\"\u{0}\"}",
      "{\"a\":\"unclosed}",
      "{\"a\":[1,]}",
      "{\"a\":[1 2]}",
      "{\"a\":[}",
      "{\"a\":{]}",
      "\u{FEFF}{}",
      "{}\u{a0}",
      "{\"a\":1}//",
      "{/**/}",
      "{\"a\":1,\"a\"}",
      "{\"\\u00\":1}",
    ] {
      assert!(fields(text).is_none(), "{text:?}");
      assert!(!matches!(serde_json::from_str::<Json>(text), Ok(Json::Object(_))), "{text:?}");
    }
    for (text, count) in [
      ("{}", 0),
      (" { } ", 0),
      ("{\"a\":1}", 1),
      ("\n{ \"a\" : [ ] , \"b\" : { } }\r\n", 2),
      ("{\"a\":1,\"a\":2}", 2),
      ("{\"\":0}", 1),
      ("{\"a\":\"\\ud83d\\ude00\\u00e9\\/\"}", 1),
    ] {
      assert_eq!(Some(count), fields(text).map(|fields| fields.len()), "{text:?}");
    }
    assert!(fields("{\"\\u0061\":1}").is_none());
  }
}
