use super::super::*;

#[test]
fn _0001() {
  let scope = scope!();
  accept(
    &scope,
    StartExpression,
    "1.364 = 2.9483",
    r#"
       Eq
       ├─ Numeric
       │  └─ `1.364`
       └─ Numeric
          └─ `2.9483`
    "#,
    false,
  );
}

#[test]
fn _0002() {
  let scope = scope!();
  accept(
    &scope,
    StartExpression,
    "5.678 != 5.679",
    r#"
       Nq
       ├─ Numeric
       │  └─ `5.678`
       └─ Numeric
          └─ `5.679`
    "#,
    false,
  );
}

#[test]
fn _0003() {
  let scope = scope!();
  accept(
    &scope,
    StartExpression,
    "-23.45 < -5.28374658493",
    r#"
       Lt
       ├─ Neg
       │  └─ Numeric
       │     └─ `23.45`
       └─ Neg
          └─ Numeric
             └─ `5.28374658493`
    "#,
    false,
  );
}

#[test]
fn _0004() {
  let scope = scope!();
  accept(
    &scope,
    StartExpression,
    "-23.45 <= -5.28374658493",
    r#"
       Le
       ├─ Neg
       │  └─ Numeric
       │     └─ `23.45`
       └─ Neg
          └─ Numeric
             └─ `5.28374658493`
    "#,
    false,
  );
}

#[test]
fn _0005() {
  let scope = scope!();
  accept(
    &scope,
    StartExpression,
    "5 > 3",
    r#"
       Gt
       ├─ Numeric
       │  └─ `5`
       └─ Numeric
          └─ `3`
    "#,
    false,
  );
}

#[test]
fn _0006() {
  let scope = scope!();
  accept(
    &scope,
    StartExpression,
    "5 >= 3",
    r#"
       Ge
       ├─ Numeric
       │  └─ `5`
       └─ Numeric
          └─ `3`
    "#,
    false,
  );
}

#[test]
fn _0007() {
  let scope = scope!();
  accept(
    &scope,
    StartExpression,
    "5 between 1 and 10",
    r#"
       Between
       ├─ Numeric
       │  └─ `5`
       ├─ Numeric
       │  └─ `1`
       └─ Numeric
          └─ `10`
    "#,
    false,
  );
}

#[test]
fn _0008() {
  let scope = scope!();
  accept(
    &scope,
    StartExpression,
    "5 in 10",
    r#"
       In
       ├─ Numeric
       │  └─ `5`
       └─ Numeric
          └─ `10`
    "#,
    false,
  );
}

#[test]
fn _0009() {
  let scope = scope!();
  accept(
    &scope,
    StartExpression,
    "5 in <10",
    r#"
       In
       ├─ Numeric
       │  └─ `5`
       └─ UnaryLt
          └─ Numeric
             └─ `10`
    "#,
    false,
  );
}

#[test]
fn _0010() {
  let scope = scope!();
  accept(
    &scope,
    StartExpression,
    "5 in <=10",
    r#"
       In
       ├─ Numeric
       │  └─ `5`
       └─ UnaryLe
          └─ Numeric
             └─ `10`
    "#,
    false,
  );
}

#[test]
fn _0011() {
  let scope = scope!();
  accept(
    &scope,
    StartExpression,
    "18 in >8",
    r#"
       In
       ├─ Numeric
       │  └─ `18`
       └─ UnaryGt
          └─ Numeric
             └─ `8`
    "#,
    false,
  );
}

#[test]
fn _0012() {
  let scope = scope!();
  accept(
    &scope,
    StartExpression,
    "18 in >=8",
    r#"
       In
       ├─ Numeric
       │  └─ `18`
       └─ UnaryGe
          └─ Numeric
             └─ `8`
    "#,
    false,
  );
}

#[test]
fn _0013() {
  let scope = scope!();
  accept(
    &scope,
    StartExpression,
    "18 in [1..100]",
    r#"
       In
       ├─ Numeric
       │  └─ `18`
       └─ Range
          ├─ IntervalStart (closed)
          │  └─ Numeric
          │     └─ `1`
          └─ IntervalEnd (closed)
             └─ Numeric
                └─ `100`
    "#,
    false,
  );
}

#[test]
fn _0014() {
  let scope = scope!();
  accept(
    &scope,
    StartExpression,
    "18 in (1..100)",
    r#"
       In
       ├─ Numeric
       │  └─ `18`
       └─ Range
          ├─ IntervalStart (opened)
          │  └─ Numeric
          │     └─ `1`
          └─ IntervalEnd (opened)
             └─ Numeric
                └─ `100`
    "#,
    false,
  );
}

#[test]
fn _0015() {
  let scope = scope!();
  accept(
    &scope,
    StartExpression,
    "5 in (1,2,3,4,5,6,7)",
    r#"
       In
       ├─ Numeric
       │  └─ `5`
       └─ ExpressionList
          ├─ Numeric
          │  └─ `1`
          ├─ Numeric
          │  └─ `2`
          ├─ Numeric
          │  └─ `3`
          ├─ Numeric
          │  └─ `4`
          ├─ Numeric
          │  └─ `5`
          ├─ Numeric
          │  └─ `6`
          └─ Numeric
             └─ `7`
    "#,
    false,
  );
}

/// Grammar rule 49: the comparisons are one level of precedence and left associative.
#[test]
fn comparisons_are_one_left_associative_level() {
  let scope = scope!();
  for name in ["a", "b", "c", "x"] {
    scope.set_entry_name(name.into());
  }
  let tree = |input: &str| match Parser::new(&scope, StartExpression, input).parse() {
    Ok(node) => node.trace(),
    Err(reason) => panic!("{input}: {reason}"),
  };
  for (input, with_parentheses) in [
    ("1 < 2 < 3", "(1 < 2) < 3"),
    ("a = b = true", "(a = b) = true"),
    ("a != b = c", "(a != b) = c"),
    ("a = b in c", "(a = b) in c"),
    ("a in b = c", "(a in b) = c"),
    ("a in b in c", "(a in b) in c"),
    ("a in (b, c) in x", "(a in (b, c)) in x"),
    ("a in (b) in c", "(a in (b)) in c"),
    ("a = x between 1 and 2", "(a = x) between 1 and 2"),
    ("x between 1 and 2 = a", "(x between 1 and 2) = a"),
    ("x between 1 and 2 in a", "(x between 1 and 2) in a"),
    ("x between 1 and 2 between a and b", "(x between 1 and 2) between a and b"),
    ("x in a between 1 and 2", "(x in a) between 1 and 2"),
    // the operands are arithmetic expressions
    ("a + 1 = b * 2 in c - 1", "((a + 1) = (b * 2)) in (c - 1)"),
    ("x between a + 1 and b + 1 = c + 1", "(x between (a + 1) and (b + 1)) = (c + 1)"),
    // a conjunction or a disjunction takes comparisons as its operands
    ("a = b and b in c or x between 1 and 2", "((a = b) and (b in c)) or (x between 1 and 2)"),
  ] {
    assert_eq!(tree(with_parentheses), tree(input), "{input}");
  }
}
