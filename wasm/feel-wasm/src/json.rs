//! A strict reader of the request text, which converts the JSON of `context` and `input` into
//! FEEL values in one pass, without a tree of JSON values between the text and the values.
//!
//! The reader answers only what it is sure of. [fields] accepts a text only when it is a JSON
//! object as `serde_json::from_str` accepts it (the same grammar, escapes and nesting limit), and
//! [Reader] converts only what converts without an error. Whatever they refuse (`None`,
//! [Refused]) is answered by the path through the tree of JSON values, as before, so every
//! error and its text come from there.
//!
//! That tree is built here as well ([tree]), and not by `serde_json`, whose own tree is not the
//! JSON of every text: built with `arbitrary_precision`, it hands a number to its tree as an
//! object with the one key `$serde_json::private::Number`, and so reads an object of the text
//! that has this key first as a number, or refuses the text. Here such an object is an object
//! like every other. `serde_json` still gives the error of a text that is no JSON.
//!
//! The values and the warnings are those of [crate::input::json_into_value] for the same text:
//! - of two entries of an object with the same key, the later one in the text gives the value;
//! - the warnings of the entries of an object come in the order of its keys, not of the text.
//!
//! An object that is an item of an array is not read at once when [plain_end] passes it, which
//! validates it and shows that it is read without a warning: it becomes a context that reads
//! the object when the context is first used ([Record]). What is read then is what would have
//! been read at once, so the values, the warnings and what is refused are the same.
//!
//! The reader refuses what it would have to work for to answer the same: a key with white space
//! at an end (the name is the key without it, and of two such keys the later one in the order of
//! the keys gives the entry), and an object whose keys do not come in ascending order when one
//! of its values has a warning.

use crate::input::Warning;
use crate::input::{DIRECT_ENTRIES, Path, number_value};
use rheon_feel::Name;
use rheon_feel::context::{DeferredEntries, FeelContext};
use rheon_feel::values::Value;
use rheon_feel_number::FeelNumber;
use serde_json::Value as Json;
use std::borrow::Cow;
use std::sync::{Arc, Mutex};

/// An object that is an item of an array is read when it is first used, when its text has at
/// least this many bytes ([Record]).
const DEFER_FROM: usize = 128;

/// Returns the position after the JSON value that starts at `at`, inside `depth` arrays and
/// objects, and the nesting depth of the value it is read into; `None` unless the value is
/// one that [Reader] reads into a value without a warning and without refusing it, whatever
/// stands around it. It answers `None` for more than it has to: for a string with an escape,
/// a number with an exponent or more than 34 digits, and a key that does not start and end
/// with a visible ASCII character, and an object that has a key twice when
/// the value of one of the two is an array or an object (the depth of the object is that of
/// the entries it keeps), or that has too many keys with such values to compare them.
///
/// Every object passed on the way that is an item of an array is added to the items of
/// `passed`, with its start, its end and its nesting depth: also when the value around it is
/// not passed.
fn plain_end(bytes: &[u8], at: usize, depth: usize, passed: &mut Passed) -> Option<(usize, u32)> {
  match bytes.get(at)? {
    b'"' => {
      let end = string_stop(bytes, at + 1);
      (bytes.get(end) == Some(&b'"')).then_some((end + 1, 0))
    }
    open @ (b'{' | b'[') => {
      let depth = depth + 1;
      if depth >= NESTING_LIMIT {
        return None;
      }
      let (object, close) = if *open == b'{' { (true, b'}') } else { (false, b']') };
      let mut at = skip_white(bytes, at + 1);
      if bytes.get(at) == Some(&close) {
        return Some((at + 1, 1));
      }
      let mut deepest = 0;
      let first_key = passed.keys.len();
      // the keys whose values are arrays or objects
      let mut nested = 0;
      loop {
        let key = at + 1;
        if object {
          if bytes.get(at) != Some(&b'"') {
            return None;
          }
          let end = string_stop(bytes, at + 1);
          let visible = |byte: &u8| (0x21..=0x7E).contains(byte);
          if bytes.get(end) != Some(&b'"') || end == at + 1 || !visible(&bytes[at + 1]) || !visible(&bytes[end - 1]) {
            return None;
          }
          passed.keys.push((key, end, false));
          at = skip_white(bytes, end + 1);
          if bytes.get(at) != Some(&b':') {
            return None;
          }
          at = skip_white(bytes, at + 1);
        }
        let (end, nesting) = plain_end(bytes, at, depth, passed)?;
        if object {
          if nesting > 0 {
            nested += 1;
            // the keys of the values inside are gone again: the last one is this one
            if let Some(key) = passed.keys.last_mut() {
              key.2 = true;
            }
          }
        } else if bytes[at] == b'{' {
          passed.items.push((at, end, nesting));
        }
        deepest = deepest.max(nesting);
        at = skip_white(bytes, end);
        match bytes.get(at)? {
          b',' => at = skip_white(bytes, at + 1),
          byte if *byte == close => {
            if object {
              let keys = &passed.keys[first_key..];
              // each key of an array or an object against every other key
              let twice = nested > 0
                && (nested * keys.len() > KEY_COMPARISONS
                  || keys.iter().enumerate().filter(|(_, key)| key.2).any(|(index, key)| {
                    keys
                      .iter()
                      .enumerate()
                      .any(|(other, entry)| other != index && bytes[entry.0..entry.1] == bytes[key.0..key.1])
                  }));
              passed.keys.truncate(first_key);
              if twice {
                return None;
              }
            }
            return Some((at + 1, deepest + 1));
          }
          _ => return None,
        }
      }
    }
    b't' => bytes.get(at..at + 4).filter(|word| *word == b"true").map(|_| (at + 4, 0)),
    b'f' => bytes.get(at..at + 5).filter(|word| *word == b"false").map(|_| (at + 5, 0)),
    b'n' => bytes.get(at..at + 4).filter(|word| *word == b"null").map(|_| (at + 4, 0)),
    b'-' | b'0'..=b'9' => {
      // a number without an exponent: `-`, digits without a leading zero, `.` and digits
      let mut end = at + usize::from(bytes[at] == b'-');
      let first = end;
      while let Some(b'0'..=b'9') = bytes.get(end) {
        end += 1;
      }
      let whole = end - first;
      if whole == 0 || (whole > 1 && bytes[first] == b'0') {
        return None;
      }
      let mut digits = whole;
      if bytes.get(end) == Some(&b'.') {
        let point = end;
        end += 1;
        while let Some(b'0'..=b'9') = bytes.get(end) {
          end += 1;
        }
        digits += end - point - 1;
        if end == point + 1 {
          return None;
        }
      }
      (digits <= 34 && !matches!(bytes.get(end), Some(b'e' | b'E'))).then_some((end, 0))
    }
    _ => None,
  }
}

/// What [plain_end] keeps of a pass.
#[derive(Default)]
struct Passed {
  /// The objects that are items of arrays: the start, the end and the nesting depth of each.
  items: Vec<(usize, usize, u32)>,
  /// The keys of the objects that are being passed, those of an inner object above those of
  /// the objects around it: the start and the end of each, and whether its value is an array
  /// or an object.
  keys: Vec<(usize, usize, bool)>,
}

/// The most comparisons of keys that [plain_end] makes for one object, to see that no key
/// with an array or an object as its value comes twice.
const KEY_COMPARISONS: usize = 4096;

/// An object of the request text that is read when the context it stands for is first used:
/// one that [plain_end] passed, so the reader reads it without a warning.
struct Record {
  shared: Arc<Shared>,
  at: usize,
  /// The number of arrays and objects around the object.
  depth: usize,
}

/// What the records of a text that are read later have in common.
struct Shared {
  text: Arc<str>,
  /// The shapes of the records read so far.
  shapes: Mutex<Vec<Shape>>,
  /// The objects that [plain_end] passed as items of arrays inside other values: for the start
  /// of each, its end and its nesting depth. The reader of the value around one finds it here.
  passed: Mutex<std::collections::HashMap<usize, (usize, u32)>>,
}

impl DeferredEntries for Record {
  fn build(&self) -> FeelContext {
    let mut warnings = vec![];
    let mut reader = Reader::new(&self.shared.text, self.at, self.depth, &mut warnings);
    reader.shared = Some(self.shared.clone());
    // no object of this reader comes once: every one takes the names of the one before
    reader.outermost = 0;
    if let Ok(mut shapes) = self.shared.shapes.lock() {
      reader.shapes = std::mem::take(&mut *shapes);
    }
    let context = reader.read_object(&Path::Root(""));
    if let Ok(mut shapes) = self.shared.shapes.lock() {
      *shapes = std::mem::take(&mut reader.shapes);
    }
    // [plain_end] passed the object, so it is read
    context.unwrap_or_else(|Refused| unreachable!("an object that was passed as plain is read"))
  }
}

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

/// The reader does not answer this text.
#[derive(Debug)]
pub(crate) struct Refused;

/// What the reader made of the JSON of `context` or `input`.
pub(crate) enum Field<'a, T> {
  /// The value and the warnings of reading it.
  Read(T, Vec<Warning>),
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
  /// The text of `entries` when it is an array: its items are read one by one ([elements]).
  pub(crate) entries: Option<&'a str>,
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
    entries: None,
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
      "entries" if bytes.get(at) == Some(&b'[') => {
        let end = skip_value(bytes, at, 1)?;
        request.entries = Some(text.get(at..end)?);
        Some((end, false))
      }
      _ => {
        // a later field of the key that is no object or array counts instead of an earlier one
        match key {
          "context" => request.context = None,
          "input" => request.input = None,
          "entries" => request.entries = None,
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
  if request.entries.is_some() {
    request.fields.retain(|(key, _)| *key != "entries");
  }
  Some(request)
}

/// The texts of the items of the JSON array `text`, which is a field of a request that
/// [request] took: `None` when the text is no such array.
pub(crate) fn elements(text: &str) -> Option<Vec<&str>> {
  let bytes = text.as_bytes();
  let mut at = skip_white(bytes, 0);
  if bytes.get(at) != Some(&b'[') {
    return None;
  }
  at = skip_white(bytes, at + 1);
  let mut items = vec![];
  if bytes.get(at) == Some(&b']') {
    return Some(items);
  }
  loop {
    // inside the object of the request and the array
    let end = skip_value(bytes, at, 2)?;
    items.push(text.get(at..end)?);
    at = skip_white(bytes, end);
    match bytes.get(at) {
      Some(b',') => at = skip_white(bytes, at + 1),
      Some(b']') => return Some(items),
      _ => return None,
    }
  }
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
      let key_end = skip_string(bytes, at)?;
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

/// The tree of JSON values of `text`: the one that `serde_json` builds for it, except that an
/// object with the key by which `serde_json` hands a number to its tree is an object. A text
/// that is no JSON has the error of `serde_json`.
pub(crate) fn tree(text: &str) -> Result<Json, serde_json::Error> {
  let mut warnings = vec![];
  let mut reader = Reader::new(text, 0, 0, &mut warnings);
  reader.skip_white();
  if let Ok(tree) = reader.read_tree()
    && reader.at == text.len()
  {
    return Ok(tree);
  }
  // no JSON: the error is that of `serde_json`, which reads the text without building its tree
  serde_json::from_str::<Checked>(text).and_then(|Checked| serde_json::from_str(text))
}

/// The order in which the keys of the objects of a JSON text are written: what a tree of JSON
/// values, which keeps the keys of an object in their own order, does not hold. A context that
/// is converted from the tree is told it ([Order::of]).
pub(crate) enum Order {
  /// A value that is no array or object.
  Other,
  /// The items of an array.
  Array(Vec<Order>),
  /// The names of the keys of an object (a key without the white space at its ends), each
  /// once at its first place in the text, with the value that the last entry of the name has.
  Object(Vec<(Name, Order)>),
}

impl Order {
  /// The order of the keys in `text`, a JSON value; `None` when it is no JSON.
  pub(crate) fn read(text: &str) -> Option<Order> {
    let mut warnings = vec![];
    let mut reader = Reader::new(text, 0, 0, &mut warnings);
    reader.skip_white();
    let order = reader.read_order().ok()?;
    (reader.at == text.len()).then_some(order)
  }

  /// The order under the key `key` of an object.
  pub(crate) fn field(self, key: &str) -> Option<Order> {
    match self {
      Order::Object(entries) => entries.into_iter().find(|(name, _)| name.as_str() == key).map(|(_, order)| order),
      _ => None,
    }
  }

  /// `value`, converted from the tree of the text of this order, with every context in it
  /// told the order its entries are written in. Where the value is not of the shape of the
  /// order, it stays as it is.
  pub(crate) fn of(&self, value: Value) -> Value {
    match (self, value) {
      (Order::Array(orders), Value::List(items)) if orders.len() == items.len() => Value::List(items.iter().zip(orders).map(|(item, order)| order.of(item.clone())).collect()),
      (Order::Object(_), Value::Context(context)) => self.of_context(Arc::unwrap_or_clone(context)).into(),
      (_, value) => value,
    }
  }

  /// [Order::of] for a context.
  pub(crate) fn of_context(&self, mut context: FeelContext) -> FeelContext {
    let Order::Object(entries) = self else {
      return context;
    };
    for (name, order) in entries {
      if !matches!(order, Order::Other)
        && let Some(value) = context.get_entry(name).cloned()
      {
        context.set_entry(name, order.of(value));
      }
    }
    context.set_written_names(entries.iter().map(|(name, _)| name.clone()).collect());
    context
  }
}

/// A JSON value that is read and not kept: what `serde_json` reads as its tree, an object with
/// its private number key included.
struct Checked;

impl<'de> serde::Deserialize<'de> for Checked {
  fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
      type Value = Checked;

      fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("any valid JSON value")
      }

      fn visit_bool<E>(self, _: bool) -> Result<Checked, E> {
        Ok(Checked)
      }

      // a short integer; every other number comes as an object with the private key
      fn visit_i64<E>(self, _: i64) -> Result<Checked, E> {
        Ok(Checked)
      }

      fn visit_u64<E>(self, _: u64) -> Result<Checked, E> {
        Ok(Checked)
      }

      fn visit_f64<E>(self, _: f64) -> Result<Checked, E> {
        Ok(Checked)
      }

      fn visit_str<E>(self, _: &str) -> Result<Checked, E> {
        Ok(Checked)
      }

      fn visit_unit<E>(self) -> Result<Checked, E> {
        Ok(Checked)
      }

      fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut items: A) -> Result<Checked, A::Error> {
        while let Some(Checked) = items.next_element()? {}
        Ok(Checked)
      }

      // an object, and a number: `serde_json` hands one on as an object with its private key
      fn visit_map<A: serde::de::MapAccess<'de>>(self, mut entries: A) -> Result<Checked, A::Error> {
        while let Some((_, Checked)) = entries.next_entry::<String, Checked>()? {}
        Ok(Checked)
      }
    }
    deserializer.deserialize_any(Visitor)
  }
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
          at = skip_white(bytes, skip_string(bytes, at)?);
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
  /// When the keys are not ascending: the order they are written in, each key once at its
  /// first place in the text, as its place in the ascending order. The context shows its
  /// entries in this order; one list for all the objects with these keys.
  written: Option<std::sync::Arc<[u32]>>,
  /// Whether a key is written in another form than FEEL reads a name (see
  /// [FeelContext::note_key_forms]).
  other_forms: bool,
}

/// Reads JSON text into FEEL values. It accepts only JSON that `serde_json` accepts: what it
/// does not expect at a position, it refuses.
pub(crate) struct Reader<'a, 'w> {
  text: &'a str,
  at: usize,
  warnings: &'w mut Vec<Warning>,
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
  /// The text as the records that are read later hold it, with their shapes; made for the
  /// first of them.
  shared: Option<Arc<Shared>>,
}

impl<'a, 'w> Reader<'a, 'w> {
  /// A reader of `text` at the position `at`, inside `depth` arrays and objects.
  fn new(text: &'a str, at: usize, depth: usize, warnings: &'w mut Vec<Warning>) -> Self {
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
      shared: None,
    }
  }

  /// Reads the JSON object that starts at `at` in the text of a request, a field of its
  /// top-level object, into a context; returns it with the position after the object. It reads
  /// only JSON that `serde_json` accepts there.
  pub(crate) fn object_at(text: &'a str, at: usize, path: &Path, warnings: &'w mut Vec<Warning>) -> Result<(FeelContext, usize), Refused> {
    let mut reader = Reader::new(text, at, 1, warnings);
    if reader.peek() != Some(b'{') {
      return Err(Refused);
    }
    let context = reader.read_object(path)?;
    Ok((context, reader.at))
  }

  /// Reads the JSON array or object that starts at `at` in the text of a request, a field of
  /// its top-level object, into a value; returns it with the position after it.
  pub(crate) fn value_at(text: &'a str, at: usize, path: &Path, warnings: &'w mut Vec<Warning>) -> Result<(Value, usize), Refused> {
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
  pub(crate) fn object(text: &'a str, path: &Path, warnings: &'w mut Vec<Warning>) -> Result<FeelContext, Refused> {
    let (context, end) = Reader::object_at(text, 0, path, warnings)?;
    if end == text.len() { Ok(context) } else { Err(Refused) }
  }

  /// What the records that are read later have in common, made for the first of them.
  fn shared(&mut self) -> &Arc<Shared> {
    self.shared.get_or_insert_with(|| {
      Arc::new(Shared {
        text: Arc::from(self.text),
        shapes: Mutex::default(),
        passed: Mutex::default(),
      })
    })
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

  /// Reads the JSON value at the position into a tree of JSON values, and passes the white
  /// space after it. A number keeps its text, as `serde_json` hands it on.
  fn read_tree(&mut self) -> Result<Json, Refused> {
    let tree = match self.peek().ok_or(Refused)? {
      b'{' => {
        self.enter()?;
        let mut entries = serde_json::Map::new();
        if self.peek() == Some(b'}') {
          self.leave();
        } else {
          loop {
            self.skip_white();
            let key = self.read_string()?.into_owned();
            self.expect(b':')?;
            self.skip_white();
            // of two entries with one key the later one stays
            entries.insert(key, self.read_tree()?);
            match self.peek() {
              Some(b',') => self.at += 1,
              Some(b'}') => {
                self.leave();
                break;
              }
              _ => return Err(Refused),
            }
          }
        }
        Json::Object(entries)
      }
      b'[' => {
        self.enter()?;
        let mut items = vec![];
        if self.peek() == Some(b']') {
          self.leave();
        } else {
          loop {
            self.skip_white();
            items.push(self.read_tree()?);
            match self.peek() {
              Some(b',') => self.at += 1,
              Some(b']') => {
                self.leave();
                break;
              }
              _ => return Err(Refused),
            }
          }
        }
        Json::Array(items)
      }
      b'"' => Json::String(self.read_string()?.into_owned()),
      b't' => self.word("true", Json::Bool(true))?,
      b'f' => self.word("false", Json::Bool(false))?,
      b'n' => self.word("null", Json::Null)?,
      _ => {
        let end = skip_number(self.text.as_bytes(), self.at).ok_or(Refused)?;
        let number = self.text.get(self.at..end).ok_or(Refused)?;
        self.at = end;
        Json::Number(exponent_as_serde_json(number).parse().map_err(|_| Refused)?)
      }
    };
    self.skip_white();
    Ok(tree)
  }

  /// Reads the JSON value at the position as [Reader::read_tree] does, for the order of the
  /// keys of its objects.
  fn read_order(&mut self) -> Result<Order, Refused> {
    let order = match self.peek().ok_or(Refused)? {
      b'{' => {
        self.enter()?;
        let mut entries: Vec<(Name, Order)> = vec![];
        // the place of each name among the entries
        let mut places: std::collections::HashMap<Name, usize> = std::collections::HashMap::new();
        if self.peek() == Some(b'}') {
          self.leave();
        } else {
          loop {
            self.skip_white();
            let name = Name::from(&*self.read_string()?);
            self.expect(b':')?;
            self.skip_white();
            let order = self.read_order()?;
            match places.get(&name) {
              Some(place) => entries[*place].1 = order,
              None => {
                places.insert(name.clone(), entries.len());
                entries.push((name, order));
              }
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
        }
        Order::Object(entries)
      }
      b'[' => {
        self.enter()?;
        let mut items = vec![];
        if self.peek() == Some(b']') {
          self.leave();
        } else {
          loop {
            self.skip_white();
            items.push(self.read_order()?);
            match self.peek() {
              Some(b',') => self.at += 1,
              Some(b']') => {
                self.leave();
                break;
              }
              _ => return Err(Refused),
            }
          }
        }
        Order::Array(items)
      }
      b'"' => {
        self.read_string()?;
        Order::Other
      }
      b't' => self.word("true", Order::Other)?,
      b'f' => self.word("false", Order::Other)?,
      b'n' => self.word("null", Order::Other)?,
      _ => {
        self.at = skip_number(self.text.as_bytes(), self.at).ok_or(Refused)?;
        Order::Other
      }
    };
    self.skip_white();
    Ok(order)
  }

  fn word<T>(&mut self, word: &str, value: T) -> Result<T, Refused> {
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
    // whether the objects among the items are looked at for reading them later
    let mut defer = true;
    loop {
      if defer {
        self.skip_white();
      }
      if defer && self.peek() == Some(b'{') {
        // the end of the object, from the pass over a value around it or from one now
        let known = self.shared.as_ref().and_then(|shared| shared.passed.lock().ok()?.get(&self.at).copied());
        let passed = known.or_else(|| {
          let mut inner = Passed::default();
          let passed = plain_end(self.text.as_bytes(), self.at, self.depth, &mut inner);
          if !inner.items.is_empty()
            && let Ok(mut all) = self.shared().passed.lock()
          {
            all.extend(inner.items.into_iter().map(|(start, end, depth)| (start, (end, depth))));
          }
          passed
        });
        match passed {
          Some((end, depth)) if end - self.at >= DEFER_FROM => {
            let record = Record {
              shared: self.shared().clone(),
              at: self.at,
              depth: self.depth,
            };
            items.push(Value::Context(Arc::new(FeelContext::deferred(Arc::new(record), depth))));
            self.at = end;
            self.skip_white();
          }
          // the items of an array are mostly alike: after a short one the others are read at once
          Some(_) => {
            defer = false;
            continue;
          }
          None => items.push(self.read_value(&Path::Index(path, items.len()))?),
        }
      } else {
        // the items of an array are mostly alike: after one that is no object none is looked at
        defer = false;
        items.push(self.read_value(&Path::Index(path, items.len()))?);
      }
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
    // whether a key of this object is written in another form than FEEL reads a name
    let mut other_forms = false;
    loop {
      self.skip_white();
      let key = self.read_string()?;
      // an empty key is an error, and a key with white space at an end is another name
      if key.is_empty() || rheon_feel::names::trim(&key).len() != key.len() {
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
          // the keys before this one are those of the object before
          other_forms = self.shapes[level].other_forms || !rheon_feel::names::is_in_name_form(&key);
          self.name_values(level, first_value);
          self.entries.push((Name::from(&*key), value));
        }
      } else {
        other_forms |= !rheon_feel::names::is_in_name_form(&key);
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
        let context = FeelContext::from_shared_names(names.clone(), values).with_other_key_forms(shape.other_forms);
        return Ok(match &shape.written {
          Some(written) => context.with_written_places(written.clone()),
          None => context,
        });
      }
      // fewer keys than the object before
      other_forms = self.shapes[level].other_forms;
      self.name_values(level, first_value);
    }
    self.remember_shape(level, first_entry, other_forms);
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
    let written = self.shapes[level].written.clone();
    // the first object with its keys at its depth shares the names with those after it
    if let Some(names) = &self.shapes[level].sorted
      && names.len() == self.entries.len() - first_entry
      && !(first_entry == 0 && self.entries.len() > DIRECT_ENTRIES)
    {
      let other_forms = self.shapes[level].other_forms;
      let values = self.entries.drain(first_entry..).map(|entry| entry.1).collect();
      let context = FeelContext::from_shared_names(names.clone(), values).with_other_key_forms(other_forms);
      return Ok(match written {
        Some(written) => context.with_written_places(written),
        None => context,
      });
    }
    // a context of many entries is told the places its entries were written at, which costs no
    // comparison of names; one of few is told their names
    let context = if first_entry == 0 && self.entries.len() > DIRECT_ENTRIES {
      // the outermost object takes the buffer as it is, without a copy of its entries
      let entries = std::mem::take(&mut self.entries);
      match &written {
        Some(written) => FeelContext::from_ascending_entries_written(entries, written),
        None => FeelContext::from_ascending_entries(entries),
      }
    } else if self.entries.len() - first_entry <= DIRECT_ENTRIES {
      // the names in the order they are written in
      let names: Option<Vec<Name>> = written.map(|written| {
        let entries = &self.entries[first_entry..];
        written.iter().filter_map(|place| entries.get(*place as usize).map(|entry| entry.0.clone())).collect()
      });
      let mut context = FeelContext::default();
      for (name, value) in self.entries.drain(first_entry..) {
        context.insert(name, value);
      }
      match names {
        Some(names) => context.with_written_names(names),
        None => context,
      }
    } else {
      let entries = self.entries.drain(first_entry..);
      match &written {
        Some(written) => FeelContext::from_ascending_entries_written(entries.collect(), written),
        None => FeelContext::from_ascending_entries(entries),
      }
    };
    Ok(context.with_other_key_forms(other_forms))
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
  fn remember_shape(&mut self, level: usize, first: usize, other_forms: bool) {
    let entries = &self.entries[first..];
    let shape = &mut self.shapes[level];
    shape.names.clear();
    // the outermost object comes once, so no object takes its names
    if level > self.outermost {
      shape.names.extend(entries.iter().map(|entry| entry.0.clone()));
    }
    shape.ascending = entries.windows(2).all(|pair| pair[0].0 < pair[1].0);
    shape.other_forms = other_forms;
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
    shape.written = (!shape.ascending).then(|| {
      // the place of each key in the ascending order, by the index of its last entry
      let mut place_of = vec![u32::MAX; entries.len()];
      for (place, index) in shape.order.iter().enumerate() {
        place_of[*index] = place as u32;
      }
      // an earlier entry of a key has the place of the last one, which the search finds
      let names: Vec<&Name> = shape.order.iter().map(|index| &entries[*index].0).collect();
      let mut seen = vec![false; names.len()];
      let mut written = Vec::with_capacity(names.len());
      for (index, entry) in entries.iter().enumerate() {
        let place = match place_of[index] {
          u32::MAX => names.binary_search(&&entry.0).unwrap_or(0) as u32,
          place => place,
        };
        if !std::mem::replace(&mut seen[place as usize], true) {
          written.push(place);
        }
      }
      written.into()
    });
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
  fn through_the_tree(text: &str) -> Result<(FeelContext, Vec<Warning>), String> {
    let Json::Object(map) = tree(text).map_err(|e| e.to_string())? else {
      return Err("no object".to_string());
    };
    let mut warnings = vec![];
    // the tree keeps the keys in their own order: the context is told the written one
    let written = |context| Order::read(text).map_or_else(FeelContext::default, |order| order.of_context(context));
    json_object_into_context(map, &Path::Root(""), &mut warnings).map(|context| (written(context), warnings))
  }

  /// A context as it is shown, with the contexts in it read: in the order its entries were
  /// written in.
  fn shown(context: &FeelContext) -> String {
    context.to_string()
  }

  fn cases() -> usize {
    if std::env::var_os("RHEON_FULL_TESTS").is_some() { 400_000 } else { 20_000 }
  }

  /// What the reader reads is what the conversion through the tree reads, with the same warnings
  /// in the same order; it refuses what that conversion refuses, and beyond that only an object
  /// with a key twice (whose first value may not convert), with a key that has white space at
  /// an end, or with a warning.
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
      let mut warnings = vec![Warning::new("earlier", "earlier")];
      let fast = Reader::object(&text, &Path::Root(""), &mut warnings);
      match (fast, through_the_tree(&text)) {
        (Ok(context), Ok((expected, expected_warnings))) => {
          assert_eq!(expected, context, "case {case}: {text}");
          assert_eq!(format!("{expected:?}"), format!("{context:?}"), "case {case}: {text}");
          assert_eq!(shown(&expected), shown(&context), "case {case}: {text}");
          assert_eq!(expected_warnings, warnings[1..], "case {case}: {text}");
          read += 1;
          with_warnings += usize::from(!expected_warnings.is_empty());
        }
        (Ok(context), Err(error)) => panic!("case {case}: {text}\nread {context:?}\nthe tree refuses: {error}"),
        (Err(Refused), Ok((_, expected_warnings))) => {
          // an entry that a later one with its key replaces is read here and not in the tree
          assert!(damaged || rng.1 || !expected_warnings.is_empty(), "case {case}: refused {text}");
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

  /// The nesting depth of a value: a list or a context is one deeper than its deepest part.
  fn nesting(value: &Value) -> u32 {
    match value {
      Value::List(items) => 1 + items.iter().map(nesting).max().unwrap_or(0),
      Value::Context(context) => 1 + context.iter().map(|(_, entry)| nesting(entry)).max().unwrap_or(0),
      _ => 0,
    }
  }

  /// Whether a value holds a context that is not read yet.
  fn has_unread(value: &Value) -> bool {
    match value {
      Value::List(items) => items.iter().any(has_unread),
      Value::Context(context) => !context.is_built() || context.iter().any(|(_, entry)| has_unread(entry)),
      _ => false,
    }
  }

  /// A value that is passed as plain is read: to the same end, without a warning, into a
  /// value of the nesting depth that the pass gives. So an object that is read later is read
  /// then, and has the depth it was given before.
  #[test]
  fn a_plain_value_is_read_without_a_warning() {
    let mut rng = Rng(0x0123_4567_89AB_CDEF, false);
    let (mut plain, mut other, mut items) = (0, 0, 0);
    for case in 0..cases() {
      let mut text = String::new();
      rng.1 = false;
      object(&mut rng, 0, &mut text);
      if rng.below(3) == 0 {
        text = damage(&mut rng, &text);
      }
      // what follows the value does not count
      text.push_str(rng.pick(&["", " ", ",1", "}", "x"]));
      let mut passed = Passed::default();
      let Some((end, depth)) = plain_end(text.as_bytes(), 0, 1, &mut passed) else {
        other += 1;
        // the objects that were passed inside a value that is not plain are plain
        for (start, end, depth) in passed.items {
          let mut warnings = vec![];
          let (value, read_end) = Reader::value_at(&text, start, &Path::Root(""), &mut warnings).unwrap_or_else(|Refused| panic!("case {case}: {start} of {text}"));
          assert_eq!((end, depth, true), (read_end, nesting(&value), warnings.is_empty()), "case {case}: {start} of {text}");
          items += 1;
        }
        continue;
      };
      // (a damaged text may start with a value that is no object)
      if !text.starts_with('{') {
        continue;
      }
      let mut warnings = vec![];
      let (value, read_end) = Reader::value_at(&text, 0, &Path::Root(""), &mut warnings).unwrap_or_else(|Refused| panic!("case {case}: {text}"));
      assert_eq!(end, read_end, "case {case}: {text}");
      assert_eq!(Vec::<Warning>::new(), warnings, "case {case}: {text}");
      assert_eq!(depth, nesting(&value), "case {case}: {text}");
      // and it is the value of the tree
      let (expected, expected_warnings) = through_the_tree(&text[..end]).unwrap_or_else(|e| panic!("case {case}: {text}: {e}"));
      assert_eq!(Value::from(expected), value, "case {case}: {text}");
      assert!(expected_warnings.is_empty(), "case {case}: {text}");
      plain += 1;
    }
    println!("{plain} plain, {other} not, {items} items passed inside those");
    assert!(
      plain > cases() / 20 && other > cases() / 4 && items > cases() / 100,
      "{plain} plain, {other} not, {items} items"
    );
  }

  /// One record of at least [DEFER_FROM] bytes: fields of numbers, texts and a list of small
  /// records; with `odd`, one thing that keeps it from being read later.
  fn record(rng: &mut Rng, odd: bool, out: &mut String) {
    out.push('{');
    let fields = 10 + rng.below(8);
    let odd_at = if odd { rng.below(fields) } else { usize::MAX };
    for field in 0..fields {
      if field > 0 {
        out.push_str(rng.pick(&[",", ", ", ",\n  "]));
      }
      if field == odd_at {
        out.push_str(rng.pick(&[
          "\"big\": 1234567890123456789012345678901234567890",
          "\"small\": 1e-7000",
          "\"exponent\": 12e3",
          "\"escape\": \"a\\nb\"",
          "\"key\\u0041\": 1",
          "\" spaced\": 1",
          "\"spaced \": 1",
          "\"field0\": \"again\"",
          "\"é\": 1",
          "\"$x\": 1",
          "\"deep\": [[[[{\"a\": [1, {\"b\": 12345678901234567890123456789012345}]}]]]]",
        ]));
        continue;
      }
      out.push_str(&format!("\"field{field}\": "));
      match rng.below(6) {
        0 => out.push_str(&format!("\"text {}\"", rng.below(1000))),
        1 => out.push_str(rng.pick(&["true", "false", "null"])),
        2 => {
          out.push('[');
          for item in 0..rng.below(4) {
            if item > 0 {
              out.push(',');
            }
            out.push_str(&format!("{{\"n\": {}, \"t\": \"{}\"}}", rng.below(100), rng.below(10)));
          }
          out.push(']');
        }
        _ => out.push_str(&format!("{}{}.{:02}", rng.pick(&["", "-"]), rng.below(100_000), rng.below(100))),
      }
    }
    out.push('}');
  }

  /// A list of records is read as through the tree, with the same warnings, whether its
  /// records are read at once or when they are first used; a record with something that could
  /// warn or be refused is read at once, and the others stay unread until something uses them.
  #[test]
  fn records_read_on_first_use_equal_the_conversion_through_the_tree() {
    let mut rng = Rng(0x0F1E_2D3C_4B5A_6978, false);
    let (mut read, mut refused, mut unread) = (0, 0, 0);
    for case in 0..cases() / 20 {
      let mut text = String::from("{\"small\": [{\"a\": 1}, {\"a\": 2}], \"rows\": [");
      let rows = 1 + rng.below(6);
      let mut any_odd = false;
      for row in 0..rows {
        if row > 0 {
          text.push_str(rng.pick(&[",", " , "]));
        }
        let odd = rng.below(5) == 0;
        any_odd |= odd;
        if rng.below(4) == 0 {
          // records inside a record
          text.push_str("{\"inner\": [");
          record(&mut rng, false, &mut text);
          text.push(',');
          record(&mut rng, odd, &mut text);
          text.push_str("], \"n\": 1}");
        } else {
          record(&mut rng, odd, &mut text);
        }
      }
      text.push_str("]}");
      let mut warnings = vec![];
      match (Reader::object(&text, &Path::Root(""), &mut warnings), through_the_tree(&text)) {
        (Ok(context), Ok((expected, expected_warnings))) => {
          let rows_value = context.get_entry(&Name::from("rows")).cloned().unwrap_or_else(|| panic!("case {case}: {text}"));
          let small = context.get_entry(&Name::from("small")).cloned().unwrap_or_else(|| panic!("case {case}: {text}"));
          // short records are read at once; of the others, those with nothing odd are not yet
          assert!(!has_unread(&small), "case {case}: {text}");
          let unread_before = has_unread(&rows_value);
          assert!(unread_before || any_odd, "case {case}: every record was read at once: {text}");
          unread += usize::from(unread_before);
          // a copy made before the records are read is read on its own
          let copy = context.clone();
          assert_eq!(expected, context, "case {case}: {text}");
          assert_eq!(format!("{expected:?}"), format!("{context:?}"), "case {case}: {text}");
          assert_eq!(shown(&expected), shown(&context), "case {case}: {text}");
          assert_eq!(expected, copy, "case {case}: {text}");
          assert_eq!(expected_warnings, warnings, "case {case}: {text}");
          assert!(!has_unread(&rows_value), "case {case}: {text}");
          read += 1;
        }
        (Ok(context), Err(error)) => panic!("case {case}: {text}\nread {context:?}\nthe tree refuses: {error}"),
        (Err(Refused), _) => {
          assert!(any_odd, "case {case}: refused {text}");
          refused += 1;
        }
      }
    }
    println!("{read} read ({unread} with records left unread), {refused} refused");
    let cases = cases() / 20;
    assert!(
      read > cases / 2 && unread > cases / 2 && refused > cases / 50,
      "{read} read, {unread} unread, {refused} refused"
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
      assert_eq!(shown(&expected), shown(&read), "case {case}: {text}");
    }
  }

  /// The key by which `serde_json` hands a number to its tree.
  const PRIVATE_NUMBER_KEY: &str = "$serde_json::private::Number";

  /// `tree` with `to` in the place of `from` in every key and every string.
  fn renamed(tree: Json, from: &str, to: &str) -> Json {
    match tree {
      Json::String(string) => Json::String(string.replace(from, to)),
      Json::Array(items) => Json::Array(items.into_iter().map(|item| renamed(item, from, to)).collect()),
      // the keys are those of the text, where the later of two entries with one key stays
      Json::Object(entries) => Json::Object(entries.into_iter().map(|(key, value)| (key.replace(from, to), renamed(value, from, to))).collect()),
      other => other,
    }
  }

  /// The tree of a text is the tree that `serde_json` builds for it, and a text that is no JSON
  /// has the error of `serde_json`; with the private number key in the text, they are those of
  /// the text with another key of the same length in its place.
  #[test]
  fn tree_is_that_of_serde_json_but_for_its_private_number_key() {
    let other_key = "$serde_json::private::NumbeX";
    let mut rng = Rng(0x0F0F_1234_5678_9ABC, false);
    let (mut trees, mut errors, mut private) = (0, 0, 0);
    for case in 0..cases() {
      let mut text = String::from(rng.pick(&WHITE));
      value(&mut rng, 0, &mut text);
      text.push_str(rng.pick(&WHITE));
      if rng.below(3) == 0 {
        text = damage(&mut rng, &text);
      }
      let has_key = text.contains(PRIVATE_NUMBER_KEY);
      let expected = serde_json::from_str::<Json>(&text.replace(PRIVATE_NUMBER_KEY, other_key)).map(|tree| renamed(tree, other_key, PRIVATE_NUMBER_KEY));
      match (tree(&text), expected) {
        (Ok(tree), Ok(expected)) => {
          assert_eq!(expected, tree, "case {case}: {text:?}");
          trees += 1;
          private += usize::from(has_key);
        }
        (Err(error), Err(expected)) => {
          assert_eq!(expected.to_string(), error.to_string(), "case {case}: {text:?}");
          errors += 1;
          private += usize::from(has_key);
        }
        (tree, expected) => panic!("case {case}: {text:?}\n{tree:?}\nserde_json: {expected:?}"),
      }
    }
    println!("{trees} trees, {errors} errors, {private} with the private number key");
    assert!(
      trees > cases() / 4 && errors > cases() / 20 && private > cases() / 100,
      "{trees} trees, {errors} errors, {private} with the key"
    );
    // a value of the key that is no number text, further keys, the key written with an escape
    for text in [
      r#"{"$serde_json::private::Number":"42"}"#,
      r#"{"$serde_json::private::Number":true}"#,
      r#"{"$serde_json::private::Number":"42","b":1}"#,
      r#"[{"a":{"$serde_json::private::Number":{"$serde_json::private::Number":null}}}]"#,
      r#"{"$serde_json::private::\u004eumber":"1e5"}"#,
    ] {
      let tree = tree(text).unwrap_or_else(|error| panic!("{text}: {error}"));
      assert_eq!(
        text.matches("::private::").count(),
        format!("{tree:?}").matches("Object {\"$serde_json::private::Number\"").count(),
        "{text}: {tree:?}"
      );
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
        let Ok(Json::Object(map)) = tree(&wrapped) else {
          panic!("case {case}: took the request {wrapped:?}");
        };
        // the fields that were not read into values are those of the tree
        assert!(read.fields.iter().all(|(key, _)| map.contains_key(*key)), "case {case}: {wrapped:?}");
        requests += 1;
      }
      let tree = tree(&text);
      match fields(&text) {
        Some(fields) => {
          let Ok(Json::Object(map)) = &tree else {
            panic!("case {case}: took {text:?}, serde_json: {tree:?}");
          };
          // the last field of a key is the entry of the tree
          for (key, value) in &fields {
            let last = fields.iter().rev().find(|(other, _)| other == key).map(|(_, value)| *value);
            assert_eq!(map.get(*key), super::tree(last.unwrap()).ok().as_ref(), "case {case}: {key} in {text:?}");
            assert!(super::tree(value).is_ok(), "case {case}: {value:?} in {text:?}");
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
            // refused though it is an object: for an escape in a top-level key
            assert!(text.contains('\\'), "case {case}: refused {text:?}");
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
