use crate::Name;
use dsntk_common::Jsonify;
use std::collections::HashMap;

#[test]
fn default_should_work() {
  let name: Name = Default::default();
  assert_eq!("", name.to_string());
}

#[test]
fn display_should_work() {
  let name: Name = vec!["   x   ".to_string(), " y      \t".to_string(), "  \n  z  \t  ".to_string()].into();
  assert_eq!("x y z", format!("{name}"));
}

#[test]
fn debug_should_work() {
  let name: Name = vec!["   x   ".to_string(), " y      \t".to_string(), "  \n  z  \t  ".to_string()].into();
  assert_eq!(r#"Name("x y z")"#, format!("{name:?}"));
}

#[test]
fn into_string_should_work() {
  let name: Name = vec!["a".to_string(), "+".to_string(), "b".to_string()].into();
  let s: String = name.into();
  assert_eq!("a+b", s);
}

#[test]
fn to_string_should_work() {
  let name: Name = vec!["a".to_string(), "+".to_string(), "b".to_string()].into();
  let s = name.to_string();
  assert_eq!("a+b", s);
}

#[test]
fn jsonify_should_work() {
  let name: Name = vec!["a".to_string(), "+".to_string(), "b".to_string()].into();
  let s: String = name.jsonify();
  assert_eq!("a+b", s);
}

#[test]
fn creating_empty_name_should_work() {
  let name: Name = vec!["".to_string()].into();
  assert_eq!("", name.to_string());
  assert_eq!("", name.jsonify());
}

#[test]
fn is_empty_function_should_work() {
  let name: Name = vec!["".to_string()].into();
  assert!(name.is_empty());
  let name: Name = vec!["a".to_string()].into();
  assert!(!name.is_empty());
}

#[test]
fn conversion_from_string_vector() {
  let name: Name = vec!["".to_string(), "".to_string(), "".to_string()].into();
  assert_eq!("", name.to_string());
  let name: Name = vec!["x".to_string(), "y".to_string()].into();
  assert_eq!("x y", name.to_string());
  let name: Name = vec!["x".to_string(), "+".to_string(), "y".to_string()].into();
  assert_eq!("x+y", name.to_string());
  let name: Name = vec!["x".to_string(), "    +    ".to_string(), "y".to_string()].into();
  assert_eq!("x+y", name.to_string());
  let name: Name = vec!["a".to_string(), "b".to_string(), "c".to_string()].into();
  assert_eq!("a b c", name.to_string());
  let name: Name = vec!["   x   ".to_string(), " y      \t".to_string(), "  \n  z  \t  ".to_string()].into();
  assert_eq!("x y z", name.to_string());
}

#[test]
fn conversion_from_str_vector() {
  let name: Name = vec!["", "", ""].into();
  assert_eq!("", name.to_string());
  let name: Name = vec!["x", "y"].into();
  assert_eq!("x y", name.to_string());
  let name: Name = vec!["x", "+", "y"].into();
  assert_eq!("x+y", name.to_string());
  let name: Name = vec!["a", "b", "c"].into();
  assert_eq!("a b c", name.to_string());
  let name: Name = vec!["   x   ", " y      \t", "  \n  z  \t  "].into();
  assert_eq!("x y z", name.to_string());
}

#[test]
fn from_parts_equals_new() {
  let cases: [&[&str]; 9] = [
    &[],
    &[""],
    &["x", "y"],
    &["   x   ", " y      \t", "  \n  z  \t  "],
    &["x", "   .    ", "y"],
    &[".", "x", "y", "-"],
    &["a", "+", "b", "*", "c", "/", "d", "'", "e"],
    &["x", "", "y", " "],
    &["\u{1680}x\u{1680}", "y\u{1680}z", "\u{1680}"],
  ];
  for parts in cases {
    let expected = Name::new(parts);
    assert_eq!(expected, Name::from_parts(parts), "{parts:?}");
    assert_eq!(expected, Name::from_parts(parts.iter().map(|part| part.to_string())), "{parts:?}");
    assert_eq!(expected, Name::from(parts.iter().map(|part| part.to_string()).collect::<Vec<String>>()), "{parts:?}");
  }
  // U+1680 is white space: trimmed at both ends of a part, kept inside it
  assert_eq!("x y\u{1680}z", Name::from_parts(["\u{1680}x\u{1680}", "y\u{1680}z", "\u{1680}"]).as_str());
}

#[test]
fn additional_symbols() {
  let name: Name = vec!["x", "y"].into();
  assert_eq!("x y", name.to_string());
  let name: Name = vec!["x", ".", "y"].into();
  assert_eq!("x.y", name.to_string());
  let name: Name = vec!["x", "   .    ", "y"].into();
  assert_eq!("x.y", name.to_string());
  let name: Name = vec![".", "x", "y"].into();
  assert_eq!(".x y", name.to_string());
  let name: Name = vec!["x", "y", "."].into();
  assert_eq!("x y.", name.to_string());
  let name: Name = vec!["x", "/", "y"].into();
  assert_eq!("x/y", name.to_string());
  let name: Name = vec!["x", "-", "y"].into();
  assert_eq!("x-y", name.to_string());
  let name: Name = vec!["x", "'", "y"].into();
  assert_eq!("x'y", name.to_string());
  let name: Name = vec!["x", "+", "y"].into();
  assert_eq!("x+y", name.to_string());
  let name: Name = vec!["x", "*", "y"].into();
  assert_eq!("x*y", name.to_string());
}

#[test]
fn test_name_as_hash_map_key() {
  let name_a: Name = "a".into();
  let name_b: Name = "b".into();
  let name_c: Name = "c".into();
  let mut map = HashMap::new();
  map.insert(name_a.clone(), "A".to_string());
  map.insert(name_b.clone(), "B".to_string());
  map.insert(name_c.clone(), "C".to_string());
  assert_eq!(3, map.len());
  assert!(map.contains_key(&name_a));
  assert_eq!("A", map.get(&name_a).unwrap());
  assert!(map.contains_key(&name_b));
  assert_eq!("B", map.get(&name_b).unwrap());
  assert!(map.contains_key(&name_c));
  assert_eq!("C", map.get(&name_c).unwrap());
  assert!(!map.contains_key(&Name::from("d")));
  // a name is looked up by its text too (`Borrow<str>`)
  assert!(map.contains_key("a") && !map.contains_key("d"));
}

#[test]
#[allow(clippy::nonminimal_bool)]
fn test_equality() {
  let name_a: Name = "a".into();
  let name_b: Name = "b".into();
  let name_x_y: Name = vec!["x", "y"].into();
  let name_m_n: Name = vec!["m", "n"].into();
  let name_xy: Name = vec!["x y"].into();
  let name_mn: Name = vec!["m n"].into();
  assert_eq!("a", name_a.to_string());
  assert_eq!("b", name_b.to_string());
  assert_eq!("x y", name_x_y.to_string());
  assert_eq!("m n", name_m_n.to_string());
  assert_eq!("x y", name_xy.to_string());
  assert_eq!("m n", name_m_n.to_string());
  assert!((name_a == name_a));
  assert!((name_x_y == name_x_y));
  assert!((name_x_y == name_xy));
  assert!((name_m_n == name_m_n));
  assert!((name_m_n == name_mn));
  assert!((name_a != name_b));
  assert!((name_x_y != name_m_n));
  assert!((name_xy != name_mn));
  assert!((name_xy != name_a));
  assert!((name_x_y != name_a));
  assert!((name_mn != name_b));
  assert!(!(name_a != name_a));
  assert!(!(name_x_y != name_x_y));
  assert!(!(name_x_y != name_xy));
  assert!(!(name_m_n != name_m_n));
  assert!(!(name_m_n != name_mn));
  assert!(!(name_a == name_b));
  assert!(!(name_x_y == name_m_n));
  assert!(!(name_xy == name_mn));
  assert!(!(name_x_y == name_a));
  assert!(!(name_xy == name_a));
  assert!(!(name_m_n == name_b));
  assert!((name_m_n != name_b));
  assert!(!(name_mn == name_b));
}

#[test]
fn test_compare() {
  let name_a: Name = "a".into();
  let name_b: Name = "b".into();
  let name_xy: Name = vec!["x y"].into();
  let name_mn: Name = vec!["m n"].into();
  assert_eq!("a", name_a.to_string());
  assert_eq!("b", name_b.to_string());
  assert_eq!("x y", name_xy.to_string());
  assert_eq!("m n", name_mn.to_string());
  assert!((name_a <= name_a));
  assert!((name_a < name_b));
  assert!((name_xy > name_mn));
  assert!((name_xy >= name_a));
}

/// A name is a list of tokens (DMN 1.6 clause 10.3.1.4): the white space between them does not
/// matter, and FEEL reads a name in one form.
#[test]
fn a_text_in_another_form_is_the_same_name() {
  use crate::names::{is_in_name_form, is_name, name_form};
  for text in ["a", "a b", "Date-Time", "a+b", "that's it", "a.b/c*d", "", "x1 y2", "Grace's age", "é ü"] {
    assert!(is_in_name_form(text), "{text:?}");
    assert_eq!(text, name_form(text), "{text:?}");
    assert!(is_name(text, text), "{text:?}");
  }
  for (text, form) in [
    ("Date - Time", "Date-Time"),
    ("Date -Time", "Date-Time"),
    ("Date- Time", "Date-Time"),
    ("a  b", "a b"),
    ("a\tb", "a b"),
    ("a\u{00A0}b", "a b"),
    ("a\u{1680}b", "a b"),
    ("a\u{FEFF}b", "a b"),
    (" a b ", "a b"),
    ("Profit / Loss", "Profit/Loss"),
    ("That 's  it", "That's it"),
    ("a + + b", "a++b"),
    ("a . b", "a.b"),
    ("  ", ""),
  ] {
    assert!(!is_in_name_form(text), "{text:?}");
    assert_eq!(form, name_form(text), "{text:?}");
    assert!(is_name(text, form), "{text:?}");
    assert!(is_name(form, form), "{text:?}");
    assert!(!is_name(text, text) || text.trim().is_empty(), "{text:?}");
  }
  // other names
  for (text, name) in [
    ("Date - Time", "Date Time"),
    ("a b", "ab"),
    ("ab", "a b"),
    ("a b c", "a b"),
    ("a b", "a b c"),
    ("a-b", "a b"),
    ("a", ""),
    ("", "a"),
  ] {
    assert!(!is_name(text, name), "{text:?} {name:?}");
  }
}

/// A context whose keys come from outside FEEL finds an entry by the name its key is; the keys
/// stay as they are, and a context that does not say so finds by the text only.
#[test]
fn a_context_with_keys_in_other_forms_finds_an_entry_by_its_name() {
  use crate::context::FeelContext;
  use crate::values::Value;
  let entries = || {
    [
      (Name::from("Date - Time"), Value::Boolean(true)),
      (Name::from("a  b"), Value::Boolean(false)),
      (Name::from("plain"), Value::Null(None)),
    ]
  };
  let exact: FeelContext = entries().into_iter().collect();
  assert!(!exact.has_other_key_forms());
  assert!(exact.get_entry(&Name::from("Date-Time")).is_none());
  assert!(exact.contains_entry("Date - Time"));
  let mut noted: FeelContext = entries().into_iter().collect();
  noted.note_key_forms();
  assert!(noted.has_other_key_forms());
  assert_eq!(Some(&Value::Boolean(true)), noted.get_entry(&Name::from("Date-Time")));
  assert_eq!(Some(&Value::Boolean(true)), noted.get_entry(&Name::from("Date - Time")));
  assert_eq!(Some(&Value::Boolean(false)), noted.get_entry(&Name::from("a b")));
  assert!(noted.contains_entry("a b") && noted.contains_entry("plain") && !noted.contains_entry("a c"));
  // the keys are unchanged, a clone keeps what was said, and equality is that of the entries
  assert_eq!(vec!["Date - Time", "a  b", "plain"], noted.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>());
  assert!(noted.clone().has_other_key_forms());
  assert_eq!(exact, noted);
  // a context whose keys are all in the form of a name has nothing to search
  let mut plain: FeelContext = [(Name::from("a b"), Value::Null(None))].into_iter().collect();
  plain.note_key_forms();
  assert!(!plain.has_other_key_forms());
}
