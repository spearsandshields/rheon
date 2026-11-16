use super::*;

#[test]
fn test_0001() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "true or true", true);
}

#[test]
fn test_0002() {
  let scope = &te_scope("{}");
  super::te_bool(false, scope, "true or not(false)", true);
}

#[test]
fn test_0003() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "true or true or true", true);
}

#[test]
fn test_0004() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "true or true or false", true);
}

#[test]
fn test_0005() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "true or false", true);
}

#[test]
fn test_0006() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "true or 1", true);
}

#[test]
fn test_0007() {
  let scope = &te_scope("{}");
  te_bool(false, scope, r#" true or "true" "#, true);
}

#[test]
fn test_0008() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "true or null", true);
}

#[test]
fn test_0009() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "false or true", true);
}

#[test]
fn test_0010() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "false or false", false);
}

#[test]
fn test_0011() {
  let scope = &te_scope("{}");
  te_null(false, scope, "false or 1", "");
}

#[test]
fn test_0012() {
  let scope = &te_scope("{}");
  te_null(false, scope, "false or null", "");
}

#[test]
fn test_0013() {
  let scope = &te_scope("{}");
  te_null(false, scope, r#" false or "true" "#, "");
}

#[test]
fn test_0014() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "1 or true", true);
}

#[test]
fn test_0015() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "null or true", true);
}

#[test]
fn test_0016() {
  let scope = &te_scope("{}");
  te_null(false, scope, "1 or false", "");
}

#[test]
fn test_0017() {
  let scope = &te_scope("{}");
  te_null(false, scope, "null or false", "");
}

#[test]
fn test_0018() {
  let scope = &te_scope("{}");
  te_null(false, scope, "1 or 1", "");
}

#[test]
fn test_0019() {
  let scope = &te_scope("{}");
  te_null(false, scope, "null or null", "");
}

#[test]
fn test_0020() {
  let scope = &te_scope("{}");
  te_null(false, scope, "10.2 or 10.2", "");
}

#[test]
fn test_0021() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "(1 < 2) or (3 > 1)", true);
}

#[test]
fn test_0022() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "((1 < 2) or (3 > 1)) and false", false);
}

#[test]
fn test_0023() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "((1 < 2) or (3 > 1)) or false", true);
}

/// DMN 1.6 Table 51: `or` is a total function, "the input can be true, false, or otherwise", and
/// the table gives null for an operand that is no boolean unless the other operand decides
/// (`true or x` is true); a null the specification defines is no error.
#[test]
fn a_disjunction_with_a_non_boolean_operand_is_a_null_without_an_error() {
  te_errors(r#"false or "x""#, "null", &[]);
  te_errors("1 or 1", "null", &[]);
}

/// The value of `or` for every pair of operands of DMN 1.6 Table 51: true, false and otherwise
/// (here a number), in both orders.
#[test]
fn a_disjunction_follows_table_51() {
  for (lhs, rhs, expected) in [
    ("true", "true", "true"),
    ("true", "false", "true"),
    ("true", "1", "true"),
    ("false", "true", "true"),
    ("false", "false", "false"),
    ("false", "1", "null"),
    ("1", "true", "true"),
    ("1", "false", "null"),
    ("1", "1", "null"),
  ] {
    te_errors(&format!("{lhs} or {rhs}"), expected, &[]);
  }
}

/// The right operand of `or` is evaluated only when the left one does not decide (DMN 1.6
/// Table 51 gives true for `true or x` whatever x is): the errors and warnings of a right operand
/// after `true` are not recorded, and a recursion that `or` guards ends. The left operand is
/// always evaluated.
#[test]
fn the_right_operand_of_a_disjunction_is_evaluated_only_when_the_left_does_not_decide() {
  let addition = r#"incompatible types in addition: 1(number) + "a"(string)"#;
  te_errors(r#"true or (1 + "a")"#, "true", &[]);
  te_errors(r#"(1 + "a") or true"#, "true", &[addition]);
  te_errors(r#"false or (1 + "a")"#, "null", &[addition]);
  te_warnings(r#"true or @"2021-01-01T10:00:00.1234567891" = null"#, "true", &[]);
  te_errors("{f: function(n) n <= 0 or f(n - 1), r: f(3)}.r", "true", &[]);
}
