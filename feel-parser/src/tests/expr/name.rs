use super::super::*;

/// Compares the given `name` with `expected` name after parsing.
fn accept_name(scope: &ParsingScope, name: &str, expected: &str) {
  accept(scope, StartExpression, name, &format!("\n       Name\n       └─ `{expected}`\n    "), false);
}

#[test]
fn _0001() {
  let scope = scope!();
  scope.set_entry_name("hello world".into());
  accept(
    &scope,
    StartExpression,
    "hello world",
    r#"
       Name
       └─ `hello world`
    "#,
    false,
  );
}

#[test]
fn _0002() {
  let scope = scope!();
  scope.set_entry_name("  \n  \n  \t  thing \n \t \t ".into());
  accept(
    &scope,
    StartExpression,
    "thing",
    r#"
       Name
       └─ `thing`
    "#,
    false,
  );
}

#[test]
fn _0003() {
  let scope = scope!();
  scope.set_entry_name("income/loss".into());
  accept(
    &scope,
    StartExpression,
    "income/loss",
    r#"
       Name
       └─ `income/loss`
    "#,
    false,
  );
}

#[test]
fn _0004() {
  let scope = scope!();
  scope.set_entry_name("fr**n*s".into());
  accept(
    &scope,
    StartExpression,
    "fr**n*s",
    r#"
       Name
       └─ `fr**n*s`
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
    "book",
    r#"
       Name
       └─ `book`
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
    "profit/loss",
    r#"
       Name
       └─ `profit/loss`
    "#,
    false,
  );
}

#[test]
fn _0007() {
  let scope = scope!();
  scope.set_entry_name("before.after".into());
  accept(
    &scope,
    StartExpression,
    "before.after",
    r#"
       Name
       └─ `before.after`
    "#,
    false,
  );
}

#[test]
fn _0008() {
  let scope = rheon_feel::FeelScope::default();
  assert!(crate::parse_name(&scope, "5Cards", false).is_err());
}

#[test]
fn _0009() {
  let scope = rheon_feel::FeelScope::default();
  assert!(crate::parse_name(&scope, "123.45", false).is_err());
}

#[test]
fn _0010() {
  let scope = scope!();
  scope.set_entry_name("thing".into());
  scope.set_entry_name("one two three four".into());
  scope.set_entry_name("one and two".into());
  scope.set_entry_name("one or two".into());
  scope.set_entry_name("before.after".into());
  scope.set_entry_name("before.or.after".into());
  scope.set_entry_name("before.and.after".into());
  scope.set_entry_name("before.between.after".into());
  scope.set_entry_name("before.next to between.worm".into());
  scope.set_entry_name("income/loss".into());
  scope.set_entry_name("per/month/income/loss".into());
  scope.set_entry_name("a-b".into());
  scope.set_entry_name("to-be-or-not-to-be".into());
  scope.set_entry_name("that's".into());
  scope.set_entry_name("ok that's".into());
  scope.set_entry_name("bed+breakfast".into());
  scope.set_entry_name("night+and+day".into());
  scope.set_entry_name("fr**n*s".into());
  scope.set_entry_name("bo++e--m*".into());
  scope.set_entry_name("wh*t*v*r".into());
  accept_name(&scope, "thing", "thing");
  accept_name(&scope, "one two three four", "one two three four");
  accept_name(&scope, "one and two", "one and two");
  accept_name(&scope, "one or two", "one or two");
  accept_name(&scope, "before.after", "before.after");
  accept_name(&scope, "before.or.after", "before.or.after");
  accept_name(&scope, "before.and.after", "before.and.after");
  accept_name(&scope, "before.between.after", "before.between.after");
  accept_name(&scope, "before.next to between.worm", "before.next to between.worm");
  accept_name(&scope, "income/loss", "income/loss");
  accept_name(&scope, "per/month/income/loss", "per/month/income/loss");
  accept_name(&scope, "a-b", "a-b");
  accept_name(&scope, "to-be-or-not-to-be", "to-be-or-not-to-be");
  accept_name(&scope, "that's", "that's");
  accept_name(&scope, "ok that's", "ok that's");
  accept_name(&scope, "bed+breakfast", "bed+breakfast");
  accept_name(&scope, "night+and+day", "night+and+day");
  accept_name(&scope, "fr**n*s", "fr**n*s");
  accept_name(&scope, "bo++e--m*", "bo++e--m*");
  accept_name(&scope, "wh*t*v*r", "wh*t*v*r");
}

#[test]
fn _0011() {
  let scope = scope!();
  scope.set_entry_name("?".into());
  accept(
    &scope,
    StartExpression,
    "?.Rating > 0",
    r#"
       Gt
       ├─ Path
       │  ├─ Name
       │  │  └─ `?`
       │  └─ Name
       │     └─ `Rating`
       └─ Numeric
          └─ `0`
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
    "?",
    r#"
       Name
       └─ `?`
    "#,
    false,
  );
}

#[test]
fn _0013() {
  let scope = scope!();
  scope.set_entry_name("_0001-input-data-string".into());
  accept(
    &scope,
    StartExpression,
    "_0001-input-data-string",
    r#"
       Name
       └─ `_0001-input-data-string`
    "#,
    false,
  );
}

/// Returns the tree of `input` parsed with the `names` in scope.
fn tree(names: &[&str], input: &str) -> std::string::String {
  let scope = scope!();
  for name in names {
    scope.set_entry_name((*name).into());
  }
  match Parser::new(&scope, StartExpression, input, false).parse() {
    Ok(node) => node.trace(),
    Err(reason) => format!("{reason}"),
  }
}

/// A name that is not in scope ends before a word that is a keyword which can follow an operand
/// (`return`, `then`, `else`, `satisfies`, `instance`, `in`, `and`, `or`, `between`), so the
/// expression has the tree it has when the name is in scope.
#[test]
fn unknown_name_ends_before_a_keyword() {
  for (input, names) in [
    ("for s in x return 1", &["x"][..]),
    ("for s in x y return s", &["x y"]),
    ("for s in x.y return s.z", &["x.y"]),
    ("for s in x, t in y return s", &["x", "y"]),
    ("some s in x satisfies s.z", &["x"]),
    ("every s in x satisfies s", &["x"]),
    ("if x then y else z", &["x", "y", "z"]),
    ("if a b then c d else e f", &["a b", "c d", "e f"]),
    ("x and y", &["x", "y"]),
    ("x or y", &["x", "y"]),
    ("x and y or z", &["x", "y", "z"]),
    ("x in y", &["x", "y"]),
    ("x between y and z", &["x", "y", "z"]),
    ("x instance of number", &["x"]),
    ("x and", &["x"]),
    ("x return", &["x"]),
    ("x\tand\ny", &["x", "y"]),
  ] {
    assert_eq!(tree(names, input), tree(&[], input), "{input}");
    // with some of the names in scope
    for name in names {
      assert_eq!(tree(names, input), tree(&[name], input), "{input} with {name}");
    }
  }
}

/// A name in scope that has a keyword is still read whole, also before another keyword.
#[test]
fn name_in_scope_with_a_keyword_is_read_whole() {
  let names = ["tax return", "x and y", "go in then out"];
  assert_eq!("\n       Name\n       └─ `tax return`\n    ", tree(&names, "tax return"));
  assert_eq!("\n       Name\n       └─ `x and y`\n    ", tree(&names, "x and y"));
  assert_eq!("\n       Name\n       └─ `go in then out`\n    ", tree(&names, "go in then out"));
  assert_eq!(
    tree(&["a", "b"], "for t in a return b").replace("`a`", "`tax return`"),
    tree(&names, "for t in tax return return b")
  );
  assert_eq!(tree(&["a", "b"], "a and b").replace("`a`", "`tax return`"), tree(&names, "tax return and b"));
  assert_eq!(tree(&["a", "b"], "a and b").replace("`a`", "`x and y`"), tree(&names, "x and y and b"));
}

/// An unknown name keeps the words that are no keyword where they stand: a keyword that never
/// follows an operand, a keyword without white space on both sides, and a longer word.
#[test]
fn unknown_name_keeps_other_words() {
  for input in [
    "price list",
    "x for y",
    "x if y",
    "x of y",
    "x true",
    "x null",
    "opt-in rate",
    "x and-y",
    "x and(y)",
    "x android",
    "x band y",
    "returns x",
  ] {
    let tree = tree(&[], input);
    assert!(
      tree.starts_with("\n       Name\n       └─ `") || tree.starts_with("\n       FunctionInvocation\n       ├─ Name\n       │  └─ `x and`"),
      "{input}: {tree}"
    );
  }
  assert_eq!("\n       Name\n       └─ `price list`\n    ", tree(&[], "price list"));
  assert_eq!("\n       Name\n       └─ `opt-in rate`\n    ", tree(&[], "opt-in rate"));
}
