//! Contexts share their entries between copies of a context value, and a change to one copy never
//! shows through another: a change copies only the contexts on its path, and the entries off the
//! path stay shared.

use crate::context::FeelContext;
use crate::values::Value;
use crate::{Name, QualifiedName, value_number};

fn name(text: &str) -> Name {
  Name::from(text)
}

/// A context with one entry `t` holding `tag`: a sibling off the written paths.
fn sibling(tag: &str) -> Value {
  let mut ctx = FeelContext::default();
  ctx.set_entry(&name("t"), Value::String(tag.into()));
  ctx.into()
}

/// `{s1: .., v: 1, a: {s2: .., v: 2, b: {s3: .., v: 3}}}`: a context 3 levels deep, with a sibling
/// context `s<k>` and a number `v` at each level `k`.
fn original() -> Value {
  let mut level3 = FeelContext::default();
  level3.set_entry(&name("s3"), sibling("s3"));
  level3.set_entry(&name("v"), value_number!(3));
  let mut level2 = FeelContext::default();
  level2.set_entry(&name("s2"), sibling("s2"));
  level2.set_entry(&name("v"), value_number!(2));
  level2.set_entry(&name("b"), level3.into());
  let mut level1 = FeelContext::default();
  level1.set_entry(&name("s1"), sibling("s1"));
  level1.set_entry(&name("v"), value_number!(1));
  level1.set_entry(&name("a"), level2.into());
  level1.into()
}

/// The names of the contexts from the top one to the one at `nesting` (1 is the top one).
const PATH: [&str; 2] = ["a", "b"];

/// The siblings, by their paths from the top context.
const SIBLINGS: [&[&str]; 3] = [&["s1"], &["a", "s2"], &["a", "b", "s3"]];

/// The value at `path` in a context value.
fn entry<'a>(value: &'a Value, path: &[&str]) -> &'a Value {
  path.iter().fold(value, |value, key| {
    let Value::Context(ctx) = value else { panic!("{key}: not in a context") };
    ctx.get_entry(&name(key)).unwrap_or_else(|| panic!("{key}: no such entry"))
  })
}

/// The address of the context a context value holds.
fn address(value: &Value) -> usize {
  let Value::Context(ctx) = value else { panic!("not a context") };
  std::ptr::from_ref::<FeelContext>(ctx).addr()
}

/// `value` with `change` applied to its context at `nesting` (1 is the top one), each context on
/// the way taken out of its value and put back, as a caller changes a context value.
fn changed_at(value: Value, nesting: usize, change: &dyn Fn(&mut FeelContext)) -> Value {
  changed_at_path(value, &PATH[..nesting - 1], change)
}

fn changed_at_path(value: Value, path: &[&str], change: &dyn Fn(&mut FeelContext)) -> Value {
  let mut ctx = FeelContext::try_from(value).expect("a context");
  match path.split_first() {
    None => change(&mut ctx),
    Some((key, rest)) => {
      let key = name(key);
      let inner = ctx.remove_entry(&key).expect("the path exists");
      ctx.set_entry(&key, changed_at_path(inner, rest, change));
    }
  }
  ctx.into()
}

/// A change of the context it is applied to.
type Change = Box<dyn Fn(&mut FeelContext)>;

/// Every method that changes a context, each a change at the context it is applied to.
fn changes() -> Vec<(&'static str, Change)> {
  let other = |key: &str, value: Value| {
    let mut ctx = FeelContext::default();
    ctx.set_entry(&name(key), value);
    ctx
  };
  vec![
    ("set_entry", Box::new(|ctx: &mut FeelContext| ctx.set_entry(&name("x"), value_number!(9)))),
    ("insert", Box::new(|ctx: &mut FeelContext| ctx.insert(name("x"), value_number!(9)))),
    (
      "assign_entry",
      Box::new(|ctx: &mut FeelContext| {
        ctx.assign_entry(&name("v"), value_number!(9));
      }),
    ),
    (
      "remove_entry",
      Box::new(|ctx: &mut FeelContext| {
        ctx.remove_entry(&name("v"));
      }),
    ),
    ("set_null", Box::new(|ctx: &mut FeelContext| ctx.set_null(name("v")))),
    ("zip", Box::new(move |ctx: &mut FeelContext| ctx.zip(&other("x", value_number!(9))))),
    ("overwrite", Box::new(move |ctx: &mut FeelContext| ctx.overwrite(&other("v", value_number!(9))))),
    ("move_entry", Box::new(|ctx: &mut FeelContext| ctx.move_entry(name("v"), name("p")))),
    (
      "create_entry",
      Box::new(|ctx: &mut FeelContext| ctx.create_entry(&QualifiedName::new(&[&name("p"), &name("q")]), value_number!(9))),
    ),
    (
      "create_entries",
      Box::new(|ctx: &mut FeelContext| ctx.create_entries(&[name("p"), name("q")], value_number!(9))),
    ),
    (
      "apply_entries",
      Box::new(|ctx: &mut FeelContext| ctx.apply_entries(&[name("p"), name("q")], value_number!(9)).expect("not blocked")),
    ),
  ]
}

#[test]
fn a_copy_shares_the_entries() {
  let value = original();
  let copy = value.clone();
  assert_eq!(address(&value), address(&copy));
  assert_eq!(value, copy);
}

/// Each method applied to a copy at nesting 1, 2 and 3 leaves the original equal to a snapshot,
/// changes the copy, and leaves the siblings shared by the copy and the original.
#[test]
fn a_change_to_a_copy_leaves_the_original_unchanged() {
  let snapshot = original();
  let value = original();
  for (method, change) in changes() {
    for nesting in 1..=3 {
      let copy = changed_at(value.clone(), nesting, change.as_ref());
      assert_eq!(snapshot, value, "{method} at nesting {nesting} changed the original");
      assert_eq!(format!("{snapshot:?}"), format!("{value:?}"), "{method} at nesting {nesting}");
      assert_ne!(value, copy, "{method} at nesting {nesting} did not change the copy");
      for path in SIBLINGS {
        assert_eq!(
          address(entry(&value, path)),
          address(entry(&copy, path)),
          "{method} at nesting {nesting}: {} is not shared",
          path.join(".")
        );
      }
      // the contexts on the path of the change are the copy's own
      for depth in 0..nesting {
        assert_ne!(
          address(entry(&value, &PATH[..depth])),
          address(entry(&copy, &PATH[..depth])),
          "{method} at nesting {nesting}"
        );
      }
    }
  }
}

/// A change made through a path copies the contexts on that path only.
#[test]
fn a_change_through_a_path_copies_the_contexts_on_the_path() {
  let snapshot = original();
  let value = original();
  for length in 1..=3 {
    let names: Vec<Name> = PATH[..length - 1].iter().map(|key| name(key)).chain([name("x")]).collect();
    for method in ["create_entries", "apply_entries"] {
      let mut ctx = FeelContext::try_from(value.clone()).expect("a context");
      match method {
        "create_entries" => ctx.create_entries(&names, value_number!(9)),
        _ => ctx.apply_entries(&names, value_number!(9)).expect("not blocked"),
      }
      let copy = Value::from(ctx);
      assert_eq!(snapshot, value, "{method} of {length} names changed the original");
      let path: Vec<&str> = names.iter().map(Name::as_str).collect();
      assert_eq!(&value_number!(9), entry(&copy, &path), "{method} of {length} names");
      for sibling in SIBLINGS {
        assert_eq!(
          address(entry(&value, sibling)),
          address(entry(&copy, sibling)),
          "{method} of {length} names: {}",
          sibling.join(".")
        );
      }
      for depth in 0..length {
        assert_ne!(address(entry(&value, &PATH[..depth])), address(entry(&copy, &PATH[..depth])), "{method} of {length} names");
      }
      // the contexts below the path are shared
      if length < 3 {
        assert_eq!(
          address(entry(&value, &PATH[..length])),
          address(entry(&copy, &PATH[..length])),
          "{method} of {length} names"
        );
      }
    }
  }
}

/// A context value copied into 8 threads and changed in each at nesting 1, 2 or 3: every thread
/// sees its own change only, and the value it was copied from stays as it was.
#[test]
fn copies_changed_in_threads_leave_the_original_unchanged() {
  let value = original();
  let handles = (0..8)
    .map(|thread| {
      let copy = value.clone();
      std::thread::spawn(move || {
        let change = |ctx: &mut FeelContext| ctx.set_entry(&name("x"), value_number!(100 + thread as i64));
        changed_at(copy, thread % 3 + 1, &change)
      })
    })
    .collect::<Vec<_>>();
  for (thread, handle) in handles.into_iter().enumerate() {
    let copy = handle.join().expect("the thread ends");
    let path: Vec<&str> = PATH[..thread % 3].iter().copied().chain(["x"]).collect();
    assert_eq!(&value_number!(100 + thread as i64), entry(&copy, &path));
  }
  assert_eq!(original(), value);
}
