//! Unit tests of the lexer's character classes and of the flattening of name parts.

use super::*;
use crate::tests::full_size;

#[test]
fn test_ends_word() {
  for ch in [' ', '=', '!', '<', '>', '+', '-', '*', '/', '%', '.', ',', ')', '[', ']', '}', '(', '{', '"', ':', '\''] {
    assert!(ends_word(ch), "{ch}");
  }
  for ch in ['a', 'Z', '_', '1', '?', 'é', '\u{00B7}'] {
    assert!(!ends_word(ch), "{ch}");
  }
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
      // without a part that is empty after trimming, no candidate is a composite name (see Lexer::longest_name_in_scope)
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
fn test_built_in_property_bounds() {
  // after a dot, the bounds of the candidate names admit every built-in property name
  for name in BUILT_IN_PROPERTY_NAMES {
    assert!(name.len() <= MAX_BUILT_IN_PROPERTY_NAME_LEN, "{name}");
    assert!(property_group_bound(first_word(name)) >= name.len(), "{name}");
  }
  assert_eq!(BUILT_IN_PROPERTY_NAMES.iter().map(|name| name.len()).max(), Some(MAX_BUILT_IN_PROPERTY_NAME_LEN));
  assert_eq!(0, property_group_bound("x"));
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

/// The keys of a context literal that are names of several tokens are found before its entries
/// are parsed; strings, comments and what stands in brackets are passed over.
#[test]
fn the_keys_of_a_literal_of_several_tokens_are_found_by_one_pass() {
  let keys = |text: &str| literal_keys(text, text.find('{').unwrap()).iter().map(|name| name.to_string()).collect::<Vec<_>>();
  assert_eq!(keys("{a: 1, b: 2}"), Vec::<String>::new());
  assert_eq!(keys("{unit price: 1, a-b: 2, c: 3}"), ["unit price", "a-b"]);
  assert_eq!(
    keys("{ Profit  and   Loss : 1, a - b: 2, x.y: 3, that's it: 4}"),
    ["Profit and Loss", "a-b", "x.y", "that's it"]
  );
  // a key that is a string is read by the name its text is; a key is read once per entry
  assert_eq!(keys(r#"{"a b": 1, c d: "x: y, e f: 1"}"#), ["a b", "c d"]);
  assert_eq!(keys(r#"{"Date - Time": 1, "a  b": 2, "one": 3, "1 a": 4, "a\"b c": 5}"#), ["Date-Time", "a b"]);
  // nested literals, parameters, lists and comments are passed over
  assert_eq!(
    keys("{a b: {c d: 1}, e f: g(h i: 1, j k: 2), l m: [n o, 1], /* p q: */ r s: 1 // t u: 2\n, v w: 3}"),
    ["a b", "e f", "l m", "r s", "v w"]
  );
  // an interval with its brackets reversed
  assert_eq!(keys("{a b: 1 in ]1..2[, c d: 2}"), ["a b", "c d"]);
  // the literal ends at its brace
  assert_eq!(keys("{a b: 1} + {c d: 2}"), ["a b"]);
  assert_eq!(keys("[1, {a b: 1}, x y: 2]"), ["a b"]);
  // white space before the brace, of one byte or more
  assert_eq!(literal_keys("  {a b: 1}", 0).len(), 1);
  assert_eq!(literal_keys("\u{feff}{a b: 1}", 0).len(), 1);
  assert_eq!(literal_keys("\u{feff}{}:", 0).len(), 0);
  assert_eq!(literal_keys("no brace", 0).len(), 0);
  // no name: a number first, another character, or nothing before the colon
  assert_eq!(keys("{1 a: 1, a $ b: 2, : 3, a b"), Vec::<String>::new());
}
