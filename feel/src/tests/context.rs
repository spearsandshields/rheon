use crate::context::FeelContext;
use crate::names::Name;
use crate::qualified_names::QualifiedName;
use crate::values::{VALUE_FALSE, VALUE_TRUE, Value};
use crate::{ToFeelString, value_number};
use dsntk_common::Jsonify;
use std::slice;

#[test]
fn test_context_default() {
  let ctx: FeelContext = Default::default();
  assert_eq!("{}", ctx.to_string());
  assert_eq!("FeelContext({})", format!("{ctx:?}"));
  assert_eq!(None, ctx.get_first());
  assert_eq!(0, ctx.len());
  assert!(ctx.is_empty());
}

#[test]
fn test_context_to_string() {
  let name_a = Name::from("a");
  let name_x_y = Name::new(&["x", "y"]);
  let name_k_plus_l_minus_m = Name::new(&["k", "+", "l", "-", "m"]);
  let mut ctx: FeelContext = Default::default();
  ctx.set_entry(&name_a, value_number!(10));
  assert_eq!(r#"{a: 10}"#, ctx.to_string());
  ctx.set_entry(&name_x_y, Value::Boolean(true));
  assert_eq!(r#"{a: 10, x y: true}"#, ctx.to_string());
  ctx.set_entry(&name_k_plus_l_minus_m, Value::String("KLM".into()));
  assert_eq!(r#"{a: 10, k+l-m: "KLM", x y: true}"#, ctx.to_string());
}

#[test]
fn test_context_to_feel_string() {
  let name_a = Name::from("a");
  let name_x_y = Name::new(&["x", "y"]);
  let name_k_plus_l_minus_m = Name::new(&["k", "+", "l", "-", "m"]);
  let mut ctx: FeelContext = Default::default();
  ctx.set_entry(&name_a, value_number!(10));
  assert_eq!(r#"{a: 10}"#, ctx.to_feel_string());
  ctx.set_entry(&name_x_y, Value::Boolean(true));
  assert_eq!(r#"{a: 10, x y: true}"#, ctx.to_feel_string());
  ctx.set_entry(&name_k_plus_l_minus_m, Value::String("KLM".into()));
  assert_eq!(r#"{a: 10, k+l-m: "KLM", x y: true}"#, ctx.to_feel_string());
  let mut ctx: FeelContext = Default::default();
  let name_left_bracket = Name::from("{");
  ctx.set_entry(&name_left_bracket, value_number!(1));
  assert_eq!(r#"{"{": 1}"#, ctx.to_feel_string());
  let name_right_bracket = Name::from("}");
  ctx.set_entry(&name_right_bracket, value_number!(2));
  assert_eq!(r#"{"{": 1, "}": 2}"#, ctx.to_feel_string());
  let name_colon = Name::from(":");
  ctx.set_entry(&name_colon, value_number!(3));
  assert_eq!(r#"{":": 3, "{": 1, "}": 2}"#, ctx.to_feel_string());
  let name_comma = Name::from(",");
  ctx.set_entry(&name_comma, value_number!(4));
  assert_eq!(r#"{",": 4, ":": 3, "{": 1, "}": 2}"#, ctx.to_feel_string());
  let name_double_quote = Name::from("\"");
  ctx.set_entry(&name_double_quote, value_number!(5));
  assert_eq!(r#"{"\"": 5, ",": 4, ":": 3, "{": 1, "}": 2}"#, ctx.to_feel_string());
}

#[test]
fn test_context_to_json() {
  let name_a = Name::from("a");
  let name_x_y = Name::new(&["x", "y"]);
  let name_k_plus_l_minus_m = Name::new(&["k", "+", "l", "-", "m"]);
  let mut ctx: FeelContext = Default::default();
  ctx.set_entry(&name_a, value_number!(10));
  assert_eq!(r#"{"a": 10}"#, ctx.jsonify());
  ctx.set_entry(&name_x_y, Value::Boolean(true));
  assert_eq!(r#"{"a": 10, "x y": true}"#, ctx.jsonify());
  ctx.set_entry(&name_k_plus_l_minus_m, Value::String("KLM".into()));
  assert_eq!(r#"{"a": 10, "k+l-m": "KLM", "x y": true}"#, ctx.jsonify());
}

#[test]
fn test_context_one_level() {
  let name_a = Name::from("a");
  let name_a_b = Name::new(&["a", "b"]);
  let name_a_b_c = Name::new(&["a", "b", "c"]);
  let qname_a = QualifiedName::new(&[&name_a]);
  let qname_a_b = QualifiedName::new(&[&name_a_b]);
  let qname_a_b_c = QualifiedName::new(&[&name_a_b_c]);
  let mut ctx_a: FeelContext = Default::default();
  ctx_a.set_entry(&name_a, value_number!(10));
  assert_eq!("{a: 10}", ctx_a.to_string());
  assert!(ctx_a.contains_entry(&name_a));
  assert!(ctx_a.contains_entries(&qname_a));
  assert!(!ctx_a.contains_entry(&name_a_b));
  assert!(!ctx_a.contains_entries(&qname_a_b));
  assert!(!ctx_a.contains_entry(&name_a_b_c));
  assert!(!ctx_a.contains_entries(&qname_a_b_c));
  assert_eq!("10", ctx_a.get_entry(&name_a).unwrap().to_string().as_str());
}

#[test]
fn test_context_two_levels() {
  let name_married = Name::from("married");
  let name_age = Name::from("age");
  let name_b = Name::from("b");
  let name_x_y = Name::new(&["x", "y"]);
  let qname_married = QualifiedName::new(&[&name_married]);
  let qname_age = QualifiedName::new(&[&name_age]);
  let qname_b = QualifiedName::new(&[&name_b]);
  let qname_b_married = QualifiedName::new(&[&name_b, &name_married]);
  let qname_b_married_age = QualifiedName::new(&[&name_b, &name_married, &name_age]);
  let mut ctx_b: FeelContext = Default::default();
  ctx_b.set_entry(&name_married, Value::Boolean(true));
  assert_eq!("{married: true}", ctx_b.to_string());
  assert!(ctx_b.contains_entry(&name_married));
  assert!(ctx_b.contains_entries(&qname_married));
  let mut ctx_a: FeelContext = Default::default();
  ctx_a.set_entry(&name_age, value_number!(49));
  ctx_a.set_entry(&name_x_y, Value::Boolean(true));
  ctx_a.set_entry(&name_b, ctx_b.into());
  assert_eq!("{age: 49, b: {married: true}, x y: true}", ctx_a.to_string());
  assert!(ctx_a.contains_entry(&name_age));
  assert!(ctx_a.contains_entry(&name_b));
  assert!(ctx_a.contains_entry(&name_x_y));
  assert!(ctx_a.contains_entries(&qname_age));
  assert!(ctx_a.contains_entries(&qname_b));
  assert!(ctx_a.contains_entries(&qname_b_married));
  assert!(!ctx_a.contains_entries(&qname_b_married_age));
  assert_eq!("49", ctx_a.get_entry(&name_age).unwrap().to_string().as_str());
  assert_eq!("{married: true}", ctx_a.get_entry(&name_b).unwrap().to_string().as_str());
}

#[test]
fn test_context_three_levels() {
  let name_car = Name::from("car");
  let name_married = Name::from("married");
  let name_age = Name::from("age");
  let name_b = Name::from("b");
  let name_c = Name::from("c");
  let mut ctx_c: FeelContext = Default::default();
  ctx_c.set_entry(&name_car, Value::String("opel".into()));
  assert_eq!(r#"{car: "opel"}"#, ctx_c.to_string());
  assert!(ctx_c.contains_entry(&name_car));
  let mut ctx_b: FeelContext = Default::default();
  ctx_b.set_entry(&name_married, Value::Boolean(true));
  ctx_b.set_entry(&name_c, ctx_c.into());
  assert_eq!(r#"{c: {car: "opel"}, married: true}"#, ctx_b.to_string());
  assert!(ctx_b.contains_entry(&name_married));
  assert!(ctx_b.contains_entry(&name_c));
  let mut ctx_a: FeelContext = Default::default();
  ctx_a.set_entry(&name_age, value_number!(49));
  ctx_a.set_entry(&name_b, ctx_b.into());
  assert_eq!(r#"{age: 49, b: {c: {car: "opel"}, married: true}}"#, ctx_a.to_string());
  assert!(ctx_a.contains_entry(&name_age));
  assert!(ctx_a.contains_entry(&name_b));
}

#[test]
fn test_context_search_entry() {
  let name_married = Name::from("married");
  let name_age = Name::from("age");
  let name_b = Name::from("b");
  let mut ctx_b: FeelContext = Default::default();
  ctx_b.set_entry(&name_married, Value::Boolean(true));
  let mut ctx_a: FeelContext = Default::default();
  ctx_a.set_entry(&name_age, value_number!(49));
  ctx_a.set_entry(&name_b, ctx_b.into());
  let qn_empty = QualifiedName::new(&[]);
  assert!(ctx_a.search_entry(&qn_empty).is_none());
  let qn_b = QualifiedName::new(&[&name_b]);
  assert_eq!("{married: true}", ctx_a.search_entry(&qn_b).unwrap().to_string().as_str());
  let qn_b_married = QualifiedName::new(&[&name_b, &name_married]);
  assert_eq!("true", ctx_a.search_entry(&qn_b_married).unwrap().to_string().as_str());
}

#[test]
fn test_context_search_deep_stops_at_a_value_that_is_no_context() {
  let name_a = Name::from("a");
  let name_b = Name::from("b");
  let mut ctx_b = FeelContext::default();
  ctx_b.set_entry(&name_b, value_number!(2));
  let mut ctx = FeelContext::default();
  ctx.set_entry(&name_a, ctx_b.into());
  assert_eq!(None, ctx.search_deep(&[]));
  assert_eq!("{b: 2}", ctx.search_deep(slice::from_ref(&name_a)).unwrap().to_string());
  assert_eq!("2", ctx.search_deep(&[name_a.clone(), name_b.clone()]).unwrap().to_string());
  // a name after a value that is no context, and a name that no context holds
  assert_eq!(None, ctx.search_deep(&[name_a.clone(), name_b.clone(), name_b.clone()]));
  assert_eq!(None, ctx.search_deep(&[name_a.clone(), name_a.clone()]));
  assert_eq!(None, ctx.search_deep(slice::from_ref(&name_b)));
}

#[test]
fn test_context_search_deep_follows_many_names() {
  // a context nested as deeply as values may nest (diagnostics::MAX_VALUE_DEPTH)
  let name = Name::from("a");
  let mut value = value_number!(1);
  for _ in 0..500 {
    let mut ctx = FeelContext::default();
    ctx.set_entry(&name, value);
    value = ctx.into();
  }
  let Value::Context(ctx) = value else { unreachable!() };
  let names = vec![name.clone(); 500];
  assert_eq!("1", ctx.search_deep(&names).unwrap().to_string());
  assert_eq!(None, ctx.search_deep(&vec![name; 501]));
}

#[test]
fn test_context_search_entries() {
  // prepare names
  let name_a = Name::from("a");
  let name_b = Name::from("b");
  let name_c = Name::from("c");
  let name_d = Name::from("d");
  let name_e = Name::from("e");
  // prepare qualified names
  let qn_empty = QualifiedName::new(&[]);
  let qn_a = QualifiedName::new(&[&name_a]);
  let qn_b = QualifiedName::new(&[&name_b]);
  let qn_c = QualifiedName::new(&[&name_c]);
  let qn_a_b = QualifiedName::new(&[&name_a, &name_b]);
  let qn_a_c = QualifiedName::new(&[&name_a, &name_c]);
  let qn_b_c = QualifiedName::new(&[&name_b, &name_c]);
  let qn_b_e = QualifiedName::new(&[&name_b, &name_e]);
  let qn_b_d = QualifiedName::new(&[&name_b, &name_d]);
  let qn_b_d_a = QualifiedName::new(&[&name_b, &name_d, &name_a]);
  let qn_b_d_e = QualifiedName::new(&[&name_b, &name_d, &name_e]);
  // prepare contexts
  let mut ctx_c: FeelContext = Default::default();
  ctx_c.set_entry(&name_e, Value::String("e".into()));
  assert_eq!(r#"{e: "e"}"#, ctx_c.to_string());
  let mut ctx_b: FeelContext = Default::default();
  ctx_b.set_entry(&name_c, Value::String("c".into()));
  ctx_b.set_entry(&name_d, ctx_c.into());
  assert_eq!(r#"{c: "c", d: {e: "e"}}"#, ctx_b.to_string());
  let mut ctx_a: FeelContext = Default::default();
  ctx_a.set_entry(&name_a, Value::String("a".into()));
  ctx_a.set_entry(&name_b, ctx_b.into());
  assert_eq!(r#"{a: "a", b: {c: "c", d: {e: "e"}}}"#, ctx_a.to_string());
  // test searching entries
  assert!(!ctx_a.contains_entries(&qn_empty));
  assert!(ctx_a.contains_entries(&qn_a));
  assert!(ctx_a.contains_entries(&qn_b));
  assert!(!ctx_a.contains_entries(&qn_c));
  assert!(!ctx_a.contains_entries(&qn_a_b));
  assert!(!ctx_a.contains_entries(&qn_a_c));
  assert!(ctx_a.contains_entries(&qn_b_c));
  assert!(!ctx_a.contains_entries(&qn_b_e));
  assert!(ctx_a.contains_entries(&qn_b_d));
  assert!(!ctx_a.contains_entries(&qn_b_d_a));
  assert!(ctx_a.contains_entries(&qn_b_d_e));
}

#[test]
fn test_context_create_entry() {
  let name_a = Name::from("a");
  let name_b = Name::from("b");
  let name_c = Name::from("c");
  let name_d = Name::from("d");
  let qn_a = QualifiedName::new(&[&name_a]);
  let qn_b = QualifiedName::new(&[&name_b]);
  let qn_a_b = QualifiedName::new(&[&name_a, &name_b]);
  let qn_a_c = QualifiedName::new(&[&name_a, &name_c]);
  let qn_a_d = QualifiedName::new(&[&name_a, &name_d]);
  let qn_c_d = QualifiedName::new(&[&name_c, &name_d]);
  let qn_a_b_c = QualifiedName::new(&[&name_a, &name_b, &name_c]);
  let qn_a_b_c_d = QualifiedName::new(&[&name_a, &name_b, &name_c, &name_d]);
  let mut ctx: FeelContext = Default::default();
  ctx.create_entry(&qn_a_b_c_d, Value::Boolean(true));
  assert_eq!("{a: {b: {c: {d: true}}}}", ctx.to_string());
  assert_eq!("{b: {c: {d: true}}}", ctx.search_entry(&qn_a).unwrap().to_string());
  assert_eq!("{c: {d: true}}", ctx.search_entry(&qn_a_b).unwrap().to_string());
  assert_eq!("{d: true}", ctx.search_entry(&qn_a_b_c).unwrap().to_string());
  assert_eq!("true", ctx.search_entry(&qn_a_b_c_d).unwrap().to_string());
  let mut ctx: FeelContext = Default::default();
  ctx.create_entry(&qn_a, Value::Boolean(true));
  ctx.create_entry(&qn_b, Value::Boolean(false));
  ctx.create_entry(&qn_c_d, Value::String("deep".into()));
  assert_eq!(r#"{a: true, b: false, c: {d: "deep"}}"#, ctx.to_string());
  let mut ctx: FeelContext = Default::default();
  ctx.create_entry(&qn_a_b, Value::String("b".into()));
  ctx.create_entry(&qn_a_c, Value::String("c".into()));
  ctx.create_entry(&qn_a_d, Value::String("d".into()));
  assert_eq!(r#"{a: {b: "b", c: "c", d: "d"}}"#, ctx.to_string());
}

#[test]
fn test_context_create_entries() {
  let name_a: Name = "a".into();
  let name_b: Name = "b".into();
  let name_c: Name = "c".into();
  let name_d: Name = "d".into();
  let v_a = VALUE_TRUE;
  let v_b = VALUE_FALSE;
  let mut ctx: FeelContext = Default::default();
  ctx.create_entries(&[], v_a.clone());
  assert_eq!("{}", ctx.to_string());
  ctx.create_entries(std::slice::from_ref(&name_a), v_a.clone());
  assert_eq!("{a: true}", ctx.to_string());
  ctx.create_entries(&[name_a.clone(), name_b.clone()], v_a.clone());
  assert_eq!("{a: {b: true}}", ctx.to_string());
  ctx.create_entries(&[name_a.clone(), name_b.clone(), name_c.clone()], v_a.clone());
  assert_eq!("{a: {b: {c: true}}}", ctx.to_string());
  ctx.create_entries(&[name_a.clone(), name_b.clone(), name_c, name_d], v_a);
  assert_eq!("{a: {b: {c: {d: true}}}}", ctx.to_string());
  ctx.create_entries(&[name_a, name_b], v_b);
  assert_eq!("{a: {b: false}}", ctx.to_string());
}

#[test]
fn test_context_apply_entries() {
  let name_a: Name = "a".into();
  let name_b: Name = "b".into();
  let name_c: Name = "c".into();
  let name_d: Name = "d".into();
  let v_a = VALUE_TRUE;
  let v_b = VALUE_FALSE;
  let v_c = Value::from(FeelContext::new());
  let mut ctx = FeelContext::new();
  ctx.apply_entries(&[], v_a.clone()).unwrap();
  assert_eq!("{}", ctx.to_string());
  ctx.apply_entries(slice::from_ref(&name_a), v_c.clone()).unwrap();
  assert_eq!("{a: {}}", ctx.to_string());
  ctx.apply_entries(&[name_a.clone(), name_b.clone()], v_c.clone()).unwrap();
  assert_eq!("{a: {b: {}}}", ctx.to_string());
  ctx.apply_entries(&[name_a.clone(), name_b.clone(), name_c.clone()], v_c.clone()).unwrap();
  assert_eq!("{a: {b: {c: {}}}}", ctx.to_string());
  ctx.apply_entries(&[name_a.clone(), name_b.clone(), name_c.clone(), name_d.clone()], v_a.clone()).unwrap();
  assert_eq!("{a: {b: {c: {d: true}}}}", ctx.to_string());
  ctx.apply_entries(&[name_a.clone(), name_b.clone(), name_c.clone()], v_b).unwrap();
  assert_eq!("{a: {b: {c: false}}}", ctx.to_string());
  // net operation should fail, because `c` is not a context
  ctx.apply_entries(&[name_a, name_b, name_c, name_d], v_a).unwrap_err();
}

#[test]
fn test_context_set_null() {
  let name_a = Name::from("a");
  let mut ctx: FeelContext = Default::default();
  ctx.set_null(name_a);
  assert_eq!(r#"{a: null}"#, ctx.to_string());
}

#[test]
fn test_context_insert() {
  let mut ctx: FeelContext = Default::default();
  ctx.insert(Name::from("a"), value_number!(1));
  ctx.insert(Name::from("b"), value_number!(2));
  assert_eq!(r#"{a: 1, b: 2}"#, ctx.to_string());
  // an entry with the same name is replaced
  ctx.insert(Name::from("a"), Value::Boolean(true));
  assert_eq!(r#"{a: true, b: 2}"#, ctx.to_string());
}

#[test]
fn test_context_assign_entry() {
  let name_a = Name::from("a");
  let name_b = Name::from("b");
  let mut ctx: FeelContext = Default::default();
  // a new entry is inserted, and no value is replaced
  assert_eq!(None, ctx.assign_entry(&name_a, value_number!(1)));
  assert_eq!(r#"{a: 1}"#, ctx.to_string());
  // the value of an existing entry is replaced and returned
  assert_eq!(Some(value_number!(1)), ctx.assign_entry(&name_a, value_number!(2)));
  assert_eq!(r#"{a: 2}"#, ctx.to_string());
  assert_eq!(None, ctx.assign_entry(&name_b, value_number!(3)));
  assert_eq!(r#"{a: 2, b: 3}"#, ctx.to_string());
  // a value moved into an entry is taken back out by assigning a placeholder
  assert_eq!(Some(value_number!(3)), ctx.assign_entry(&name_b, Value::Null(None)));
  assert_eq!(r#"{a: 2, b: null}"#, ctx.to_string());
  assert_eq!(2, ctx.len());
}

#[test]
fn test_get_entries() {
  let mut ctx: FeelContext = Default::default();
  assert_eq!(0, ctx.get_entries().len());
  let name_a = Name::new(&["a"]);
  ctx.set_entry(&name_a, value_number!(1));
  assert_eq!(1, ctx.get_entries().len());
  assert_eq!(vec![(&name_a, &value_number!(1))], ctx.get_entries());
  let name_b = Name::new(&["b"]);
  ctx.set_entry(&name_b, value_number!(2));
  assert_eq!(2, ctx.get_entries().len());
  assert_eq!(vec![(&name_a, &value_number!(1)), (&name_b, &value_number!(2))], ctx.get_entries());
}

#[test]
fn test_zip() {
  let name_a = Name::from("a");
  let name_b = Name::from("b");
  let name_c = Name::from("c");
  let mut ctx_a: FeelContext = Default::default();
  ctx_a.set_entry(&name_a, value_number!(1));
  ctx_a.set_entry(&name_b, value_number!(2));
  let mut ctx_b: FeelContext = Default::default();
  ctx_b.set_entry(&name_c, value_number!(3));
  ctx_a.zip(&ctx_b);
  assert_eq!(
    vec![(&name_a, &value_number!(1)), (&name_b, &value_number!(2)), (&name_c, &value_number!(3))],
    ctx_a.get_entries()
  );
}

#[test]
fn test_overwrite() {
  let name_a = Name::from("a");
  let name_b = Name::from("b");
  let name_c = Name::from("c");
  let name_d = Name::from("d");
  let mut ctx_a: FeelContext = Default::default();
  ctx_a.set_entry(&name_a, value_number!(1));
  ctx_a.set_entry(&name_b, value_number!(2));
  ctx_a.set_entry(&name_c, value_number!(3));
  ctx_a.set_entry(&name_d, value_number!(4));
  let mut ctx_b: FeelContext = Default::default();
  ctx_b.set_entry(&name_b, value_number!(20));
  ctx_b.set_entry(&name_c, value_number!(30));
  ctx_a.overwrite(&ctx_b);
  assert_eq!(
    vec![
      (&name_a, &value_number!(1)),
      (&name_b, &value_number!(20)),
      (&name_c, &value_number!(30)),
      (&name_d, &value_number!(4))
    ],
    ctx_a.get_entries()
  );
}

#[test]
fn test_try_from_value_context() {
  let name_a = Name::from("a");
  let name_b = Name::from("b");
  let name_c = Name::from("c");
  let name_d = Name::from("d");
  let mut ctx_a: FeelContext = Default::default();
  ctx_a.set_entry(&name_a, value_number!(1));
  ctx_a.set_entry(&name_b, value_number!(2));
  ctx_a.set_entry(&name_c, value_number!(3));
  ctx_a.set_entry(&name_d, value_number!(4));
  let v = Value::from(ctx_a);
  let ctx_b = FeelContext::try_from(v).unwrap();
  assert_eq!(
    vec![
      (&name_a, &value_number!(1)),
      (&name_b, &value_number!(2)),
      (&name_c, &value_number!(3)),
      (&name_d, &value_number!(4))
    ],
    ctx_b.get_entries()
  );
}

#[test]
fn test_read_api() {
  let name_a = Name::from("a");
  let name_b = Name::from("b");
  let name_c = Name::from("c");
  let mut ctx_a: FeelContext = Default::default();
  ctx_a.set_entry(&name_c, value_number!(3));
  ctx_a.set_entry(&name_a, value_number!(1));
  ctx_a.set_entry(&name_b, value_number!(2));
  assert!(ctx_a.contains_entry(&name_a));
  assert!(!ctx_a.contains_entry(&Name::from("d")));
  assert_eq!(Some(&value_number!(2)), ctx_a.get_entry(&name_b));
  assert_eq!(
    vec![(&name_a, &value_number!(1)), (&name_b, &value_number!(2)), (&name_c, &value_number!(3))],
    ctx_a.iter().collect::<Vec<_>>()
  );
  assert_eq!(3, ctx_a.iter().len());
}

#[test]
fn test_clone() {
  let name_a = Name::from("a");
  let name_b = Name::from("b");
  let name_c = Name::from("c");
  let name_d = Name::from("d");
  let mut ctx_a: FeelContext = Default::default();
  ctx_a.set_entry(&name_a, value_number!(10));
  ctx_a.set_entry(&name_b, value_number!(20));
  ctx_a.set_entry(&name_c, value_number!(30));
  ctx_a.set_entry(&name_d, value_number!(40));
  let ctx_b = ctx_a.clone();
  assert_eq!(
    vec![
      (&name_a, &value_number!(10)),
      (&name_b, &value_number!(20)),
      (&name_c, &value_number!(30)),
      (&name_d, &value_number!(40))
    ],
    ctx_b.get_entries()
  );
}

#[test]
fn test_value_is_not_a_context() {
  assert_eq!(
    "<ContextError> 'true' is not a value containing context",
    <Value as TryInto<FeelContext>>::try_into(Value::Boolean(true)).err().unwrap().to_string()
  );
}

/// A context collected from entries has the entries that inserting them one after another gives:
/// an entry with the name of an earlier one replaces it, whatever the order of the names.
#[test]
fn test_context_collected_from_entries() {
  let entries = |names: &[&str]| -> Vec<(Name, Value)> { names.iter().enumerate().map(|(i, name)| (Name::from(*name), value_number!(i as i64))).collect() };
  for names in [
    &[][..],
    &["a"],
    &["a", "b", "c", "d"],
    &["d", "c", "b", "a"],
    &["a", "a"],
    &["a", "b", "b", "c"],
    &["a", "b", "a"],
    &["b", "a", "b", "a", "c"],
    &["a", "c", "b", "d"],
    &["a b", "a", "a  b", "a b"],
    &["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "c", "n", "a", "z", "y"],
    &["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "m"],
  ] {
    let mut inserted = FeelContext::default();
    for (name, value) in entries(names) {
      inserted.insert(name, value);
    }
    let collected: FeelContext = entries(names).into_iter().collect();
    assert_eq!(inserted, collected, "{names:?}");
    assert_eq!(inserted.get_entries(), collected.get_entries(), "{names:?}");
  }
  let many: Vec<(Name, Value)> = (0..1000).map(|i| (Name::from(format!("k{i:04}")), value_number!(i))).collect();
  let collected: FeelContext = many.clone().into_iter().collect();
  assert_eq!(many.iter().map(|(name, value)| (name, value)).collect::<Vec<_>>(), collected.get_entries());
}

/// A context of shared names is, for every reader, the context of the same entries, and it
/// changes like one: the first change gives it entries of its own, and the names of the other
/// records stay as they were.
#[test]
fn test_context_of_shared_names_is_the_context_of_its_entries() {
  use rheon_feel_number::FeelNumber;
  use std::sync::Arc;
  let texts = ["a", "a b", "b", "k10", "k2", "z"];
  let names: Arc<[Name]> = texts.iter().map(|text| Name::from(*text)).collect();
  let values = |seed: i64| -> Vec<Value> {
    (0..texts.len() as i64)
      .map(|index| {
        if index == 2 {
          Value::from(FeelContext::default())
        } else {
          Value::Number(FeelNumber::new(seed + index, 0))
        }
      })
      .collect()
  };
  let record = FeelContext::from_shared_names(names.clone(), values(10));
  let other = FeelContext::from_shared_names(names.clone(), values(20));
  let plain: FeelContext = texts.iter().map(|text| Name::from(*text)).zip(values(10)).collect();
  // equal, in both directions, and with the same texts
  assert_eq!(plain, record);
  assert_eq!(record, plain);
  assert_ne!(record, other);
  assert_eq!(format!("{plain:?}"), format!("{record:?}"));
  assert_eq!(plain.to_string(), record.to_string());
  assert_eq!(plain.to_feel_string(), record.to_feel_string());
  assert_eq!(plain.jsonify(), record.jsonify());
  // every read
  assert_eq!(plain.len(), record.len());
  assert!(!record.is_empty());
  assert_eq!(plain.get_entries(), record.get_entries());
  assert_eq!(plain.iter().len(), record.iter().len());
  assert_eq!(plain.get_first(), record.get_first());
  for text in texts.iter().chain(&["", "a ", "aa", "k1", "k3", "zz"]) {
    let name = Name::from(*text);
    assert_eq!(plain.get_entry(&name), record.get_entry(&name), "{text}");
    assert_eq!(plain.contains_entry(&name), record.contains_entry(&name), "{text}");
    assert_eq!(plain.contains_entry(*text), record.contains_entry(*text), "{text}");
    assert_eq!(plain.is_context(&name), record.is_context(&name), "{text}");
    assert_eq!(plain.keys_from(text).collect::<Vec<_>>(), record.keys_from(text).collect::<Vec<_>>(), "{text}");
    assert_eq!(plain.search_deep(std::slice::from_ref(&name)), record.search_deep(std::slice::from_ref(&name)), "{text}");
    assert_eq!(
      plain.contains_deep(std::slice::from_ref(&name)),
      record.contains_deep(std::slice::from_ref(&name)),
      "{text}"
    );
  }
  // a clone, and a value that holds the record
  assert_eq!(plain, record.clone());
  assert_eq!(Value::from(plain.clone()), Value::from(record.clone()));
  // every change
  let changes: [fn(&mut FeelContext); 7] = [
    |context| context.set_entry(&Name::from("k2"), Value::Boolean(true)),
    |context| context.insert(Name::from("new"), Value::Boolean(true)),
    |context| {
      context.assign_entry(&Name::from("a"), Value::Boolean(false));
    },
    |context| {
      context.remove_entry(&Name::from("a b"));
    },
    |context| context.set_null(Name::from("z")),
    |context| context.create_entries(&[Name::from("b"), Name::from("inner")], Value::Boolean(true)),
    |context| context.move_entry(Name::from("k10"), Name::from("moved")),
  ];
  for (index, change) in changes.iter().enumerate() {
    let (mut changed_plain, mut changed_record) = (plain.clone(), record.clone());
    change(&mut changed_plain);
    change(&mut changed_record);
    assert_eq!(changed_plain, changed_record, "change {index}");
    assert_eq!(format!("{changed_plain:?}"), format!("{changed_record:?}"), "change {index}");
    assert_ne!(plain, changed_record, "change {index}");
  }
  // the records that share the names are as they were
  assert_eq!(plain, record);
  assert_eq!(names.len(), other.len());
  assert_eq!(Some(&Value::Number(FeelNumber::new(25, 0))), other.get_entry(&Name::from("z")));
  // no entries
  let empty = FeelContext::from_shared_names(Arc::from(Vec::<Name>::new()), vec![]);
  assert_eq!(FeelContext::default(), empty);
  assert!(empty.is_empty());
  assert_eq!(None, empty.get_first());
}
