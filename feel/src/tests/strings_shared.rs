//! A FEEL string shares its text between copies and is never changed while it is shared.

use crate::values::{FeelString, Value};
use std::collections::BTreeSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[test]
fn a_copy_shares_the_text() {
  let a = FeelString::from("shared text");
  let b = a.clone();
  assert_eq!(a.as_ptr(), b.as_ptr());
  let value = Value::String(a.clone());
  let Value::String(c) = value.clone() else { unreachable!() };
  assert_eq!(a.as_ptr(), c.as_ptr());
}

#[test]
fn a_string_moves_in_and_out_without_a_copy() {
  let text = String::from("moved text");
  let ptr = text.as_ptr();
  let s = FeelString::from(text);
  assert_eq!(ptr, s.as_ptr());
  let text = s.into_string();
  assert_eq!(ptr, text.as_ptr());
  assert_eq!("moved text", text);
}

#[test]
fn into_string_of_a_shared_string_copies_it() {
  let a = FeelString::from("abc");
  let b = a.clone();
  let text = b.into_string();
  assert_ne!(a.as_ptr(), text.as_ptr());
  assert_eq!("abc", text);
  assert_eq!("abc", a.as_str());
}

#[test]
fn concat_of_a_shared_string_leaves_it_unchanged_and_allocates_exactly() {
  let a = FeelString::from("abc");
  let b = a.clone();
  let c = b.concat("defg");
  assert_eq!("abc", a.as_str());
  assert_eq!("abcdefg", c.as_str());
  assert_ne!(a.as_ptr(), c.as_ptr());
  assert_eq!(7, c.into_string().capacity());
}

#[test]
fn concat_with_itself_allocates_exactly() {
  let a = FeelString::from("ab");
  let b = a.clone().concat(&a);
  assert_eq!("ab", a.as_str());
  assert_eq!("abab", b.as_str());
  assert_eq!(4, b.into_string().capacity());
}

#[test]
fn concat_of_a_unique_string_appends_in_place() {
  let mut text = String::with_capacity(16);
  text.push_str("abc");
  let ptr = text.as_ptr();
  let s = FeelString::from(text).concat("def");
  assert_eq!(ptr, s.as_ptr());
  assert_eq!("abcdef", s.as_str());
  let text = s.into_string();
  assert_eq!(ptr, text.as_ptr());
  assert_eq!(16, text.capacity());
}

#[test]
fn debug_and_display_are_those_of_the_text() {
  let s = FeelString::from("a \"quoted\" text");
  assert_eq!(format!("{:?}", "a \"quoted\" text"), format!("{s:?}"));
  assert_eq!("a \"quoted\" text", format!("{s}"));
  assert_eq!(r#"String("beta")"#, format!("{:?}", Value::String("beta".into())));
}

#[test]
fn conversions() {
  let owned = String::from("abc");
  assert_eq!(FeelString::from("abc"), FeelString::from(&owned));
  assert_eq!(FeelString::from("abc"), FeelString::from(owned.clone()));
  assert_eq!(owned, String::from(FeelString::from("abc")));
  assert_eq!(Value::String("abc".into()), Value::from(FeelString::from("abc")));
  let s = FeelString::from("abc");
  assert!(s == "abc");
  assert!(s == *"abc");
  assert!(s == owned);
  assert_eq!("abc", s.as_ref() as &str);
  assert_eq!(3, s.len());
  assert_eq!("", FeelString::default().as_str());
}

/// A small deterministic generator of strings with one- to four-byte characters.
struct Rng(u64);

impl Rng {
  fn next(&mut self) -> u64 {
    self.0 ^= self.0 << 13;
    self.0 ^= self.0 >> 7;
    self.0 ^= self.0 << 17;
    self.0
  }

  fn text(&mut self) -> String {
    const CHARS: [char; 8] = ['a', 'b', 'Z', ' ', 'é', 'ß', '€', '😀'];
    let len = self.next() % 6;
    (0..len).map(|_| CHARS[(self.next() % CHARS.len() as u64) as usize]).collect()
  }
}

fn hash_of<T: Hash + ?Sized>(value: &T) -> u64 {
  let mut hasher = DefaultHasher::new();
  value.hash(&mut hasher);
  hasher.finish()
}

#[test]
fn equality_order_and_hash_are_those_of_the_text() {
  let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
  for _ in 0..2_000 {
    let (x, y) = (rng.text(), rng.text());
    let (a, b) = (FeelString::from(x.as_str()), FeelString::from(y.as_str()));
    assert_eq!(x == y, a == b, "{x:?} = {y:?}");
    assert_eq!(x.cmp(&y), a.cmp(&b), "{x:?} cmp {y:?}");
    assert_eq!(x.partial_cmp(&y), a.partial_cmp(&b), "{x:?} partial_cmp {y:?}");
    assert_eq!(hash_of(x.as_str()), hash_of(&a), "hash of {x:?}");
    assert_eq!(a, a.clone());
  }
}

#[test]
fn a_set_of_strings_is_searched_by_text() {
  let set: BTreeSet<FeelString> = ["b", "a", "c"].into_iter().map(FeelString::from).collect();
  assert!(set.contains("a"));
  assert!(!set.contains("d"));
  assert_eq!(vec!["a", "b", "c"], set.iter().map(FeelString::as_str).collect::<Vec<_>>());
}
