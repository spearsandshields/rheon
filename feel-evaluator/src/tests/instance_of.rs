use super::*;
use rheon_feel::{FeelType, scope};

#[test]
fn _0001() {
  let scope = &te_scope(r#"{Order:4.0}"#);
  te_bool(false, scope, "Order instance of number", true);
}

#[test]
fn _0002() {
  let scope = &te_scope(r#"{Order:4.0}"#);
  te_bool(false, scope, "Order instance of string", false);
}

#[test]
fn _0003() {
  let scope = &te_scope(r#"{Order:4.0}"#);
  te_bool(false, scope, "Order instance of boolean", false);
}

#[test]
fn _0004() {
  let scope = &te_scope(r#"{Order:4.0}"#);
  te_bool(false, scope, "Order instance of date", false);
}

#[test]
fn _0005() {
  let scope = &te_scope(r#"{Order:4.0}"#);
  te_bool(false, scope, "Order instance of time", false);
}

#[test]
fn _0006() {
  let scope = &te_scope(r#"{Order:4.0}"#);
  te_bool(false, scope, "Order instance of date and time", false);
}

#[test]
fn _0007() {
  let scope = &te_scope(r#"{Birth Date: @"1983-02-12T13:00:00"}"#);
  te_bool(false, scope, "Birth Date instance of date and time", true);
}

#[test]
fn _0008() {
  let scope = &te_scope(r#"{Birth Date: @"1983-02-12T13:00:00"}"#);
  te_bool(false, scope, "Birth Date instance of Any", true);
}

#[test]
fn _0009() {
  let scope = &te_scope(r#"{Birth Date: @"1983-02-12T13:00:00"}"#);
  te_bool(false, scope, "Birth Date instance of number", false);
}

#[test]
fn _0010() {
  let scope = &te_scope(r#"{Order:4.0}"#);
  te_bool(false, scope, "Order instance of days and time duration", false);
}

#[test]
fn _0011() {
  let scope = &te_scope(r#"{Order:4.0}"#);
  te_bool(false, scope, "Order instance of years and months duration", false);
}

#[test]
fn _0012() {
  let scope = &te_scope(r#"{Order:4.0}"#);
  te_bool(false, scope, "Order instance of Any", true);
}

#[test]
fn _0013() {
  let scope = &te_scope(r#"{Order:4.0}"#);
  te_bool(false, scope, "Order instance of Null", false);
}

#[test]
fn _0014() {
  let scope = &te_scope(r#"{Order:null}"#);
  te_bool(false, scope, "Order instance of Null", true);
}

#[test]
fn _0015() {
  let scope = &te_scope(r#"{Customer:"Business"}"#);
  te_bool(false, scope, "Customer instance of string", true);
}

#[test]
fn _0016() {
  let scope = &te_scope(r#"{Customer:"Business"}"#);
  te_bool(false, scope, "Customer instance of Any", true);
}

#[test]
fn _0017() {
  let scope = &te_scope(r#"{Customer:"Business"}"#);
  te_bool(false, scope, "Customer instance of number", false);
}

#[test]
fn _0018() {
  let scope = &te_scope(r#"{Customer:"Business"}"#);
  te_bool(false, scope, "Customer instance of boolean", false);
}

#[test]
fn _0019() {
  let scope = &te_scope(r#"{Delivery status:true}"#);
  te_bool(false, scope, "Delivery status instance of boolean", true);
}

#[test]
fn _0020() {
  let scope = &te_scope(r#"{Delivery status:true}"#);
  te_bool(false, scope, "Delivery status instance of Any", true);
}

#[test]
fn _0021() {
  let scope = &te_scope(r#"{Delivery status:true}"#);
  te_bool(false, scope, "Delivery status instance of number", false);
}

#[test]
fn _0022() {
  let scope = &te_scope(r#"{Delivery status:true}"#);
  te_bool(false, scope, "Delivery status instance of string", false);
}

#[test]
fn _0023() {
  let scope = &te_scope(r#"{Delivery Time: @"10:11:12"}"#);
  te_bool(false, scope, "Delivery Time instance of time", true);
}

#[test]
fn _0024() {
  let scope = &te_scope(r#"{Delivery Time: @"10:11:12"}"#);
  te_bool(false, scope, "Delivery Time instance of Any", true);
}

#[test]
fn _0025() {
  let scope = &te_scope(r#"{Delivery Time: @"10:11:12"}"#);
  te_bool(false, scope, "Delivery Time instance of string", false);
}

#[test]
fn _0026() {
  let scope = &te_scope(r#"{Delivery Date: @"2023-02-07T10:11:12"}"#);
  te_bool(false, scope, "Delivery Date instance of date and time", true);
}

#[test]
fn _0027() {
  let scope = &te_scope(r#"{Delivery Date: @"2023-02-07T10:11:12"}"#);
  te_bool(false, scope, "Delivery Date instance of Any", true);
}

#[test]
fn _0028() {
  let scope = &te_scope(r#"{Delivery Date: @"2023-02-07T10:11:12"}"#);
  te_bool(false, scope, "Delivery Date instance of string", false);
}

#[test]
fn _0029() {
  let scope = &te_scope(r#"{Delivery Date: @"2023-02-07"}"#);
  te_bool(false, scope, "Delivery Date instance of date", true);
}

#[test]
fn _0030() {
  let scope = &te_scope(r#"{Delivery Date: @"2023-02-07"}"#);
  te_bool(false, scope, "Delivery Date instance of Any", true);
}

#[test]
fn _0031() {
  let scope = &te_scope(r#"{Delivery Date: @"2023-02-07"}"#);
  te_bool(false, scope, "Delivery Date instance of string", false);
}

#[test]
fn _0032() {
  let scope = &te_scope(r#"{Delivery Duration: @"P1DT2H"}"#);
  te_bool(false, scope, "Delivery Duration instance of days and time duration", true);
}

#[test]
fn _0033() {
  let scope = &te_scope(r#"{Delivery Duration: @"P1DT2H"}"#);
  te_bool(false, scope, "Delivery Duration instance of Any", true);
}

#[test]
fn _0034() {
  let scope = &te_scope(r#"{Delivery Duration: @"P1DT2H"}"#);
  te_bool(false, scope, "Delivery Duration instance of string", false);
}

#[test]
fn _0035() {
  let scope = &te_scope(r#"{Delivery Duration: @"P1Y3M"}"#);
  te_bool(false, scope, "Delivery Duration instance of years and months duration", true);
}

#[test]
fn _0036() {
  let scope = &te_scope(r#"{Delivery Duration: @"P1Y3M"}"#);
  te_bool(false, scope, "Delivery Duration instance of Any", true);
}

#[test]
fn _0037() {
  let scope = &te_scope(r#"{Delivery Duration: @"P1Y3M"}"#);
  te_bool(false, scope, "Delivery Duration instance of string", false);
}

#[test]
fn _0038() {
  let scope = &te_scope(r#"{Orders:[1,2,3,4]}"#);
  te_bool(false, scope, "Orders instance of list<number>", true);
}

#[test]
fn _0039() {
  let scope = &te_scope(r#"{Orders:[1,2,3,4]}"#);
  te_bool(false, scope, "Orders instance of Any", true);
}

#[test]
fn _0040() {
  let scope = &te_scope(r#"{Orders:[1,2,3,4]}"#);
  te_bool(false, scope, "Orders instance of list<string>", false);
}

#[test]
fn _0041() {
  let scope = &te_scope(r#"{Orders:[1,2,3,4]}"#);
  te_bool(false, scope, "Orders instance of list<boolean>", false);
}

#[test]
fn _0042() {
  let scope = &te_scope(r#"{Orders:[1,2,3,4]}"#);
  te_bool(false, scope, "Orders instance of list<date>", false);
}

#[test]
fn _0043() {
  let scope = &te_scope(r#"{Orders:[1,2,3,4]}"#);
  te_bool(false, scope, "Orders instance of list<time>", false);
}

#[test]
fn _0044() {
  let scope = &te_scope(r#"{Orders:[1,2,3,4]}"#);
  te_bool(false, scope, "Orders instance of list<date and time>", false);
}

#[test]
fn _0045() {
  let scope = &te_scope(r#"{Orders:[1,2,3,4]}"#);
  te_bool(false, scope, "Orders instance of list<years and months duration>", false);
}

#[test]
fn _0046() {
  let scope = &te_scope(r#"{Orders:[1,2,3,4]}"#);
  te_bool(false, scope, "Orders instance of list<days and time duration>", false);
}

#[test]
fn _0047() {
  let scope = &te_scope(r#"{Items:[1..10]}"#);
  te_bool(false, scope, "Items instance of range<number>", true);
}

#[test]
fn _0048() {
  let scope = &te_scope(r#"{Items:[1..10]}"#);
  te_bool(false, scope, "Items instance of Any", true);
}

#[test]
fn _0049() {
  let scope = &te_scope(r#"{Items:[1..10]}"#);
  te_bool(false, scope, "Items instance of range<string>", false);
}

#[test]
fn _0050() {
  let scope = &te_scope(r#"{Items:[1..10]}"#);
  te_bool(false, scope, "Items instance of range<boolean>", false);
}

#[test]
fn _0051() {
  let scope = &te_scope(r#"{Items:[1..10]}"#);
  te_bool(false, scope, "Items instance of range<date>", false);
}

#[test]
fn _0052() {
  let scope = &te_scope(r#"{Items:[1..10]}"#);
  te_bool(false, scope, "Items instance of range<time>", false);
}

#[test]
fn _0053() {
  let scope = &te_scope(r#"{Items:[1..10]}"#);
  te_bool(false, scope, "Items instance of range<date and time>", false);
}

#[test]
fn _0054() {
  let scope = &te_scope(r#"{Items:[1..10]}"#);
  te_bool(false, scope, "Items instance of range<years and months duration>", false);
}

#[test]
fn _0055() {
  let scope = &te_scope(r#"{Items:[1..10]}"#);
  te_bool(false, scope, "Items instance of range<days and time duration>", false);
}

#[test]
fn _0056() {
  let scope = &te_scope(r#"{Person:{name:"John",age:49}}"#);
  te_bool(false, scope, "Person instance of context<name:string>", true);
}

#[test]
fn _0057() {
  let scope = &te_scope(r#"{Person:{name:"John",age:49}}"#);
  te_bool(false, scope, "Person instance of Any", true);
}

#[test]
fn _0058() {
  let scope = &te_scope(r#"{Person:{name:"John",age:49}}"#);
  te_bool(false, scope, "Person instance of context<name:string,age:number,car:string>", false);
}

#[test]
fn _0059() {
  let scope = &te_scope(r#"{Person:{name:"John",age:49}}"#);
  te_bool(false, scope, "Person instance of context<name:string,a:number>", false);
}

#[test]
fn _0060() {
  let scope = &te_scope(r#"{Person:{name:"John",age:49}}"#);
  te_bool(false, scope, "Person instance of context<n:string,age:number>", false);
}

#[test]
fn _0061() {
  let scope = &te_scope(r#"{Person:{name:"John",age:49}}"#);
  te_bool(false, scope, "Person instance of context<name:string,age:number>", true);
}

#[test]
fn _0062() {
  let scope = &te_scope(r#"{Person:{name:"John",age:49}}"#);
  te_bool(false, scope, "Person instance of function<string>->string", false);
}

#[test]
fn _0063() {
  let scope = &te_scope(r#"{Power: 25.5,engine:{power:280.5}}"#);
  // a value on the right maps to no node of the type lattice: null (Table 62)
  te_null(false, scope, "Power instance of engine.power", "instance of expected a type on the right, found number");
}

#[test]
fn _0064() {
  let scope = &te_scope(r#"{multiple: function(x: number,y: number) x * y}"#);
  te_bool(false, scope, "multiple instance of function<number,number>->Any", true);
}

#[test]
fn _0065() {
  let scope = &te_scope(r#"{multiple: function(x: number,y: number) x * y}"#);
  te_bool(false, scope, "multiple instance of Any", true);
}

#[test]
fn _0066() {
  let scope = &te_scope(r#"{multiple: function(x: number,y: number) x * y}"#);
  te_bool(false, scope, "multiple instance of number", false);
}

#[test]
fn _0067() {
  te_bool(false, &scope!(), "null instance of Null", true);
}

#[test]
fn _0068() {
  te_bool(false, &scope!(), "null instance of Any", false);
}

#[test]
fn _0069() {
  te_bool(false, &scope!(), "null instance of number", false);
}

#[test]
fn _0070() {
  let node = AstNode::InstanceOf(Box::new(AstNode::Irrelevant), Box::new(AstNode::FeelType(FeelType::Number)));
  assert_eq!(
    r#"null(invalid value in 'instance of' operator: Irrelevant)"#,
    crate::tests::eval(&scope!(), &node).to_string()
  );
}

/// The type of the items of a list is the type of the item that the types of all the others
/// conform to, wherever that item stands: a null beside numbers is a list of numbers.
#[test]
fn the_type_of_a_list_does_not_depend_on_the_order_of_its_items() {
  for (items, feel_type, expected) in [
    (vec!["null", "1"], "list<number>", true),
    (vec!["null", "1", "null", "2"], "list<number>", true),
    (vec!["null", "1"], "list<string>", false),
    (vec!["null", "1", r#""a""#], "list<number>", false),
    (vec!["null", "1", r#""a""#], "list<Any>", true),
    (vec!["[null]", "[1]"], "list<list<number>>", true),
    (vec!["[null]", "[1]", "[]"], "list<list<number>>", true),
    (vec!["{a: null}", "{a: 1}"], "list<context<a: number>>", true),
    (vec!["{a: 1, b: 2}", "{a: 1}"], "list<context<a: number>>", true),
    (vec!["{a: 1, b: 2}", "{a: 1}"], "list<context<a: number, b: number>>", false),
    (vec!["{a: 1}", "{b: 1}"], "list<context<a: number>>", false),
    // null conforms to every type (Table 62: conformance)
    (vec!["null", "null"], "list<number>", true),
    (vec!["null", "null"], "list<Null>", true),
  ] {
    // every rotation of the items, and their reverse
    let mut items = items;
    for _ in 0..items.len() {
      items.rotate_left(1);
      for list in [items.clone(), items.iter().rev().cloned().collect()] {
        let text = format!("[{}] instance of {feel_type}", list.join(", "));
        te_bool(false, &scope!(), &text, expected);
      }
    }
  }
  // the type that a message names
  te_null(
    false,
    &scope!(),
    "abs([null, 1, 2])",
    "[core::abs] invalid argument type, expected number, actual type is list<number>",
  );
  te_null(
    false,
    &scope!(),
    "abs([1, null, 2])",
    "[core::abs] invalid argument type, expected number, actual type is list<number>",
  );
}

#[test]
fn built_in_functions_are_of_function_types() {
  // a built-in is of a function type when one of its signatures is (10.3.2.9.2)
  let scope = &te_scope("{}");
  te_bool(false, scope, "abs instance of Any", true);
  // abs of a number is a number, and abs of a duration a duration (TCK 0092 decision_014_1)
  te_bool(false, scope, "abs instance of function<number>->number", true);
  te_bool(false, scope, "abs instance of function<years and months duration>->years and months duration", true);
  te_bool(false, scope, "abs instance of function<number>->string", false);
  te_bool(false, scope, "abs instance of function<number>->Any", true);
  te_bool(false, scope, "sqrt instance of function<number>->number", true);
  te_bool(false, scope, "sqrt instance of function<string>->number", false);
  te_bool(false, scope, "sqrt instance of function<number, number>->number", false);
  te_bool(false, scope, "string length instance of function<string>->number", true);
  te_bool(false, scope, "sum instance of function<number, number, number>->number", true);
  te_bool(false, scope, "sort instance of function<list<number>, function<number, number>->boolean>->list<Any>", true);
  te_bool(false, scope, "abs instance of number", false);
}

#[test]
fn a_rest_parameter_takes_one_argument_or_more() {
  let scope = &te_scope("{}");
  te_bool(false, scope, "sum instance of function<>->number", false);
  te_bool(false, scope, "sum instance of function<number>->number", true);
  // instance of holds a result of type Any against the target
  te_bool(false, scope, "max instance of function<list<number>>->number", false);
}

#[test]
fn the_context_type_without_entries_is_every_context() {
  // DMN 1.7 Beta 1, grammar rule 54: context<>
  let scope = &te_scope("{}");
  te_bool(false, scope, "{a: 1} instance of context<>", true);
  te_bool(false, scope, "{} instance of context<>", true);
  te_bool(false, scope, "1 instance of context<>", false);
  te_number(false, scope, "(function(c: context<>) c.a)({a: 5})", 5, 0);
}
