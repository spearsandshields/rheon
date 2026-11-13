use crate::composite::contains_composite;
use crate::context::ParsingContext;
use rheon_feel::context::FeelContext;
use rheon_feel::values::Value;
use rheon_feel::{FeelType, Name};

#[test]
fn test_display() {
  let mut pctx_a = ParsingContext::default();
  assert_eq!("{}", format!("{pctx_a}"));
  pctx_a.set_name("a".into());
  assert_eq!("{a: <v>}", format!("{pctx_a}"));
  pctx_a.set_name("b".into());
  assert_eq!("{a: <v>, b: <v>}", format!("{pctx_a}"));
  pctx_a.set_name("c".into());
  assert_eq!("{a: <v>, b: <v>, c: <v>}", format!("{pctx_a}"));
  let mut pctx_b = ParsingContext::default();
  pctx_b.set_name("x".into());
  pctx_b.set_name("y".into());
  pctx_b.set_name("z".into());
  pctx_a.set_context("d".into(), pctx_b);
  assert_eq!("{a: <v>, b: <v>, c: <v>, d: {x: <v>, y: <v>, z: <v>}}", format!("{pctx_a}"));
}

#[test]
fn test_from_feel_context_0001() {
  let fctx = FeelContext::default();
  let pctx: ParsingContext = fctx.into();
  assert_eq!("{}", format!("{pctx}"));
}

#[test]
fn test_from_feel_context_0002() {
  let mut fctx = FeelContext::default();
  fctx.set_null("a".into());
  let pctx: ParsingContext = fctx.into();
  assert_eq!("{a: <v>}", format!("{pctx}"));
}

#[test]
fn test_from_feel_context_0003() {
  let mut fctx = FeelContext::default();
  fctx.set_null("a".into());
  fctx.set_null("b".into());
  let pctx: ParsingContext = fctx.into();
  assert_eq!("{a: <v>, b: <v>}", format!("{pctx}"));
}

#[test]
fn test_from_feel_context_0004() {
  let mut fctx_a = FeelContext::default();
  fctx_a.set_null("a".into());
  fctx_a.set_null("b".into());
  let mut fctx_b = FeelContext::default();
  fctx_b.set_null("x".into());
  fctx_b.set_entry(&"y".into(), Value::Context(fctx_a));
  let pctx: ParsingContext = fctx_b.into();
  assert_eq!("{x: <v>, y: {a: <v>, b: <v>}}", format!("{pctx}"));
}

#[test]
fn test_from_feel_context_0005() {
  let mut fctx_a = FeelContext::default();
  fctx_a.set_null("a".into());
  fctx_a.set_entry(&"b".into(), Value::FeelType(FeelType::Any));
  let mut fctx_b = FeelContext::default();
  fctx_b.set_null("x".into());
  fctx_b.set_entry(&"y".into(), Value::Context(fctx_a));
  let pctx: ParsingContext = fctx_b.into();
  assert_eq!("{x: <v>, y: {a: <v>, b: <v>}}", format!("{pctx}"));
}

#[test]
fn test_from_feel_context_0006() {
  let mut fctx_a = FeelContext::default();
  fctx_a.set_null("a".into());
  fctx_a.set_entry(
    &"b".into(),
    Value::FeelType(FeelType::context(&[(&"i".into(), &FeelType::Number), (&"j".into(), &FeelType::Boolean)])),
  );
  let mut fctx_b = FeelContext::default();
  fctx_b.set_null("x".into());
  fctx_b.set_entry(&"y".into(), Value::Context(fctx_a));
  let pctx: ParsingContext = fctx_b.into();
  assert_eq!("{x: <v>, y: {a: <v>, b: {i: <v>, j: <v>}}}", format!("{pctx}"));
}

#[test]
fn test_from_feel_context_0007() {
  let mut fctx_a = FeelContext::default();
  fctx_a.set_null("a".into());
  fctx_a.set_entry(
    &"b".into(),
    Value::FeelType(FeelType::context(&[
      (&"i".into(), &FeelType::Number),
      (&"j".into(), &FeelType::context(&[(&"m".into(), &FeelType::Date), (&"n".into(), &FeelType::Time)])),
    ])),
  );
  let mut fctx_b = FeelContext::default();
  fctx_b.set_null("x".into());
  fctx_b.set_entry(&"y".into(), Value::Context(fctx_a));
  let pctx: ParsingContext = fctx_b.into();
  assert_eq!("{x: <v>, y: {a: <v>, b: {i: <v>, j: {m: <v>, n: <v>}}}}", format!("{pctx}"));
}

#[test]
fn test_from_feel_context_0008() {
  let mut fctx_a = FeelContext::default();
  fctx_a.set_null("a".into());
  fctx_a.set_entry(&"b".into(), Value::FeelType(FeelType::list(&FeelType::Boolean)));
  let mut fctx_b = FeelContext::default();
  fctx_b.set_null("x".into());
  fctx_b.set_entry(&"y".into(), Value::Context(fctx_a));
  let pctx: ParsingContext = fctx_b.into();
  assert_eq!("{x: <v>, y: {a: <v>, b: <v>}}", format!("{pctx}"));
}

#[test]
fn test_from_feel_context_0009() {
  let mut fctx_a = FeelContext::default();
  fctx_a.set_null("a".into());
  fctx_a.set_entry(
    &"b".into(),
    Value::FeelType(FeelType::list(&FeelType::context(&[
      (&"c".into(), &FeelType::DaysAndTimeDuration),
      (&"d".into(), &FeelType::YearsAndMonthsDuration),
    ]))),
  );
  let mut fctx_b = FeelContext::default();
  fctx_b.set_null("x".into());
  fctx_b.set_entry(&"y".into(), Value::Context(fctx_a));
  let pctx: ParsingContext = fctx_b.into();
  assert_eq!("{x: <v>, y: {a: <v>, b: {c: <v>, d: <v>}}}", format!("{pctx}"));
}

/// Asserts that every key of `keys` is a flattened key of `ctx` (a key at any depth, or a
/// composite key `k1 . k2 . … . kn`), and that no text of `non_keys` is.
fn assert_flattened_keys(ctx: &ParsingContext, keys: &[&str], non_keys: &[&str]) {
  for key in keys {
    if key.contains(" . ") {
      assert!(contains_composite(ctx, key), "{key:?} in {ctx}");
    } else {
      assert!(ctx.contains_key_deep(key), "{key:?} in {ctx}");
    }
  }
  for text in non_keys {
    assert!(!ctx.contains_key_deep(text) && !contains_composite(ctx, text), "{text:?} in {ctx}");
  }
}

#[test]
fn test_context_flatten_keys_one_level() {
  let name_a = Name::from("a");
  let name_b = Name::from("b");
  let mut ctx: ParsingContext = Default::default();
  ctx.set_name(name_a);
  ctx.set_name(name_b);
  assert_eq!(r#"{a: <v>, b: <v>}"#, ctx.to_string());
  assert_flattened_keys(&ctx, &["a", "b"], &["a . a", "c . d . e", "", "c"]);
}

#[test]
fn test_flatten_two_levels() {
  let name_a = Name::from("a");
  let name_b = Name::from("b");
  let name_c = Name::from("c");
  let mut ctx_b: ParsingContext = Default::default();
  ctx_b.set_name(name_a.clone());
  ctx_b.set_name(name_b.clone());
  ctx_b.set_name(name_c.clone());
  let mut ctx_a: ParsingContext = Default::default();
  ctx_a.set_name(name_a);
  ctx_a.set_name(name_b);
  ctx_a.set_context(name_c, ctx_b);
  assert_flattened_keys(&ctx_a, &["a", "b", "c", "c . a", "c . b", "c . c"], &["a . a", "c . d . e", "", "c . c . c", "d"]);
}

#[test]
fn test_flatten_three_levels() {
  let name_a = Name::from("a");
  let name_b = Name::from("b");
  let name_c = Name::from("c");
  let name_d = Name::from("d");
  let mut ctx_c: ParsingContext = Default::default();
  ctx_c.set_name(name_a.clone());
  ctx_c.set_name(name_b.clone());
  ctx_c.set_name(name_c.clone());
  ctx_c.set_name(name_d.clone());
  let mut ctx_b: ParsingContext = Default::default();
  ctx_b.set_name(name_a.clone());
  ctx_b.set_name(name_b.clone());
  ctx_b.set_name(name_c.clone());
  ctx_b.set_context(name_d, ctx_c);
  let mut ctx_a: ParsingContext = Default::default();
  ctx_a.set_name(name_a);
  ctx_a.set_name(name_b);
  ctx_a.set_context(name_c, ctx_b);
  assert_flattened_keys(
    &ctx_a,
    &[
      "a",
      "b",
      "c",
      "d",
      "c . a",
      "c . b",
      "c . c",
      "c . d",
      "c . d . a",
      "c . d . b",
      "c . d . c",
      "c . d . d",
      "d . a",
      "d . b",
      "d . c",
      "d . d",
    ],
    &["a . a", "c . d . e", "", "c . a . a", "d . d . d", "e"],
  );
}

#[test]
fn test_flatten_list_values() {
  let name_a = Name::from("a");
  let name_b = Name::from("b");
  let name_c = Name::from("c");
  let name_d = Name::from("d");
  let mut ctx_c: ParsingContext = Default::default();
  ctx_c.set_name(name_a.clone());
  ctx_c.set_name(name_b.clone());
  ctx_c.set_name(name_c.clone());
  ctx_c.set_name(name_d.clone());
  let mut ctx_b: ParsingContext = Default::default();
  ctx_b.set_name(name_a.clone());
  ctx_b.set_name(name_b);
  ctx_b.set_name(name_c);
  ctx_b.set_context(name_d, ctx_c);
  let mut ctx_a: ParsingContext = Default::default();
  ctx_a.set_context(name_a, ctx_b);
  assert_flattened_keys(
    &ctx_a,
    &[
      "a",
      "b",
      "c",
      "d",
      "a . a",
      "a . b",
      "a . c",
      "a . d",
      "d . a",
      "d . b",
      "d . c",
      "d . d",
      "a . d . a",
      "a . d . b",
      "a . d . c",
      "a . d . d",
    ],
    &["b . a", "c . d . e", "", "a . a . a", "e"],
  );
}

#[test]
fn test_flatten_names_with_additional_characters() {
  let name_a = Name::new(&["lorem", "ipsum", "dolor", "sit", "amet"]);
  let name_b = Name::new(&["b"]);
  let name_c = Name::new(&["now", "let", "'", "s", "go", "to", "the", "next", "paragraph"]);
  let name_d = Name::new(&["Curabitur", "rhoncus", "+", "sodales", "odio", "in", "fringilla"]);
  let mut ctx_b: ParsingContext = Default::default();
  ctx_b.set_name(name_d);
  let mut ctx_a: ParsingContext = Default::default();
  ctx_a.set_name(name_a);
  ctx_a.set_name(name_b);
  ctx_a.set_context(name_c, ctx_b);
  assert_flattened_keys(
    &ctx_a,
    &[
      "b",
      "lorem ipsum dolor sit amet",
      "now let's go to the next paragraph",
      "now let's go to the next paragraph . Curabitur rhoncus+sodales odio in fringilla",
      "Curabitur rhoncus+sodales odio in fringilla",
    ],
    &["a . a", "c . d . e", "", "b . b", "lorem ipsum"],
  );
}
