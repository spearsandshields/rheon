//! Unit tests of the lexer's character classes and of the flattening of name parts.

use super::*;
use crate::tests::full_size;

#[test]
fn test_is_separator() {
  assert!(is_separator(' '));
  assert!(is_separator('='));
  assert!(is_separator('!'));
  assert!(is_separator('<'));
  assert!(is_separator('>'));
  assert!(is_separator('+'));
  assert!(is_separator('-'));
  assert!(is_separator('*'));
  assert!(is_separator('/'));
  assert!(is_separator('%'));
  assert!(is_separator('.'));
  assert!(is_separator(','));
  assert!(is_separator(')'));
  assert!(is_separator('['));
  assert!(is_separator(']'));
  assert!(is_separator('}'));
  assert!(!is_separator('('));
  assert!(!is_separator('{'));
  assert!(!is_separator('?'));
}

#[test]
fn test_is_additional_name_symbol() {
  assert!(is_additional_name_symbol('/'));
  assert!(is_additional_name_symbol('-'));
  assert!(is_additional_name_symbol('\''));
  assert!(is_additional_name_symbol('+'));
  assert!(is_additional_name_symbol('*'));
  assert!(!is_additional_name_symbol(' '));
  assert!(!is_additional_name_symbol('$'));
  assert!(!is_additional_name_symbol(':'));
  assert!(!is_additional_name_symbol('('));
  assert!(!is_additional_name_symbol(')'));
  assert!(!is_additional_name_symbol('['));
  assert!(!is_additional_name_symbol(']'));
  assert!(!is_additional_name_symbol('{'));
  assert!(!is_additional_name_symbol('}'));
  assert!(!is_additional_name_symbol('?'));
}

/// Returns the flattened name of `parts` by its definition: the parts, each trimmed of white
/// space at both ends, joined with single spaces; the result trimmed; then the spaces around the
/// additional name symbols removed with [str::replace], `.` first, `*` last. The reference of
/// [flatten_name_parts_into] in the tests below.
fn flatten_name_parts(parts: &[String]) -> String {
  parts
    .iter()
    .map(|s| s.trim().to_string())
    .collect::<Vec<String>>()
    .join(" ")
    .trim()
    .to_string()
    .replace(" . ", ".")
    .replace(" / ", "/")
    .replace(" - ", "-")
    .replace(" ' ", "'")
    .replace(" + ", "+")
    .replace(" * ", "*")
}

/// Returns the flattened name of `parts` written by [flatten_name_parts_into] into buffers that
/// hold an earlier text.
fn flattened(parts: &[&str]) -> String {
  let (mut out, mut tmp) = (String::from("stale"), String::from("stale"));
  flatten_name_parts_into(parts.iter().copied(), &mut out, &mut tmp);
  out
}

#[test]
fn test_flatten_name_parts() {
  assert_eq!("left middle right", flattened(&["left", "middle", "right"]));
  assert_eq!("a.b", flattened(&["   a  ", " . ", "  b "]));
  assert_eq!("a/b", flattened(&["   a  ", " / ", "  b "]));
  assert_eq!("a-b", flattened(&["   a  ", " - ", "  b "]));
  assert_eq!("a'b", flattened(&["   a  ", " ' ", "  b "]));
  assert_eq!("a+b", flattened(&["   a  ", " + ", "  b "]));
  assert_eq!("a*b", flattened(&["   a  ", " * ", "  b "]));
  assert_eq!("", flattened(&["", "", ""]));
  assert_eq!("", flattened(&[]));
}

#[test]
fn test_flatten_name_parts_into() {
  // every sequence of up to 4 parts (6 in full size) over names, U+1680-only parts and the
  // additional symbols
  let alphabet = ["a", "b c", "\u{1680}", "\u{1680}x", ".", "/", "-", "'", "+", "*", " . "];
  let max_len = if full_size() { 6 } else { 4 };
  let (mut out, mut tmp) = (String::from("stale"), String::from("stale"));
  let mut parts = vec![];
  let mut counters = [0_usize; 6];
  for len in 0..=max_len {
    counters.iter_mut().for_each(|c| *c = 0);
    loop {
      parts.clear();
      parts.extend(counters[..len].iter().map(|&i| alphabet[i].to_string()));
      flatten_name_parts_into(parts.iter().map(String::as_str), &mut out, &mut tmp);
      assert_eq!(flatten_name_parts(&parts), out, "{parts:?}");
      // without a part that is empty after trimming, no candidate is a composite name (see ParsingScope::bound)
      assert!(parts.iter().any(|part| part.trim().is_empty()) || !out.contains(" . "), "{parts:?}");
      // next combination
      let mut i = 0;
      while i < len && counters[i] == alphabet.len() - 1 {
        counters[i] = 0;
        i += 1;
      }
      if i == len {
        break;
      }
      counters[i] += 1;
    }
  }
}

#[test]
fn test_hex_to_decimal() {
  assert_eq!(0, hex_to_decimal('0').unwrap());
  assert_eq!(1, hex_to_decimal('1').unwrap());
  assert_eq!(2, hex_to_decimal('2').unwrap());
  assert_eq!(3, hex_to_decimal('3').unwrap());
  assert_eq!(4, hex_to_decimal('4').unwrap());
  assert_eq!(5, hex_to_decimal('5').unwrap());
  assert_eq!(6, hex_to_decimal('6').unwrap());
  assert_eq!(7, hex_to_decimal('7').unwrap());
  assert_eq!(8, hex_to_decimal('8').unwrap());
  assert_eq!(9, hex_to_decimal('9').unwrap());
  assert_eq!(10, hex_to_decimal('A').unwrap());
  assert_eq!(11, hex_to_decimal('B').unwrap());
  assert_eq!(12, hex_to_decimal('C').unwrap());
  assert_eq!(13, hex_to_decimal('D').unwrap());
  assert_eq!(14, hex_to_decimal('E').unwrap());
  assert_eq!(15, hex_to_decimal('F').unwrap());
  assert_eq!(10, hex_to_decimal('a').unwrap());
  assert_eq!(11, hex_to_decimal('b').unwrap());
  assert_eq!(12, hex_to_decimal('c').unwrap());
  assert_eq!(13, hex_to_decimal('d').unwrap());
  assert_eq!(14, hex_to_decimal('e').unwrap());
  assert_eq!(15, hex_to_decimal('f').unwrap());
  assert_eq!("<LexerError> expected hex digit but encountered 'G'", hex_to_decimal('G').unwrap_err().to_string());
  assert_eq!("<LexerError> expected hex digit but encountered 'g'", hex_to_decimal('g').unwrap_err().to_string());
}
