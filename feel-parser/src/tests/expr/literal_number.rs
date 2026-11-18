use super::super::*;

#[test]
fn _0001() {
  let input = "2";
  let expected = r#"
       Numeric
       └─ `2`
    "#;
  accept(&scope!(), StartExpression, input, expected, false);
}

#[test]
fn _0002() {
  let input = "2.3";
  let expected = r#"
       Numeric
       └─ `2.3`
    "#;
  accept(&scope!(), StartExpression, input, expected, false);
}

#[test]
fn _0003() {
  let input = ".2";
  let expected = r#"
       Numeric
       └─ `0.2`
    "#;
  accept(&scope!(), StartExpression, input, expected, false);
}

#[test]
fn _0004() {
  let input = "1";
  let expected = r#"
       Numeric
       └─ `1`
    "#;
  accept(&scope!(), StartExpression, input, expected, false);
}

#[test]
fn _0005() {
  let input = "(1.32983740938573405329458372450983275)";
  let expected = r#"
       Numeric
       └─ `1.32983740938573405329458372450983275`
    "#;
  accept(&scope!(), StartExpression, input, expected, false);
}

#[test]
fn _0006() {
  let input = "-14";
  let expected = r#"
       Neg
       └─ Numeric
          └─ `14`
    "#;
  accept(&scope!(), StartExpression, input, expected, false);
}

#[test]
fn _0007() {
  let input = "1.0";
  let expected = r#"
       Numeric
       └─ `1.0`
    "#;
  accept(&scope!(), StartExpression, input, expected, false);
}

#[test]
fn _0008() {
  let input = "1.23e1";
  let expected = r#"
       Numeric
       └─ `1.23e+1`
    "#;
  accept(&scope!(), StartExpression, input, expected, false);
}

#[test]
fn _0009() {
  let input = "1.23e+1";
  let expected = r#"
       Numeric
       └─ `1.23e+1`
    "#;
  accept(&scope!(), StartExpression, input, expected, false);
}

#[test]
fn _0010() {
  let input = "1.23e-1";
  let expected = r#"
       Numeric
       └─ `1.23e-1`
    "#;
  accept(&scope!(), StartExpression, input, expected, false);
}

#[test]
fn _0011() {
  let input = "1e-10";
  let expected = r#"
       Numeric
       └─ `1e-10`
    "#;
  accept(&scope!(), StartExpression, input, expected, false);
}

#[test]
fn _0012() {
  let input = "1E-10";
  let expected = r#"
       Numeric
       └─ `1e-10`
    "#;
  accept(&scope!(), StartExpression, input, expected, false);
}

#[test]
fn _0013() {
  let input = "1.23E-1";
  let expected = r#"
       Numeric
       └─ `1.23e-1`
    "#;
  accept(&scope!(), StartExpression, input, expected, false);
}

/// The letter of an exponent belongs to a number only when exponent digits follow it, after an
/// optional sign (grammar rule 35): otherwise the number ends before the letter.
#[test]
fn exponent_letter_needs_exponent_digits() {
  let scope = scope!();
  let parse = |input: &str| Parser::new(&scope, StartExpression, input).parse().map(|node| node.trace());
  for input in ["1e", "1e+", "1E-", "1.5e", "1e + 2", "1e+ 3", ".5e", "1e.5", "1ee5", "1e+-5"] {
    assert!(parse(input).is_err(), "{input}: {:?}", parse(input));
  }
  for (input, digits) in [("1e5", "`1e+5`"), ("1E+5", "`1e+5`"), ("1.5e-10", "`1.5e-10`"), ("2e0", "`2e+0`"), (".5E3", "`0.5e+3`")] {
    let tree = parse(input).unwrap();
    assert!(tree.contains("Numeric") && tree.contains(digits), "{input}: {tree}");
  }
  // the letter starts the next token
  let conditional = parse("if true then 1else 2").unwrap();
  assert!(conditional.contains("If"), "{conditional}");
}
