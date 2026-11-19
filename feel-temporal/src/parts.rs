//! # The parts of a temporal literal
//!
//! A date, a time, a date and time, a zone and the two durations are split into their parts by
//! hand, one pass over the bytes of the text. The grammar of each is given as a pattern above its
//! function; the tests hold every function against that pattern, compiled as a regular expression,
//! which is what split the literals before.
//!
//! A run of digits is always taken whole: in every pattern a run of digits is followed by a
//! character that is no digit or by the end of the text.

/// A part of a literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Part<'a>(&'a str);

impl<'a> Part<'a> {
  /// The text of the part.
  pub(crate) fn as_str(&self) -> &'a str {
    self.0
  }
}

/// The parts of a temporal literal, by the names the patterns give them; a part that the literal
/// does not have is `None`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Parts<'a> {
  pub(crate) sign: Option<&'a str>,
  pub(crate) year: Option<&'a str>,
  pub(crate) month: Option<&'a str>,
  pub(crate) day: Option<&'a str>,
  pub(crate) hours: Option<&'a str>,
  pub(crate) minutes: Option<&'a str>,
  pub(crate) seconds: Option<&'a str>,
  /// The `.` and the digits after it.
  pub(crate) fractional: Option<&'a str>,
  pub(crate) zulu: Option<&'a str>,
  /// The name of a zone, without the `@`.
  pub(crate) zone: Option<&'a str>,
  pub(crate) off_sign: Option<&'a str>,
  pub(crate) off_hours: Option<&'a str>,
  pub(crate) off_minutes: Option<&'a str>,
  pub(crate) days: Option<&'a str>,
  pub(crate) years: Option<&'a str>,
  pub(crate) months: Option<&'a str>,
}

impl<'a> Parts<'a> {
  /// The part with the name the patterns give it.
  pub(crate) fn name(&self, name: &str) -> Option<Part<'a>> {
    let part = match name {
      "sign" => self.sign,
      "year" => self.year,
      "month" => self.month,
      "day" => self.day,
      "hours" => self.hours,
      "minutes" => self.minutes,
      "seconds" => self.seconds,
      "fractional" => self.fractional,
      "zulu" => self.zulu,
      "zone" => self.zone,
      "offSign" => self.off_sign,
      "offHours" => self.off_hours,
      "offMinutes" => self.off_minutes,
      "days" => self.days,
      "years" => self.years,
      "months" => self.months,
      _ => None,
    };
    part.map(Part)
  }
}

/// A position in the text of a literal.
struct Cursor<'a> {
  text: &'a str,
  at: usize,
}

impl<'a> Cursor<'a> {
  fn new(text: &'a str) -> Self {
    Self { text, at: 0 }
  }

  fn peek(&self) -> Option<u8> {
    self.text.as_bytes().get(self.at).copied()
  }

  fn at_end(&self) -> bool {
    self.at == self.text.len()
  }

  /// Passes `byte` (an ASCII character) when it is next, and gives its text.
  fn eat(&mut self, byte: u8) -> Option<&'a str> {
    (self.peek() == Some(byte)).then(|| self.take(1))
  }

  /// The next `length` bytes, which are ASCII.
  fn take(&mut self, length: usize) -> &'a str {
    let start = self.at;
    self.at += length;
    &self.text[start..self.at]
  }

  /// The run of digits that is next, whole, when it has `least` to `most` digits.
  fn digits(&mut self, least: usize, most: usize) -> Option<&'a str> {
    let length = self.text.as_bytes()[self.at..].iter().take_while(|byte| byte.is_ascii_digit()).count();
    (least..=most).contains(&length).then(|| self.take(length))
  }
}

/// `(?P<sign>-)?(?P<year>[1-9][0-9]{3,8})-(?P<month>[0-9]{2})-(?P<day>[0-9]{2})`
fn date<'a>(cursor: &mut Cursor<'a>, parts: &mut Parts<'a>) -> Option<()> {
  parts.sign = cursor.eat(b'-');
  if cursor.peek() == Some(b'0') {
    return None;
  }
  parts.year = Some(cursor.digits(4, 9)?);
  cursor.eat(b'-')?;
  parts.month = Some(cursor.digits(2, 2)?);
  cursor.eat(b'-')?;
  parts.day = Some(cursor.digits(2, 2)?);
  Some(())
}

/// `(?P<hours>[0-9]{2}):(?P<minutes>[0-9]{2}):(?P<seconds>[0-9]{2})(?P<fractional>\.[0-9]+)?`
fn time<'a>(cursor: &mut Cursor<'a>, parts: &mut Parts<'a>) -> Option<()> {
  parts.hours = Some(cursor.digits(2, 2)?);
  cursor.eat(b':')?;
  parts.minutes = Some(cursor.digits(2, 2)?);
  cursor.eat(b':')?;
  parts.seconds = Some(cursor.digits(2, 2)?);
  if cursor.peek() == Some(b'.') {
    let start = cursor.at;
    cursor.take(1);
    cursor.digits(1, usize::MAX)?;
    parts.fractional = Some(&cursor.text[start..cursor.at]);
  }
  Some(())
}

/// `(?P<zulu>[zZ])|@(?P<zone>[a-zA-Z_/]+)|(?P<offSign>[+-])(?P<offHours>[0-9]{2}):(?P<offMinutes>[0-9]{2})`
fn zone<'a>(cursor: &mut Cursor<'a>, parts: &mut Parts<'a>) -> Option<()> {
  match cursor.peek()? {
    b'z' | b'Z' => parts.zulu = Some(cursor.take(1)),
    b'@' => {
      cursor.take(1);
      let length = cursor.text.as_bytes()[cursor.at..]
        .iter()
        .take_while(|byte| byte.is_ascii_alphabetic() || matches!(byte, b'_' | b'/'))
        .count();
      if length == 0 {
        return None;
      }
      parts.zone = Some(cursor.take(length));
    }
    b'+' | b'-' => {
      parts.off_sign = Some(cursor.take(1));
      parts.off_hours = Some(cursor.digits(2, 2)?);
      cursor.eat(b':')?;
      parts.off_minutes = Some(cursor.digits(2, 2)?);
    }
    _ => return None,
  }
  Some(())
}

/// The parts when the text ends here.
fn ended<'a>(cursor: &Cursor<'a>, parts: Parts<'a>) -> Option<Parts<'a>> {
  cursor.at_end().then_some(parts)
}

/// The parts of a date: `^DATE$`.
pub(crate) fn split_date(text: &str) -> Option<Parts<'_>> {
  let (mut cursor, mut parts) = (Cursor::new(text), Parts::default());
  date(&mut cursor, &mut parts)?;
  ended(&cursor, parts)
}

/// The parts of a time: `^TIME(ZONE)?$`.
pub(crate) fn split_time(text: &str) -> Option<Parts<'_>> {
  let (mut cursor, mut parts) = (Cursor::new(text), Parts::default());
  time(&mut cursor, &mut parts)?;
  if !cursor.at_end() {
    zone(&mut cursor, &mut parts)?;
  }
  ended(&cursor, parts)
}

/// The parts of a zone: `^(?:ZONE)$`.
pub(crate) fn split_zone(text: &str) -> Option<Parts<'_>> {
  let (mut cursor, mut parts) = (Cursor::new(text), Parts::default());
  zone(&mut cursor, &mut parts)?;
  ended(&cursor, parts)
}

/// The parts of a date and time: `^DATETTIME(ZONE)?$`.
pub(crate) fn split_date_and_time(text: &str) -> Option<Parts<'_>> {
  let (mut cursor, mut parts) = (Cursor::new(text), Parts::default());
  date(&mut cursor, &mut parts)?;
  cursor.eat(b'T')?;
  time(&mut cursor, &mut parts)?;
  if !cursor.at_end() {
    zone(&mut cursor, &mut parts)?;
  }
  ended(&cursor, parts)
}

/// The parts of a days and time duration:
/// `^(?P<sign>-)?P((?P<days>[0-9]+)D)?(T((?P<hours>[0-9]+)H)?((?P<minutes>[0-9]+)M)?((?P<seconds>[0-9]+)(?P<fractional>\.[0-9]*)?S)?)?$`
pub(crate) fn split_days_and_time(text: &str) -> Option<Parts<'_>> {
  let (mut cursor, mut parts) = (Cursor::new(text), Parts::default());
  parts.sign = cursor.eat(b'-');
  cursor.eat(b'P')?;
  if let Some(days) = cursor.digits(1, usize::MAX) {
    cursor.eat(b'D')?;
    parts.days = Some(days);
  }
  if cursor.eat(b'T').is_some() {
    // the components in their order, each once: 0 before any, 1 after hours, 2 after minutes
    let mut passed = 0;
    while let Some(number) = cursor.digits(1, usize::MAX) {
      match cursor.peek()? {
        b'H' if passed < 1 => {
          parts.hours = Some(number);
          passed = 1;
          cursor.take(1);
        }
        b'M' if passed < 2 => {
          parts.minutes = Some(number);
          passed = 2;
          cursor.take(1);
        }
        b'S' | b'.' => {
          parts.seconds = Some(number);
          if cursor.peek() == Some(b'.') {
            let start = cursor.at;
            cursor.take(1);
            cursor.digits(0, usize::MAX)?;
            parts.fractional = Some(&cursor.text[start..cursor.at]);
          }
          cursor.eat(b'S')?;
          break;
        }
        _ => return None,
      }
    }
  }
  ended(&cursor, parts)
}

/// The parts of a years and months duration:
/// `^(?P<sign>-)?P((?P<years>[0-9]+)Y)?((?P<months>[0-9]+)M)?$`
pub(crate) fn split_years_and_months(text: &str) -> Option<Parts<'_>> {
  let (mut cursor, mut parts) = (Cursor::new(text), Parts::default());
  parts.sign = cursor.eat(b'-');
  cursor.eat(b'P')?;
  if let Some(number) = cursor.digits(1, usize::MAX) {
    match cursor.peek()? {
      b'Y' => {
        cursor.take(1);
        parts.years = Some(number);
        if let Some(months) = cursor.digits(1, usize::MAX) {
          cursor.eat(b'M')?;
          parts.months = Some(months);
        }
      }
      b'M' => {
        cursor.take(1);
        parts.months = Some(number);
      }
      _ => return None,
    }
  }
  ended(&cursor, parts)
}

#[cfg(test)]
mod tests {
  use super::*;
  use regex::Regex;

  const DATE: &str = r"(?P<sign>-)?(?P<year>[1-9][0-9]{3,8})-(?P<month>[0-9]{2})-(?P<day>[0-9]{2})";
  const TIME: &str = r"(?P<hours>[0-9]{2}):(?P<minutes>[0-9]{2}):(?P<seconds>[0-9]{2})(?P<fractional>\.[0-9]+)?";
  const ZONE: &str = r"(?P<zulu>[zZ])|@(?P<zone>[a-zA-Z_/]+)|(?P<offSign>[+-])(?P<offHours>[0-9]{2}):(?P<offMinutes>[0-9]{2})";
  const DAYS_AND_TIME: &str = r"^(?P<sign>-)?P((?P<days>[0-9]+)D)?(T((?P<hours>[0-9]+)H)?((?P<minutes>[0-9]+)M)?((?P<seconds>[0-9]+)(?P<fractional>\.[0-9]*)?S)?)?$";
  const YEARS_AND_MONTHS: &str = r"^(?P<sign>-)?P((?P<years>[0-9]+)Y)?((?P<months>[0-9]+)M)?$";

  const NAMES: [&str; 16] = [
    "sign",
    "year",
    "month",
    "day",
    "hours",
    "minutes",
    "seconds",
    "fractional",
    "zulu",
    "zone",
    "offSign",
    "offHours",
    "offMinutes",
    "days",
    "years",
    "months",
  ];

  /// The parts that the pattern gives, by name.
  fn by_pattern<'a>(pattern: &Regex, text: &'a str) -> Option<Vec<Option<&'a str>>> {
    let captures = pattern.captures(text)?;
    Some(
      NAMES
        .iter()
        .map(|name| {
          pattern
            .capture_names()
            .flatten()
            .any(|known| known == *name)
            .then(|| captures.name(name))
            .flatten()
            .map(|part| part.as_str())
        })
        .collect(),
    )
  }

  /// The parts that a function gives, by name.
  fn by_hand<'a>(split: fn(&'a str) -> Option<Parts<'a>>, text: &'a str) -> Option<Vec<Option<&'a str>>> {
    let parts = split(text)?;
    Some(NAMES.iter().map(|name| parts.name(name).map(|part| part.as_str())).collect())
  }

  struct Generator(u64);

  impl Generator {
    fn next(&mut self, below: usize) -> usize {
      self.0 ^= self.0 << 13;
      self.0 ^= self.0 >> 7;
      self.0 ^= self.0 << 17;
      (self.0 % below as u64) as usize
    }

    /// A text made of `pieces`, or one of `whole` changed in a few places: a character removed,
    /// doubled, replaced or inserted, or a piece put in.
    fn text(&mut self, whole: &[&str], pieces: &[&str]) -> String {
      if self.next(4) == 0 {
        return (0..self.next(9)).map(|_| pieces[self.next(pieces.len())]).collect();
      }
      let mut text: Vec<char> = whole[self.next(whole.len())].chars().collect();
      for _ in 0..self.next(4) {
        let at = self.next(text.len() + 1);
        let piece: Vec<char> = pieces[self.next(pieces.len())].chars().collect();
        match self.next(5) {
          0 if at < text.len() => drop(text.remove(at)),
          1 if at < text.len() => text.insert(at, text[at]),
          2 if at < text.len() => text[at] = piece[0],
          3 => text.insert(at, piece[0]),
          _ => drop(text.splice(at..at, piece)),
        }
      }
      text.into_iter().collect()
    }
  }

  fn rounds() -> usize {
    if std::env::var_os("RHEON_FULL_TESTS").is_some() { 5_000_000 } else { 200_000 }
  }

  /// Holds a function against its pattern on generated texts, and says how many of them matched.
  fn agree(pattern: &str, split: for<'a> fn(&'a str) -> Option<Parts<'a>>, whole: &[&str], pieces: &[&str]) {
    let pattern = Regex::new(pattern).unwrap();
    let mut generator = Generator(0x9E37_79B9_7F4A_7C15);
    let mut matched = 0;
    for text in whole {
      assert!(by_pattern(&pattern, text).is_some(), "{text}");
    }
    for _ in 0..rounds() {
      let text = generator.text(whole, pieces);
      let expected = by_pattern(&pattern, &text);
      matched += usize::from(expected.is_some());
      assert_eq!(expected, by_hand(split, &text), "{text:?}");
    }
    assert!(matched > rounds() / 20, "too few texts matched: {matched}");
  }

  const TEMPORAL_PIECES: &[&str] = &[
    "0",
    "1",
    "2",
    "9",
    "00",
    "12",
    "59",
    "2024",
    "999999999",
    "-",
    ":",
    ".",
    "T",
    "Z",
    "z",
    "@",
    "+",
    "Europe/Paris",
    "Etc/UTC",
    "a",
    "_",
    "/",
    " ",
    "\n",
    "é",
    "٣",
    "P",
    "t",
  ];
  const DATES: &[&str] = &["2024-01-02", "-2024-01-02", "999999999-12-31", "1000-00-99", "12345-06-07"];
  const TIMES: &[&str] = &[
    "10:20:30",
    "10:20:30.5",
    "10:20:30.123456789012",
    "24:00:00",
    "10:20:30Z",
    "10:20:30z",
    "10:20:30+02:00",
    "10:20:30-11:30",
    "10:20:30@Europe/Paris",
    "10:20:30.25@Etc/UTC",
    "99:99:99.0+99:99",
  ];
  const ZONES: &[&str] = &["Z", "z", "+02:00", "-11:30", "@Europe/Paris", "@Etc/UTC", "@a_b/c", "+99:99"];

  #[test]
  fn a_date_is_split_as_its_pattern_splits_it() {
    agree(&format!("^{DATE}$"), split_date, DATES, TEMPORAL_PIECES);
  }

  #[test]
  fn a_time_is_split_as_its_pattern_splits_it() {
    agree(&format!("^{TIME}({ZONE})?$"), split_time, TIMES, TEMPORAL_PIECES);
  }

  #[test]
  fn a_zone_is_split_as_its_pattern_splits_it() {
    agree(&format!("^(?:{ZONE})$"), split_zone, ZONES, TEMPORAL_PIECES);
  }

  #[test]
  fn a_date_and_time_is_split_as_its_pattern_splits_it() {
    let whole: Vec<String> = DATES.iter().flat_map(|date| TIMES.iter().map(move |time| format!("{date}T{time}"))).collect();
    let whole: Vec<&str> = whole.iter().map(String::as_str).collect();
    agree(&format!("^{DATE}T{TIME}({ZONE})?$"), split_date_and_time, &whole, TEMPORAL_PIECES);
  }

  const DURATION_PIECES: &[&str] = &[
    "0",
    "1",
    "23",
    "999999999999999999999",
    "P",
    "T",
    "D",
    "H",
    "M",
    "S",
    "Y",
    ".",
    "-",
    "+",
    " ",
    "p",
    "\n",
    "é",
    "٣",
  ];

  #[test]
  fn a_days_and_time_duration_is_split_as_its_pattern_splits_it() {
    let whole = [
      "P1D",
      "-P1DT2H3M4S",
      "PT5H",
      "PT6M",
      "PT7S",
      "PT7.5S",
      "PT7.S",
      "P1DT2H",
      "PT2H4S",
      "PT3M4.000000001S",
      "P",
      "PT",
      "P999999999999999999999DT1H",
      "P1DT",
    ];
    agree(DAYS_AND_TIME, split_days_and_time, &whole, DURATION_PIECES);
  }

  #[test]
  fn a_years_and_months_duration_is_split_as_its_pattern_splits_it() {
    let whole = ["P1Y", "-P1Y2M", "P3M", "P", "-P", "P999999999999999999999Y1M", "P0Y0M"];
    agree(YEARS_AND_MONTHS, split_years_and_months, &whole, DURATION_PIECES);
  }
}
