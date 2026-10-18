use dsntk_feel_regex::is_match_with_flags;

#[test]
fn _0001() {
  assert!(is_match_with_flags("Mr. B. Obama", "B. Obama", "iq").unwrap());
}

#[test]
fn _0002() {
  assert!(!is_match_with_flags("Mr. B, Obama", "B. OBAMA", "iq").unwrap());
}

#[test]
fn _0003() {
  assert!(!is_match_with_flags("abcd", ".*", "q").unwrap());
}

#[test]
fn _0004() {
  assert!(is_match_with_flags("hello world", r#"\p{ IsBasicLatin}+"#, "x").unwrap());
}

#[test]
fn _0005() {
  assert!(is_match_with_flags("hello world", r#" hello[ ]world"#, "x").unwrap());
}

#[test]
fn _0006() {
  assert!(!is_match_with_flags("i", r#"[A-Z]-[OI]"#, "i").unwrap());
}

#[test]
fn back_reference_with_caseless_flag() {
  // TCK 1111 K2-MatchesFunc-17
  assert!(is_match_with_flags("aA", r"(a)\1", "i").unwrap());
}
