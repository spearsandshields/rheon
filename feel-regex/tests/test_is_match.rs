use dsntk_feel_regex::is_match;

#[test]
fn _0001() {
  assert!(is_match(r#"Mr\. Obama"#, ".+Obama").unwrap());
}

#[test]
fn _0002() {
  assert!(is_match(r#"banana"#, "[a-z]{3}").unwrap());
}

#[test]
fn back_references() {
  // XPath/XSD regular expressions allow back-references to capture groups.
  assert!(is_match("aa", r"(a)\1").unwrap());
  assert!(!is_match("ab", r"(a)\1").unwrap());
  assert!(is_match("abcabc", r"^(abc)\1$").unwrap());
}
