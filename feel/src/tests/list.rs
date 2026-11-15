use crate::value_number;
use crate::values::{FeelList, Value};

#[test]
fn test_list_new_and_push() {
  let mut list = FeelList::new();
  assert!(list.is_empty());
  list.push(value_number!(1));
  list.push(value_number!(2));
  assert_eq!(2, list.len());
  assert_eq!(&[value_number!(1), value_number!(2)], list.as_slice());
  assert_eq!(Some(&value_number!(2)), list.last());
  assert_eq!(vec![value_number!(1), value_number!(2)], list.into_vec());
}

#[test]
fn test_list_from_and_collect() {
  let from_vec = FeelList::from(vec![value_number!(1), value_number!(2)]);
  let collected = (1..=2).map(|n| value_number!(n)).collect::<FeelList>();
  assert_eq!(from_vec, collected);
  assert_ne!(from_vec, FeelList::new());
  // an empty list is equal to every other empty list
  assert_eq!(FeelList::new(), FeelList::from(Vec::with_capacity(10)));
}

#[test]
fn test_list_iterate() {
  let list = FeelList::from(vec![value_number!(1), value_number!(2)]);
  let by_reference = (&list).into_iter().cloned().collect::<Vec<Value>>();
  assert_eq!(by_reference, list.into_iter().collect::<Vec<Value>>());
}

#[test]
fn test_list_debug_and_value() {
  let list = FeelList::from(vec![value_number!(1)]);
  assert_eq!("[Number(+1E+0)]", format!("{list:?}"));
  assert_eq!("List([Number(+1E+0)])", format!("{:?}", Value::from(list)));
}
