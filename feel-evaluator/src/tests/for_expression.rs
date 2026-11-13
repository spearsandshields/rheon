use super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_be_value(false, &scope!(), r#"for n in [12,8,32] return n + 1"#, "[13,9,33]");
}

#[test]
fn _0002() {
  te_be_value(false, &scope!(), r#"for n in 1..10 return n - 1"#, "[0,1,2,3,4,5,6,7,8,9]");
}

#[test]
fn _0003() {
  let scope = &te_scope(r#"{N:4}"#);
  te_be_value(
    false,
    scope,
    r#"
      for i in 1..N return
        if i = 1 then
          1
        else
          i * partial[-1]
    "#,
    "[1,2,6,24]",
  );
}

#[test]
fn _0004() {
  let scope = &te_scope(r#"{N:5}"#);
  te_be_value(
    false,
    scope,
    r#"
      for i in 0..N return
        if i = 0 then
          1
        else if i = 1 then
          1 
        else 
          partial[-1] + partial[-2]
    "#,
    "[1,1,2,3,5,8]",
  );
}

#[test]
fn _0005() {
  te_be_value(
    false,
    &scope!(),
    r#"
      for x in [1,2,3],
          y in [6,7,8]
      return x + y
    "#,
    "[7,8,9,8,9,10,9,10,11]",
  );
}

#[test]
fn _0006() {
  te_be_value(
    false,
    &scope!(),
    r#"for x in [1,2,3], y in [6,7,8], z in [-1,-2,-3] return x + y + z"#,
    "[6,5,4,7,6,5,8,7,6,7,6,5,8,7,6,9,8,7,8,7,6,9,8,7,10,9,8]",
  );
}

#[test]
fn _0007() {
  te_be_value(false, &scope!(), r#"for n in 4..2 return n"#, "[4,3,2]");
}

#[test]
fn _0008() {
  te_be_value(
    false,
    &scope!(),
    r#"for i in 1..3, j in [4,5] return {a: i, b: j}"#,
    "[{a: 1, b: 4}, {a: 1, b: 5}, {a: 2, b: 4}, {a: 2, b: 5}, {a: 3, b: 4}, {a: 3, b: 5}]",
  );
}

#[test]
fn _0009() {
  te_be_value(
    false,
    &scope!(),
    r#"for j in [4,5], i in 1..3 return {a: i, b: j}"#,
    "[{a: 1, b: 4}, {a: 2, b: 4}, {a: 3, b: 4}, {a: 1, b: 5}, {a: 2, b: 5}, {a: 3, b: 5}]",
  );
}

#[test]
fn _0010() {
  //--------------------------------------------------------------------------------------------------------------------
  // `x` variable iterates over a list. But the elements of this list are also lists (nested lists).
  // The result is a list of elements, but because each element is also a list,
  // the we get a nested list as a result.
  //--------------------------------------------------------------------------------------------------------------------
  te_be_value(false, &scope!(), "for x in [[1,2],[3,4]] return x", "[[1, 2], [3, 4]]");
}

#[test]
fn _0011() {
  //--------------------------------------------------------------------------------------------------------------------
  // In the previous test we get a nested list as a result from iteration done by variable `x`.
  // So the nested loop with variable `y` also iterates over a nested list, so we get the same result
  // as in previous test.
  //--------------------------------------------------------------------------------------------------------------------
  te_be_value(false, &scope!(), "for x in [[1,2],[3,4]] return for y in x return y", "[[1, 2], [3, 4]]");
}

#[test]
fn _0012() {
  //--------------------------------------------------------------------------------------------------------------------
  // This is tricky. `x` iterates over values that are lists. So in each iteration,
  // the value of `x` is a list (the nested one).
  // So the variable `y` always sees a list of numbers, so it iterates over numbers,
  // and because we care only about variable `y` in the result (which is a number in each iteration),
  // so the final result is a list of numbers.
  //--------------------------------------------------------------------------------------------------------------------
  te_be_value(false, &scope!(), "for x in [[1,2],[3,4]], y in x return y", "[1, 2, 3, 4]");
}

#[test]
fn _0013() {
  //--------------------------------------------------------------------------------------------------------------------
  // Binding variable pointing to another variable can not be the before the pointed one.
  //--------------------------------------------------------------------------------------------------------------------
  te_null(false, &scope!(), "(for y in x, x in [[1,2],[3,4]] return y)[1]", "context has no value for key 'x'");
  te_null(false, &scope!(), "(for y in x, x in [[1,2],[3,4]] return y)[2]", "context has no value for key 'x'");
}

#[test]
fn _0014() {
  //--------------------------------------------------------------------------------------------------------------------
  // Binding variable is repeated in the output, because is not in the last iteration context.
  //--------------------------------------------------------------------------------------------------------------------
  te_be_value(
    false,
    &scope!(),
    "for x in [[1,2],[3,4]], y in x, z in [8,9,10] return y",
    "[1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4]",
  );
}

#[test]
fn _0015() {
  //--------------------------------------------------------------------------------------------------------------------
  // Iterating over empty list should return an empty list.
  //--------------------------------------------------------------------------------------------------------------------
  te_be_value(false, &scope!(), "for x in [] return x", "[]");
}

#[test]
fn _0016() {
  //--------------------------------------------------------------------------------------------------------------------
  // Iterating over empty list in first context should return an empty list.
  //--------------------------------------------------------------------------------------------------------------------
  te_be_value(false, &scope!(), "for x in [], y in [1,2] return y", "[]");
}

#[test]
fn _0017() {
  //--------------------------------------------------------------------------------------------------------------------
  // Iterating over empty list in last context should return an empty list.
  //--------------------------------------------------------------------------------------------------------------------
  te_be_value(false, &scope!(), "for x in [1,2], y in [] return x", "[]");
}

#[test]
fn _0018() {
  //--------------------------------------------------------------------------------------------------------------------
  // Iterating over empty list in any context should return an empty list.
  //--------------------------------------------------------------------------------------------------------------------
  te_be_value(false, &scope!(), "for x in [1,2,3], y in [], z in [4,5] return z", "[]");
}

#[test]
fn _0019() {
  //--------------------------------------------------------------------------------------------------------------------
  // Binding variable points on another one, but the list is not nested.
  //--------------------------------------------------------------------------------------------------------------------
  te_be_value(false, &scope!(), "for x in [1,2,3,4], y in x return y", "[1, 2, 3, 4]");
}

#[test]
fn _0020() {
  //--------------------------------------------------------------------------------------------------------------------
  // Binding variables chain.
  //--------------------------------------------------------------------------------------------------------------------
  te_be_value(false, &scope!(), "for x in [1,2,3,4], y in x, z in y return z", "[1, 2, 3, 4]");
}

#[test]
fn _0021() {
  //--------------------------------------------------------------------------------------------------------------------
  // Binding variables chain with nested lists.
  //--------------------------------------------------------------------------------------------------------------------
  te_be_value(false, &scope!(), "for x in [[1,2],[3,4]], y in x, z in y return z", "[1, 2, 3, 4]");
}

#[test]
fn _0022() {
  //--------------------------------------------------------------------------------------------------------------------
  // Binding variables chain with triple nested lists.
  //--------------------------------------------------------------------------------------------------------------------
  te_be_value(
    false,
    &scope!(),
    "for x in [[[1,2],[3,4]],[[5,6],[7,8]]], y in x, z in y return z",
    "[1, 2, 3, 4, 5, 6, 7, 8]",
  );
}

#[test]
fn _0023() {
  // Numbers have 34 significant digits: `1e34 + 1 = 1e34`, so the step is lost and the
  // iteration must stop instead of running forever (regression). A descending range starting
  // at 1e34 is not affected (`1e34 - 1` is representable) and simply has 1e34 elements.
  te_be_value(false, &scope!(), "for i in 1e34..1e34 return i", "[1e34]");
  te_be_value(false, &scope!(), "for i in 1e34..(1e34 + 2) return i", "[1e34]");
  te_be_value(false, &scope!(), "for i in 1e35..1e34 return i", "[1e35]");
  te_be_value(false, &scope!(), "for i in -1e40..-1e40 return i", "[-1e40]");
  te_be_value(false, &scope!(), "for i in 1e40..1e40, j in 1..2 return i + j", "[1e40, 1e40]");
  te_be_value(false, &scope!(), "count(for i in 1e33..(1e33 + 2) return i)", "3");
  te_be_value(
    false,
    &scope!(),
    "for i in 9999999999999999999999999999999998..9999999999999999999999999999999999 return i",
    "[9999999999999999999999999999999998, 9999999999999999999999999999999999]",
  );
}

/// Evaluates an expression in an empty scope.
fn eval(expression: &str) -> rheon_feel::values::Value {
  let scope = scope!();
  let node = rheon_feel_parser::parse_expression(&scope, expression, false).unwrap();
  crate::evaluate(&scope, &node)
}

#[test]
fn _0024() {
  // A filter whose index is neither a number nor a boolean returns early; the contexts it pushed
  // must not stay on the scope: every iteration gets its own null, and the loop variable is not
  // visible after the loop (0.3.0 returned `[null]` and `s: 3`).
  let nulls = eval(r#"for i in 1..3 return [1,2]["x"]"#);
  assert_eq!(
    "[null(only number or boolean indexes are allowed in filters), null(only number or boolean indexes are allowed in filters), null(only number or boolean indexes are allowed in filters)]",
    nulls.to_string()
  );
  assert!(eval(r#"{r: for i in 1..3 return [1,2]["x"], s: i}.s"#).is_null());
  assert_eq!("3", eval(r#"count({r: for i in 1..3 return [1,2]["x"]}.r)"#).to_string());
  // the same for some / every bodies and function bodies
  assert!(eval(r#"{r: some i in [1, 2] satisfies [1]["x"] = null, s: i}.s"#).is_null());
  assert!(eval(r#"{f: function(a) [1]["x"], r: f(1), s: a}.s"#).is_null());
}

#[test]
fn _0025() {
  // `partial`, loop variables, nesting: the iteration context is reused across iterations
  te_be_value(false, &scope!(), "for partial in 1..3 return partial", "[[], [[]], [[], [[]]]]");
  te_be_value(false, &scope!(), "for i in 1..3 return partial", "[[], [[]], [[], [[]]]]");
  te_be_value(false, &scope!(), "for i in 1..4 return count(partial) + i", "[1, 3, 5, 7]");
  te_be_value(false, &scope!(), "for i in 1..3 return for j in 1..i return j", "[[1], [1, 2], [1, 2, 3]]");
  te_be_value(false, &scope!(), "for i in 1..2 return for j in 1..2 return count(partial)", "[[0, 1], [0, 1]]");
  te_be_value(false, &scope!(), "for x in [[1,2],[3]], y in x return y", "[1, 2, 3]");
  te_be_value(false, &scope!(), "for i in 1..2, j in 1..3 return i * 10 + j", "[11, 12, 13, 21, 22, 23]");
  te_be_value(false, &scope!(), "for i in [] return i", "[]");
  te_be_value(false, &scope!(), "for i in [1, 2], j in [] return i", "[]");
  te_be_value(
    false,
    &scope!(),
    r#"for d in date("2020-02-28")..date("2020-03-01") return d"#,
    r#"[date("2020-02-28"), date("2020-02-29"), date("2020-03-01")]"#,
  );
  te_be_value(
    false,
    &scope!(),
    r#"for d in date("2020-01-02")..date("2020-01-01") return d"#,
    r#"[date("2020-01-02"), date("2020-01-01")]"#,
  );
  te_be_value(false, &scope!(), "for i in 1..3 return {a: i}.a", "[1, 2, 3]");
  assert!(eval("{x: for i in 1..2 return i, y: i}.y").is_null());
}

#[test]
fn _0026() {
  // Date ranges that end at the last or the first supported date. With several iteration
  // contexts the iteration is their cross product (§10.3.2.14), so every step of an outer
  // context iterates the date range again from its start.
  te_be_value(
    false,
    &scope!(),
    r#"for i in 1..2, d in @"262142-12-30"..@"262142-12-31" return string(d)"#,
    r#"["262142-12-30", "262142-12-31", "262142-12-30", "262142-12-31"]"#,
  );
  te_be_value(
    false,
    &scope!(),
    r#"for i in 1..2, d in @"-262143-01-02"..@"-262143-01-01" return string(d)"#,
    r#"["-262143-01-02", "-262143-01-01", "-262143-01-02", "-262143-01-01"]"#,
  );
  te_be_value(
    false,
    &scope!(),
    r#"for i in 1..2, d in @"262142-12-30"..@"262142-12-31", j in 1..2 return string(d)"#,
    r#"["262142-12-30", "262142-12-30", "262142-12-31", "262142-12-31", "262142-12-30", "262142-12-30", "262142-12-31", "262142-12-31"]"#,
  );
  te_be_value(
    false,
    &scope!(),
    r#"for d in @"262142-12-30"..@"262142-12-31", i in 1..2 return string(d)"#,
    r#"["262142-12-30", "262142-12-30", "262142-12-31", "262142-12-31"]"#,
  );
  te_be_value(
    false,
    &scope!(),
    r#"for i in 1..2, d in @"262142-12-31"..@"262142-12-31" return string(d)"#,
    r#"["262142-12-31", "262142-12-31"]"#,
  );
}
