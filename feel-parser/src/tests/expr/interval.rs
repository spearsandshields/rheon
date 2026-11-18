use super::super::*;

#[test]
fn _0001() {
  let scope = scope!();
  accept(
    &scope,
    StartExpression,
    "[1..10]",
    r#"
       Range
       ├─ IntervalStart (closed)
       │  └─ Numeric
       │     └─ `1`
       └─ IntervalEnd (closed)
          └─ Numeric
             └─ `10`
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
    "(1..10]",
    r#"
       Range
       ├─ IntervalStart (opened)
       │  └─ Numeric
       │     └─ `1`
       └─ IntervalEnd (closed)
          └─ Numeric
             └─ `10`
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
    "[1..10)",
    r#"
       Range
       ├─ IntervalStart (closed)
       │  └─ Numeric
       │     └─ `1`
       └─ IntervalEnd (opened)
          └─ Numeric
             └─ `10`
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
    "(1..10)",
    r#"
       Range
       ├─ IntervalStart (opened)
       │  └─ Numeric
       │     └─ `1`
       └─ IntervalEnd (opened)
          └─ Numeric
             └─ `10`
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
    "]1..10]",
    r#"
       Range
       ├─ IntervalStart (opened)
       │  └─ Numeric
       │     └─ `1`
       └─ IntervalEnd (closed)
          └─ Numeric
             └─ `10`
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
    "[1..10[",
    r#"
       Range
       ├─ IntervalStart (closed)
       │  └─ Numeric
       │     └─ `1`
       └─ IntervalEnd (opened)
          └─ Numeric
             └─ `10`
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
    "]1..10[",
    r#"
       Range
       ├─ IntervalStart (opened)
       │  └─ Numeric
       │     └─ `1`
       └─ IntervalEnd (opened)
          └─ Numeric
             └─ `10`
    "#,
    false,
  );
}

#[test]
fn _0008() {
  let scope = scope!();
  scope.set_entry_name("a".into());
  scope.set_entry_name("b".into());
  accept(
    &scope,
    StartExpression,
    "[a..b]",
    r#"
       Range
       ├─ IntervalStart (closed)
       │  └─ Name
       │     └─ `a`
       └─ IntervalEnd (closed)
          └─ Name
             └─ `b`
    "#,
    false,
  );
}

#[test]
fn _0009() {
  let frames = [input_context(&[], vec![("r", input_context(&["start", "end"], vec![]))])];
  let scope = ParsingScope::new(&frames);
  accept(
    &scope,
    StartExpression,
    "[r.start..r.end]",
    r#"
       Range
       ├─ IntervalStart (closed)
       │  └─ Path
       │     ├─ Name
       │     │  └─ `r`
       │     └─ Name
       │        └─ `start`
       └─ IntervalEnd (closed)
          └─ Path
             ├─ Name
             │  └─ `r`
             └─ Name
                └─ `end`
    "#,
    false,
  );
}

#[test]
fn _00010() {
  let scope = scope!();
  scope.set_entry_name("r".into());
  scope.set_entry_name("start".into());
  scope.set_entry_name("end".into());
  accept(
    &scope,
    StartExpression,
    "[r.start..r.end]",
    r#"
       Range
       ├─ IntervalStart (closed)
       │  └─ Path
       │     ├─ Name
       │     │  └─ `r`
       │     └─ Name
       │        └─ `start`
       └─ IntervalEnd (closed)
          └─ Path
             ├─ Name
             │  └─ `r`
             └─ Name
                └─ `end`
    "#,
    false,
  );
}

#[test]
fn _00011() {
  let scope = scope!();
  scope.set_entry_name("r".into());
  scope.set_entry_name("s".into());
  scope.set_entry_name("start".into());
  scope.set_entry_name("end".into());
  accept(
    &scope,
    StartExpression,
    "[r.start..r.s.end]",
    r#"
       Range
       ├─ IntervalStart (closed)
       │  └─ Path
       │     ├─ Name
       │     │  └─ `r`
       │     └─ Name
       │        └─ `start`
       └─ IntervalEnd (closed)
          └─ Path
             ├─ Path
             │  ├─ Name
             │  │  └─ `r`
             │  └─ Name
             │     └─ `s`
             └─ Name
                └─ `end`
    "#,
    false,
  );
}

/// A `[` after the start endpoint of an interval that starts with `]` is a filter, as after the
/// start endpoint of the other intervals; after an end endpoint it closes the interval.
#[test]
fn a_filter_follows_the_start_endpoint_of_any_interval() {
  let scope = scope!();
  for name in ["a", "b"] {
    scope.set_entry_name(name.into());
  }
  let tree = |input: &str| match Parser::new(&scope, StartExpression, input, false).parse() {
    Ok(node) => node.trace(),
    Err(reason) => panic!("{input}: {reason}"),
  };
  for (input, with_parentheses) in [
    ("]a[1]..b]", "](a[1])..b]"),
    ("]a[1]..b[", "](a[1])..b["),
    ("]a[1][2]..b[", "]((a[1])[2])..b["),
    ("]a[1].b..b[", "]((a[1]).b)..b["),
    ("[]a[1]..b[, 2]", "[(a[1]..b), 2]"),
    ("1 in ]a[1]..b[", "1 in ](a[1])..b["),
  ] {
    assert_eq!(tree(with_parentheses), tree(input), "{input}");
  }
  // the same endpoints as in the intervals that start with `[` and `(`
  assert_eq!(tree("[a[1]..b]").replace("(closed)", "(opened)"), tree("]a[1]..b["));
  assert_eq!(tree("(a[1]..b)"), tree("]a[1]..b["));
  // the `[` after the end endpoint closes the interval
  assert_eq!(tree("(a..b)"), tree("]a..b["));
  for input in ["]a..b[1]", "]a[1]", "]a[1]..", "]a[..b]"] {
    assert!(Parser::new(&scope, StartExpression, input, false).parse().is_err(), "{input}");
  }
}
