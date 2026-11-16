use super::*;

#[test]
fn test_0001() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "true and true", true);
}

#[test]
fn test_0002() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "true and true and true", true);
}

#[test]
fn test_0003() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "true and true and false", false);
}

#[test]
fn test_0004() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "true and false", false);
}

#[test]
fn test_0005() {
  let scope = &te_scope("{}");
  te_null(false, scope, "true and 1", "");
}

#[test]
fn test_0006() {
  let scope = &te_scope("{}");
  te_null(false, scope, r#" true and "true" "#, "");
}

#[test]
fn test_0007() {
  let scope = &te_scope("{}");
  te_null(false, scope, "true and null", "");
}

#[test]
fn test_0008() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "true and (false)", false);
}

#[test]
fn test_0009() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "false and true", false);
}

#[test]
fn test_0010() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "false and false", false);
}

#[test]
fn test_0011() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "false and 1", false);
}

#[test]
fn test_0012() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "false and null", false);
}

#[test]
fn test_0013() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "false and false", false);
}

#[test]
fn test_0014() {
  let scope = &te_scope("{}");
  te_bool(false, scope, r#" false and "true" "#, false);
}

#[test]
fn test_0015() {
  let scope = &te_scope("{}");
  te_null(false, scope, "1 and true", "");
}

#[test]
fn test_0016() {
  let scope = &te_scope("{}");
  te_null(false, scope, "null and true", "");
}

#[test]
fn test_0017() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "1 and false", false);
}

#[test]
fn test_0018() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "null and false", false);
}

#[test]
fn test_0019() {
  let scope = &te_scope("{}");
  te_null(false, scope, "1 and 1", "");
}

#[test]
fn test_0020() {
  let scope = &te_scope("{}");
  te_null(false, scope, "null and null", "");
}

#[test]
fn test_0021() {
  let scope = &te_scope("{}");
  te_null(false, scope, "10.2 and 10.2", "");
}

#[test]
fn test_0022() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "(1 < 2) and (3 > 1)", true);
}

#[test]
fn test_0023() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "(1 > 2) and (3 between 1 and 2) or true", true);
}

/// DMN 1.6 Table 51: `and` is a total function, "the input can be true, false, or otherwise", and
/// the table gives null for an operand that is no boolean unless the other operand decides
/// (`false and x` is false); a null the specification defines is no error.
#[test]
fn a_conjunction_with_a_non_boolean_operand_is_a_null_without_an_error() {
  te_errors("true and 1", "null", &[]);
  te_errors("1 and true", "null", &[]);
}

/// The value of `and` for every pair of operands of DMN 1.6 Table 51: true, false and otherwise
/// (here a number), in both orders.
#[test]
fn a_conjunction_follows_table_51() {
  for (lhs, rhs, expected) in [
    ("true", "true", "true"),
    ("true", "false", "false"),
    ("true", "1", "null"),
    ("false", "true", "false"),
    ("false", "false", "false"),
    ("false", "1", "false"),
    ("1", "true", "null"),
    ("1", "false", "false"),
    ("1", "1", "null"),
  ] {
    te_errors(&format!("{lhs} and {rhs}"), expected, &[]);
  }
}

/// The right operand of `and` is evaluated only when the left one does not decide (DMN 1.6
/// Table 51 gives false for `false and x` whatever x is): the errors and warnings of a right
/// operand after `false` are not recorded, and a recursion that `and` guards ends. The left
/// operand is always evaluated.
#[test]
fn the_right_operand_of_a_conjunction_is_evaluated_only_when_the_left_does_not_decide() {
  let addition = r#"incompatible types in addition: 1(number) + "a"(string)"#;
  te_errors(r#"false and (1 + "a")"#, "false", &[]);
  te_errors(r#"(1 + "a") and false"#, "false", &[addition]);
  te_errors(r#"true and (1 + "a")"#, "null", &[addition]);
  te_warnings(r#"false and @"2021-01-01T10:00:00.1234567891" = null"#, "false", &[]);
  te_errors("{f: function(n) n > 0 and f(n - 1), r: f(3)}.r", "false", &[]);
}
