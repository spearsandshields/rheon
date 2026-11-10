use super::super::*;
use rheon_feel::scope;

// DMN 1.7 beta 1 (DMN17-96), adopted early; white space is grammar rule 61.

#[test]
fn _0001() {
  te_string(false, &scope!(), r#"trim(" abc ")"#, "abc");
  te_string(false, &scope!(), r#"trim("x ")"#, "x");
  te_string(false, &scope!(), r#"trim("")"#, "");
  te_string(false, &scope!(), r#"trim(string: "  a b  ")"#, "a b");
}

#[test]
fn _0002() {
  te_string(false, &scope!(), "trim(\"\\t\\n a\\u00A0\\u3000\")", "a");
}

#[test]
fn _0003() {
  te_null(false, &scope!(), "trim(null)", "");
}
