#[test]
fn _0001() {
  let node = rheon_feel_parser::parse_range_literal("[1..100)").unwrap();
  let expected = r#"
       Range
       ├─ IntervalStart (closed)
       │  └─ Numeric
       │     └─ `1`
       └─ IntervalEnd (opened)
          └─ Numeric
             └─ `100`
    "#;
  assert_eq!(expected, node.trace());
}
