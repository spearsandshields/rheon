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
  // a name with a symbol is one name when it is in scope, and a division when it is not
  let scope = scope!();
  scope.set_entry_name("profit/loss".into());
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
  accept(
    &scope!(),
    StartExpression,
    "profit/loss",
    r#"
       Div
       ├─ Name
       │  └─ `profit`
       └─ Name
          └─ `loss`
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
/// (`return`, `then`, `else`, `satisfies`, `instance`, `in`, `and`, `or`, `between`) and before
/// a symbol that is an operator (`.`, `/`, `-`, `+`, `*`), so the expression has the tree it
/// has when the name is in scope.
#[test]
fn unknown_name_ends_before_a_keyword() {
  for (input, names) in [
    ("for s in x return 1", &["x"][..]),
    ("for s in x y return s", &["x y"]),
    ("for s in x.y return s.z", &["x"]),
    ("x - 1 and y", &["x", "y"]),
    ("x + y", &["x", "y"]),
    ("a b * c d / e f - g h + i j", &["a b", "c d", "e f", "g h", "i j"]),
    ("x.y.z", &["x"]),
    ("x.y + z", &["x", "z"]),
    ("[x..y]", &["x", "y"]),
    ("x-1", &["x"]),
    ("x ** y", &["x", "y"]),
    ("-x - -y", &["x", "y"]),
    ("x's + y", &["x's", "y"]),
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
/// follows an operand, a keyword without white space after it, and a longer word; and it keeps
/// an apostrophe.
#[test]
fn unknown_name_keeps_other_words() {
  for input in [
    "price list",
    "x for y",
    "x if y",
    "x of y",
    "x true",
    "x null",
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
  // an apostrophe is no operator, so it stays in the name
  assert_eq!("\n       Name\n       └─ `Grace's age`\n    ", tree(&[], "Grace's age"));
  // a symbol that is an operator ends the name: what follows it is read by itself
  assert_eq!(tree(&["opt", "rate"], "opt-in rate"), tree(&[], "opt-in rate"));
  // `and` without white space after it is a word of the name
  assert_eq!(
    "\n       Sub\n       ├─ Name\n       │  └─ `x and`\n       └─ Name\n          └─ `y`\n    ",
    tree(&[], "x and-y")
  );
}

/// A name that is being declared is the whole text, with the keywords and symbols it has; a
/// text that is no name is refused.
#[test]
fn declared_name_is_the_whole_text() {
  let scope = rheon_feel::FeelScope::default();
  for (text, name) in [
    ("Approved/Declined", "Approved/Declined"),
    ("Profit and loss", "Profit and loss"),
    ("Is in range", "Is in range"),
    ("opt-in rate", "opt-in rate"),
    ("a + b", "a+b"),
    ("x.y", "x.y"),
    ("tax return", "tax return"),
    ("Grace's age", "Grace's age"),
    ("  spaced   name ", "spaced name"),
    ("x", "x"),
  ] {
    assert_eq!(name, crate::parse_name(&scope, text, false).unwrap().as_str(), "{text}");
    assert_eq!(name, crate::parse_longest_name(text).unwrap().as_str(), "{text}");
  }
  for text in ["1", "a(b)", "a, b", "[a]", "\"a\"", "", "a = b"] {
    assert!(crate::parse_name(&scope, text, false).is_err(), "{text}");
  }
}

/// DMN 1.6 clause 10.3.1.4: a name start is never a literal terminal symbol. A word that is a
/// keyword is the keyword whatever character follows it; a longer word is a name.
#[test]
fn a_word_that_is_a_keyword_is_the_keyword_whatever_follows() {
  let scope = scope!();
  for name in ["x", "in-progress", "and/or", "if-flag", "iffy", "in1", "order", "format"] {
    scope.set_entry_name(name.into());
  }
  let parse = |input: &str| Parser::new(&scope, StartExpression, input, false).parse();
  for input in [
    "x in[1]",
    "x in(1, 2)",
    "x in\"a\"",
    "if(x)then(1)else(2)",
    "if x then\"a\"else\"b\"",
    "x and(x)",
    "x or[x]",
    "x between(1)and(2)",
    "for x in[1] return(x)",
    "some x in[1]satisfies(x)",
    "every x in[1]satisfies(x)",
    "x instance/**/of number",
    // a longer word is a name
    "iffy",
    "in1",
    "order",
    "format + 1",
  ] {
    assert!(parse(input).is_ok(), "{input}");
  }
  // a name does not start with a keyword, also when it is in scope, a key or a parameter
  for input in [
    "in-progress",
    "and/or",
    "if-flag",
    "{in: 1}",
    "{if: 1}",
    "{true: 1}",
    "{null: 1}",
    "x.in",
    "x.and.y",
    "x.or + 1",
    "function(in) 1",
    "f(in: 1)",
  ] {
    assert!(parse(input).is_err(), "{input}");
  }
  // a key that is a string is not a name
  assert!(parse("{\"in\": 1}").is_ok());
}

/// The word `item` is a name like any other: a name that starts with it is read whole.
#[test]
fn a_name_that_starts_with_item_is_read_whole() {
  let scope = scope!();
  for name in ["item count", "item-id"] {
    scope.set_entry_name(name.into());
  }
  let tree = |input: &str| match Parser::new(&scope, StartExpression, input, false).parse() {
    Ok(node) => node.trace(),
    Err(reason) => panic!("{input}: {reason}"),
  };
  assert!(tree("item count + 1").contains("`item count`"));
  assert!(tree("item-id").contains("`item-id`"));
  assert!(tree("[1, 2][item count > 1]").contains("`item count`"));
  assert!(tree("{item x: 1}").contains("`item x`"));
  assert!(tree("for item count in [1, 2] return item count").contains("`item count`"));
  assert!(tree("function(item x) item x + 1").contains("`item x`"));
  // alone it ends before an operator, a keyword or a bracket as any name that is not in scope
  for input in [
    "[1, 2][item > 1]",
    "[1, 2][item in [1]]",
    "[1, 2][item-1 = 0]",
    "[[1]][item[1] = 1]",
    "[1, 2][item.a = 1]",
    "[1, 2][item instance of number]",
  ] {
    assert!(tree(input).contains("`item`"), "{input}");
  }
}
