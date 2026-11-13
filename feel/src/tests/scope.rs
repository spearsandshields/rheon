use crate::context::FeelContext;
use crate::values::Value;
use crate::{FeelNumber, FeelScope, Name, scope, value_number};
use dsntk_common::Jsonify;

#[test]
fn test_scope_default() {
  assert_eq!("[{}]", FeelScope::default().to_string());
  assert_eq!("[{}]", scope!().to_string());
}

#[test]
fn test_scope_new() {
  assert_eq!("[]", FeelScope::new().to_string());
}

#[test]
fn test_scope_to_string() {
  let scope = FeelScope::default();
  let name_a = Name::from("a");
  let name_b = Name::from("b");
  scope.set_value(&name_a, value_number!(495, 1));
  scope.set_value(&name_b, value_number!(50));
  assert_eq!("[{a: 49.5, b: 50}]", scope.to_string());
}

#[test]
fn test_scope_jsonify() {
  let scope = FeelScope::default();
  let name_a = Name::from("a");
  let name_b = Name::from("b");
  scope.set_value(&name_a, value_number!(495, 1));
  scope.set_value(&name_b, value_number!(50));
  assert_eq!("[{a: 49.5, b: 50}]", scope.jsonify());
}

#[test]
fn test_scope_single_empty_context() {
  let scope = FeelScope::new();
  let ctx = FeelContext::default();
  scope.push(ctx);
  assert_eq!("[{}]", scope.to_string());
  let scope: FeelScope = FeelContext::default().into();
  assert_eq!("[{}]", scope.to_string());
}

#[test]
fn test_scope_multiple_empty_contexts() {
  let scope = FeelScope::new();
  scope.push(FeelContext::default());
  scope.push(FeelContext::default());
  scope.push(FeelContext::default());
  assert_eq!("[{}, {}, {}]", scope.to_string());
  let scope = FeelScope::default();
  scope.push(FeelContext::default());
  scope.push(FeelContext::default());
  scope.push(FeelContext::default());
  assert_eq!("[{}, {}, {}, {}]", scope.to_string());
}

#[test]
fn test_scope_single_context() {
  let scope = FeelScope::default();
  assert_eq!("[{}]", scope.to_string());
  let name_a = Name::from("a");
  let name_b = Name::from("b");
  scope.set_value(&name_a, value_number!(495, 1));
  assert_eq!("[{a: 49.5}]", scope.to_string());
  scope.set_value(&name_b, value_number!(50));
  assert_eq!("[{a: 49.5, b: 50}]", scope.to_string());
  scope.pop();
  assert_eq!("[]", scope.to_string());
}

#[test]
fn test_scope_no_context() {
  let scope = FeelScope::new();
  assert_eq!("[]", scope.to_string());
  let name_a = Name::from("a");
  let name_b = Name::from("b");
  scope.set_value(&name_a, value_number!(125, 2));
  assert_eq!("[]", scope.to_string());
  scope.set_value(&name_b, value_number!(175, 2));
  assert_eq!("[]", scope.to_string());
  scope.pop();
  assert_eq!("[]", scope.to_string());
}

#[test]
fn test_scope_push() {
  let name_a = Name::from("a");
  let name_b = Name::from("b");
  let name_c = Name::from("c");
  let mut ctx_a: FeelContext = Default::default();
  ctx_a.set_entry(&name_a, value_number!(1));
  let mut ctx_b: FeelContext = Default::default();
  ctx_b.set_entry(&name_b, value_number!(2));
  let mut ctx_c: FeelContext = Default::default();
  ctx_c.set_entry(&name_c, value_number!(3));
  let scope = FeelScope::new();
  scope.push(ctx_a);
  scope.push(ctx_b);
  scope.push(ctx_c);
  assert_eq!("[{a: 1}, {b: 2}, {c: 3}]", scope.to_string());
}

#[test]
fn test_scope_pop() {
  let name_a = Name::from("a");
  let name_b = Name::from("b");
  let name_c = Name::from("c");
  let mut ctx_a: FeelContext = Default::default();
  ctx_a.set_entry(&name_a, value_number!(1));
  let mut ctx_b: FeelContext = Default::default();
  ctx_b.set_entry(&name_b, value_number!(2));
  let mut ctx_c: FeelContext = Default::default();
  ctx_c.set_entry(&name_c, value_number!(3));
  let scope = FeelScope::new();
  scope.push(ctx_a);
  scope.push(ctx_b);
  scope.push(ctx_c);
  scope.pop();
  assert_eq!("[{a: 1}, {b: 2}]", scope.to_string());
}

// #[test]
// fn test_scope_peek() {
//   let name_a = Name::from("a");
//   let name_b = Name::from("b");
//   let name_c = Name::from("c");
//   let mut ctx_a: FeelContext = Default::default();
//   ctx_a.set_entry(&name_a, value_number!(1));
//   let mut ctx_b: FeelContext = Default::default();
//   ctx_b.set_entry(&name_b, value_number!(2));
//   let mut ctx_c: FeelContext = Default::default();
//   ctx_c.set_entry(&name_c, value_number!(3));
//   let scope = FeelScope::new();
//   scope.push(ctx_a);
//   scope.push(ctx_b);
//   scope.push(ctx_c);
//   let ctx = scope.peek();
//   assert_eq!("{c: 3}", ctx.to_string());
//   assert_eq!("[{a: 1}, {b: 2}, {c: 3}]", scope.to_string());
// }

/// A scope with the contexts `{a: 1}`, `{b: 2}` and `{c: 3}`, bottom first.
fn scope_a_b_c() -> FeelScope {
  let scope = FeelScope::new();
  for (name, value) in [("a", 1), ("b", 2), ("c", 3)] {
    let mut ctx = FeelContext::default();
    ctx.set_entry(&Name::from(name), value_number!(value));
    scope.push(ctx);
  }
  scope
}

#[test]
fn test_scope_depth() {
  assert_eq!(0, FeelScope::new().depth());
  assert_eq!(1, FeelScope::default().depth());
  let scope = scope_a_b_c();
  assert_eq!(3, scope.depth());
  scope.pop();
  assert_eq!(2, scope.depth());
}

#[test]
fn test_scope_truncate() {
  let scope = scope_a_b_c();
  // a depth above the number of contexts leaves the stack unchanged
  scope.truncate(5);
  assert_eq!("[{a: 1}, {b: 2}, {c: 3}]", scope.to_string());
  scope.truncate(3);
  assert_eq!("[{a: 1}, {b: 2}, {c: 3}]", scope.to_string());
  // a depth below it removes the contexts above that depth
  scope.truncate(1);
  assert_eq!("[{a: 1}]", scope.to_string());
  scope.truncate(0);
  assert_eq!("[]", scope.to_string());
}

#[test]
fn test_scope_pop_frame() {
  // the context above `depth` is returned, the contexts above it are dropped
  let scope = scope_a_b_c();
  assert_eq!("{b: 2}", scope.pop_frame(1).unwrap().to_string());
  assert_eq!(1, scope.depth());
  assert_eq!("[{a: 1}]", scope.to_string());
  // the top context
  let scope = scope_a_b_c();
  assert_eq!("{c: 3}", scope.pop_frame(2).unwrap().to_string());
  assert_eq!("[{a: 1}, {b: 2}]", scope.to_string());
  // no context above `depth`: None, and the stack is unchanged
  let scope = scope_a_b_c();
  assert!(scope.pop_frame(3).is_none());
  assert!(scope.pop_frame(4).is_none());
  assert_eq!("[{a: 1}, {b: 2}, {c: 3}]", scope.to_string());
  assert!(FeelScope::new().pop_frame(0).is_none());
}

#[test]
fn test_scope_contexts_and_frames() {
  let scope = scope_a_b_c();
  let contexts = scope.contexts();
  assert_eq!(3, contexts.len());
  assert_eq!(contexts.as_slice(), &*scope.frames());
  assert_eq!("{a: 1}", scope.frames()[0].to_string());
  assert_eq!("{c: 3}", scope.frames()[2].to_string());
  assert!(FeelScope::new().contexts().is_empty());
  assert!(FeelScope::new().frames().is_empty());
}

#[test]
fn test_get_entry() {
  let name_a = Name::from("a");
  let name_b = Name::from("b");
  let name_c = Name::from("c");
  let name_d = Name::from("d");
  let mut ctx_a: FeelContext = Default::default();
  ctx_a.set_entry(&name_a, value_number!(1));
  let mut ctx_b: FeelContext = Default::default();
  ctx_b.set_entry(&name_b, value_number!(2));
  let mut ctx_c: FeelContext = Default::default();
  ctx_c.set_entry(&name_c, value_number!(3));
  let scope = FeelScope::new();
  scope.push(ctx_a);
  scope.push(ctx_b);
  scope.push(ctx_c);
  assert_eq!(value_number!(1), scope.get_value(&name_a).unwrap());
  assert_eq!(None, scope.get_value(&name_d));
}

#[test]
fn test_insert_null() {
  let name_a = Name::from("a");
  let name_b = Name::from("b");
  let name_c = Name::from("c");
  let name_d = Name::from("d");
  let mut ctx_a: FeelContext = Default::default();
  ctx_a.set_entry(&name_a, value_number!(1));
  let mut ctx_b: FeelContext = Default::default();
  ctx_b.set_entry(&name_b, value_number!(2));
  let mut ctx_c: FeelContext = Default::default();
  ctx_c.set_entry(&name_c, value_number!(3));
  let scope = FeelScope::new();
  scope.push(ctx_a);
  scope.push(ctx_b);
  scope.push(ctx_c);
  scope.set_name(name_b);
  scope.set_name(name_c);
  scope.set_name(name_d);
  assert_eq!("[{a: 1}, {b: 2}, {b: null, c: null, d: null}]", scope.to_string());
}
