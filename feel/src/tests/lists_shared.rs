//! Lists share their items between copies, and a change to one copy never shows through another.

use crate::context::FeelContext;
use crate::values::{FeelList, Value};
use crate::{Name, value_number};

fn numbers(range: std::ops::RangeInclusive<i64>) -> FeelList {
  range.map(|n| value_number!(n)).collect()
}

#[test]
fn a_copy_shares_the_items() {
  let list = numbers(1..=3);
  let copy = list.clone();
  assert_eq!(list.as_slice().as_ptr(), copy.as_slice().as_ptr());
  assert_eq!(list, copy);
}

#[test]
fn a_push_to_a_shared_list_leaves_the_other_copy_unchanged() {
  let list = numbers(1..=3);
  let mut copy = list.clone();
  copy.push(value_number!(4));
  assert_eq!(numbers(1..=3), list);
  assert_eq!(numbers(1..=4), copy);
  assert_ne!(list.as_slice().as_ptr(), copy.as_slice().as_ptr());
  // the copy holds its items alone now: further pushes change it in place
  copy.push(value_number!(5));
  copy.push(value_number!(6));
  assert_eq!(numbers(1..=6), copy);
  assert_eq!(numbers(1..=3), list);
}

#[test]
fn the_items_of_a_list_that_holds_them_alone_are_moved_out() {
  let list = numbers(1..=3);
  let items = list.as_slice().as_ptr();
  let vec = list.into_vec();
  assert_eq!(items, vec.as_ptr());
  assert_eq!(vec![value_number!(1), value_number!(2), value_number!(3)], vec);
}

#[test]
fn the_items_of_a_shared_list_are_copied_out() {
  let list = numbers(1..=3);
  let mut vec = list.clone().into_vec();
  assert_ne!(list.as_slice().as_ptr(), vec.as_ptr());
  vec.push(value_number!(4));
  assert_eq!(numbers(1..=3), list);
  let mut taken = list.clone().into_iter();
  assert_eq!(3, taken.len());
  assert_eq!(Some(value_number!(1)), taken.next());
  assert_eq!(2, taken.len());
  assert_eq!(vec![value_number!(2), value_number!(3)], taken.collect::<Vec<Value>>());
  assert_eq!(numbers(1..=3), list);
  assert_eq!(numbers(1..=3).into_iter().collect::<Vec<Value>>(), list.into_vec());
}

#[test]
fn an_item_that_is_a_list_is_shared_by_the_copies() {
  let inner = Value::List(numbers(1..=2));
  let list = FeelList::from(vec![inner.clone(), inner.clone()]);
  let Value::List(inner) = inner else { unreachable!() };
  for item in &list {
    let Value::List(item) = item else { panic!("expected a list") };
    assert_eq!(inner.as_slice().as_ptr(), item.as_slice().as_ptr());
  }
}

#[test]
fn the_empty_list_holds_no_items() {
  let empty = FeelList::from(Vec::with_capacity(10));
  assert_eq!(FeelList::new(), empty);
  assert!(empty.is_empty());
  assert_eq!("[]", format!("{empty:?}"));
  assert_eq!(0, empty.clone().into_iter().len());
  assert!(empty.into_vec().is_empty());
  let mut list = FeelList::new();
  list.push(value_number!(1));
  assert_eq!(numbers(1..=1), list);
}

/// A list of contexts of strings, each copied into a thread of its own and changed there: every
/// thread sees its own change only, and the list it was copied from stays as it was.
#[test]
fn copies_changed_in_threads_leave_the_original_unchanged() {
  let item = |n: usize| {
    let mut ctx = FeelContext::default();
    ctx.set_entry(&Name::from("text"), Value::String(format!("item {n}").into()));
    Value::from(ctx)
  };
  let original: FeelList = (0..8).map(item).collect();
  let snapshot = format!("{original:?}");
  let handles = (0..8)
    .map(|thread| {
      let mut copy = original.clone();
      std::thread::spawn(move || {
        if thread % 2 == 0 {
          copy.push(item(100 + thread));
          copy
        } else {
          let mut items = copy.into_vec();
          items[thread] = item(100 + thread);
          FeelList::from(items)
        }
      })
    })
    .collect::<Vec<_>>();
  for (thread, handle) in handles.into_iter().enumerate() {
    let changed = handle.join().expect("the thread ends");
    let mut expected = original.clone().into_vec();
    if thread % 2 == 0 {
      expected.push(item(100 + thread));
    } else {
      expected[thread] = item(100 + thread);
    }
    assert_eq!(FeelList::from(expected), changed);
  }
  assert_eq!(snapshot, format!("{original:?}"));
}
