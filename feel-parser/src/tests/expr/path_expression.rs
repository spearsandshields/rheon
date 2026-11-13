use super::super::*;

#[test]
fn _0001() {
  let input = "1.first";
  let expected = r#"
       Path
       ├─ Numeric
       │  └─ `1`
       └─ Name
          └─ `first`
    "#;
  accept(&scope!(), StartExpression, input, expected, false);
}

#[test]
fn _0002() {
  let scope = scope!();
  scope.set_entry_name("Manager".into());
  accept(
    &scope,
    StartExpression,
    r#"Manager.Name"#,
    r#"
       Path
       ├─ Name
       │  └─ `Manager`
       └─ Name
          └─ `Name`
    "#,
    false,
  );
}

#[test]
fn _0003() {
  let scope = scope!();
  scope.set_entry_name("Manager".into());
  accept(
    &scope,
    StartExpression,
    r#"Manager.Address.Street"#,
    r#"
       Path
       ├─ Name
       │  └─ `Manager`
       └─ Name
          └─ `Address.Street`
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
    r#"(Manager.Address).Street"#,
    r#"
       Path
       ├─ Name
       │  └─ `Manager.Address`
       └─ Name
          └─ `Street`
    "#,
    false,
  );
}

#[test]
fn _0005() {
  let scope = scope!();
  scope.set_entry_name("Manager".into());
  scope.set_entry_name("Address".into());
  accept(
    &scope,
    StartExpression,
    r#"Manager.Address.Street"#,
    r#"
       Path
       ├─ Path
       │  ├─ Name
       │  │  └─ `Manager`
       │  └─ Name
       │     └─ `Address`
       └─ Name
          └─ `Street`
    "#,
    false,
  );
}

#[test]
fn _0006() {
  let scope = scope!();
  scope.set_entry_name("Manager".into());
  scope.set_entry_name("Address".into());
  scope.set_entry_name("City".into());
  scope.set_entry_name("Street".into());
  accept(
    &scope,
    StartExpression,
    r#"Manager.Address.City.Street"#,
    r#"
       Path
       ├─ Path
       │  ├─ Path
       │  │  ├─ Name
       │  │  │  └─ `Manager`
       │  │  └─ Name
       │  │     └─ `Address`
       │  └─ Name
       │     └─ `City`
       └─ Name
          └─ `Street`
    "#,
    false,
  );
}

#[test]
fn _0007() {
  let frames = [input_context(
    &[],
    vec![("Manager", input_context(&[], vec![("Address", input_context(&["Street"], vec![]))]))],
  )];
  let scope = ParsingScope::new(&frames);
  accept(
    &scope,
    StartExpression,
    r#"Manager.Address.Street"#,
    r#"
       Path
       ├─ Path
       │  ├─ Name
       │  │  └─ `Manager`
       │  └─ Name
       │     └─ `Address`
       └─ Name
          └─ `Street`
    "#,
    false,
  );
}

#[test]
fn _0008() {
  let frames = [input_context(&[], vec![("loan", input_context(&["principal"], vec![]))])];
  let scope = ParsingScope::new(&frames);
  accept(
    &scope,
    StartExpression,
    r#"(loan.principal)"#,
    r#"
       Path
       ├─ Name
       │  └─ `loan`
       └─ Name
          └─ `principal`
    "#,
    false,
  );
}

#[test]
fn _0009() {
  let scope = scope!();
  scope.set_entry_name("principal".into());
  scope.set_entry_name("loan".into());
  scope.set_entry_name("id".into());
  accept(
    &scope,
    StartExpression,
    r#"(loan.principal.id)"#,
    r#"
       Path
       ├─ Path
       │  ├─ Name
       │  │  └─ `loan`
       │  └─ Name
       │     └─ `principal`
       └─ Name
          └─ `id`
    "#,
    false,
  );
}

#[test]
fn _0010() {
  let scope = scope!();
  scope.set_entry_name("principal".into());
  scope.set_entry_name("loan".into());
  scope.set_entry_name("id".into());
  scope.set_entry_name("type".into());
  accept(
    &scope,
    StartExpression,
    r#"(loan.principal.id.type - 1)"#,
    r#"
       Sub
       ├─ Path
       │  ├─ Path
       │  │  ├─ Path
       │  │  │  ├─ Name
       │  │  │  │  └─ `loan`
       │  │  │  └─ Name
       │  │  │     └─ `principal`
       │  │  └─ Name
       │  │     └─ `id`
       │  └─ Name
       │     └─ `type`
       └─ Numeric
          └─ `1`
    "#,
    false,
  );
}

#[test]
fn _0011() {
  let frames = [input_context(&[], vec![("loan", input_context(&["principal", "rate", "termMonths"], vec![]))])];
  let scope = ParsingScope::new(&frames);
  accept(
    &scope,
    StartExpression,
    r#"(loan.principal) + (loan.rate)"#,
    r#"
       Add
       ├─ Path
       │  ├─ Name
       │  │  └─ `loan`
       │  └─ Name
       │     └─ `principal`
       └─ Path
          ├─ Name
          │  └─ `loan`
          └─ Name
             └─ `rate`
    "#,
    false,
  );
}

#[test]
fn _0012() {
  let frames = [input_context(&[], vec![("Date", input_context(&["fromString"], vec![]))])];
  let scope = ParsingScope::new(&frames);
  accept(
    &scope,
    StartExpression,
    r#"Date.fromString.day"#,
    r#"
       Path
       ├─ Path
       │  ├─ Name
       │  │  └─ `Date`
       │  └─ Name
       │     └─ `fromString`
       └─ Name
          └─ `day`
    "#,
    false,
  );
}

#[test]
fn _0013() {
  let tenor_rate = || input_context(&["Tenor", "Rate"], vec![]);
  let bounds = input_context(&[], vec![("Min", tenor_rate()), ("Max", tenor_rate())]);
  let frames = [input_context(&["Days"], vec![("Bounds", bounds)])];
  let scope = ParsingScope::new(&frames);
  let input = "((Bounds.Max.Rate - Bounds.Min.Rate) * (Days - Bounds.Min.Tenor) / (Bounds.Max.Tenor - Bounds.Min.Tenor)) + Bounds.Min.Rate";
  let expected = r#"
       Add
       ├─ Div
       │  ├─ Mul
       │  │  ├─ Sub
       │  │  │  ├─ Path
       │  │  │  │  ├─ Path
       │  │  │  │  │  ├─ Name
       │  │  │  │  │  │  └─ `Bounds`
       │  │  │  │  │  └─ Name
       │  │  │  │  │     └─ `Max`
       │  │  │  │  └─ Name
       │  │  │  │     └─ `Rate`
       │  │  │  └─ Path
       │  │  │     ├─ Path
       │  │  │     │  ├─ Name
       │  │  │     │  │  └─ `Bounds`
       │  │  │     │  └─ Name
       │  │  │     │     └─ `Min`
       │  │  │     └─ Name
       │  │  │        └─ `Rate`
       │  │  └─ Sub
       │  │     ├─ Name
       │  │     │  └─ `Days`
       │  │     └─ Path
       │  │        ├─ Path
       │  │        │  ├─ Name
       │  │        │  │  └─ `Bounds`
       │  │        │  └─ Name
       │  │        │     └─ `Min`
       │  │        └─ Name
       │  │           └─ `Tenor`
       │  └─ Sub
       │     ├─ Path
       │     │  ├─ Path
       │     │  │  ├─ Name
       │     │  │  │  └─ `Bounds`
       │     │  │  └─ Name
       │     │  │     └─ `Max`
       │     │  └─ Name
       │     │     └─ `Tenor`
       │     └─ Path
       │        ├─ Path
       │        │  ├─ Name
       │        │  │  └─ `Bounds`
       │        │  └─ Name
       │        │     └─ `Min`
       │        └─ Name
       │           └─ `Tenor`
       └─ Path
          ├─ Path
          │  ├─ Name
          │  │  └─ `Bounds`
          │  └─ Name
          │     └─ `Min`
          └─ Name
             └─ `Rate`
    "#;
  accept(&scope, StartExpression, input, expected, false);
}

#[test]
fn _0014() {
  let input = "{first: 10, second: 20}.first";
  let expected = r#"
       Path
       ├─ Context
       │  ├─ ContextEntry
       │  │  ├─ ContextEntryKey
       │  │  │  └─ `first`
       │  │  └─ Numeric
       │  │     └─ `10`
       │  └─ ContextEntry
       │     ├─ ContextEntryKey
       │     │  └─ `second`
       │     └─ Numeric
       │        └─ `20`
       └─ Name
          └─ `first`
    "#;
  accept(&scope!(), StartExpression, input, expected, false);
}

#[test]
fn _0015() {
  // A long chain of path expressions over a context literal: every `.` is an additional name
  // symbol, so a lexer that scanned the rest of the chain for each token and tried every prefix
  // would be cubic. The parts scanned and the candidates looked up grow, and grow linearly, with
  // the steps.
  let work = |steps: usize| {
    let input = format!("{{a:1}}{}", ".a".repeat(steps));
    crate::tests::work::take();
    let node = Parser::new(&scope!(), StartExpression, &input, false).parse().unwrap();
    assert!(matches!(node, AstNode::Path(_, _)), "expected a path expression");
    crate::tests::work::take()
  };
  let (w1, w2, w3) = (work(1), work(1_001), work(2_001));
  for unit in ["parts", "candidates"] {
    let n = |w: &std::collections::BTreeMap<&str, usize>| w.get(unit).copied().unwrap_or(0);
    assert!(n(&w2) > n(&w1), "{unit} does not grow: {w1:?} {w2:?} {w3:?}");
    assert_eq!(n(&w3) - n(&w2), n(&w2) - n(&w1), "{unit}: {w1:?} {w2:?} {w3:?}");
  }
}

#[test]
fn _0016() {
  // Names containing additional symbols are still recognised when they exist in scope.
  let scope = scope!();
  scope.set_entry_name("a.b".into());
  scope.set_entry_name("start included".into());
  accept(
    &scope,
    StartExpression,
    r#"a.b.start included"#,
    r#"
       Path
       ├─ Name
       │  └─ `a.b`
       └─ Name
          └─ `start included`
    "#,
    false,
  );
}
