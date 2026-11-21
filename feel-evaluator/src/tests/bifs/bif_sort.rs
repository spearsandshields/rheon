use super::super::*;

#[test]
fn _0001() {
  let scope = &te_scope("{}");
  te_be_value(false, scope, "sort([3,1,4,5,2], function(x,y) x < y)", r#"[1,2,3,4,5]"#);
}

#[test]
fn _0002() {
  let scope = &te_scope("{}");
  te_be_value(false, scope, "sort(list: [3,1,4,5,2], precedes: function(x,y) x < y)", r#"[1,2,3,4,5]"#);
}

#[test]
fn _0003() {
  let scope = &te_scope("{}");
  te_null(
    false,
    scope,
    "sort([3,1,4,5,2], [10,11,12], function(x,y) x < y)",
    r#"expected 2 parameters, actual number of parameters is 3"#,
  );
}

#[test]
fn _0004() {
  let scope = &te_scope("{}");
  te_null(false, scope, "sort([3,1,4,5,2])", r#"expected 2 parameters, actual number of parameters is 1"#);
}

#[test]
fn _0005() {
  let scope = &te_scope("{}");
  te_null(false, scope, "sort()", r#"expected 2 parameters, actual number of parameters is 0"#);
}

#[test]
fn _0006() {
  // 10 is the list [10] for the list parameter (10.3.2.9.4); 10 is no function
  let scope = &te_scope("{}");
  te_null(false, scope, "sort(10,10)", r#"sort: expected ordering function definition as a second argument"#);
}

#[test]
fn _0007() {
  let scope = &te_scope("{}");
  te_null(
    false,
    scope,
    r#"sort([5,4,3,2,1],10)"#,
    r#"sort: expected ordering function definition as a second argument"#,
  );
}

#[test]
fn _0008() {
  let scope = &te_scope("{}");
  te_null(
    false,
    scope,
    r#"sort([3,1,4,5,2], function(x,y,z) x < y + z)"#,
    r#"sort: ordering function should take exactly two arguments"#,
  );
}

#[test]
fn _0009() {
  let scope = &te_scope("{}");
  te_null(
    false,
    scope,
    r#"sort([3,1,4,5,2], function(x) x < 10)"#,
    r#"sort: ordering function should take exactly two arguments"#,
  );
}

#[test]
fn _0010() {
  let scope = &te_scope("{}");
  te_null(
    false,
    scope,
    r#"sort([3,1,4,5,2], function() false)"#,
    r#"sort: ordering function should take exactly two arguments"#,
  );
}

#[test]
fn _0011() {
  let scope = &te_scope("{}");
  te_be_value(false, scope, r#"sort(["a","9","A","10","Aa","8"], function(x,y) x < y)"#, r#"["10","8","9","A","Aa","a"]"#);
}

#[test]
fn _0012() {
  let scope = &te_scope("{}");
  te_null(
    false,
    scope,
    "sort(l: [3,1,4,5,2], precedes: function(x,y) x < y)",
    r#"expected the parameters of sort(list, precedes): list, found the parameter name 'l'"#,
  );
}

#[test]
fn _0013() {
  let scope = &te_scope("{}");
  te_null(
    false,
    scope,
    "sort(list: [3,1,4,5,2], p: function(x,y) x < y)",
    r#"expected the parameters of sort(list, precedes): list, found the parameter name 'p'"#,
  );
}

#[test]
fn _0014() {
  let scope = &te_scope("{}");
  te_be_value(false, scope, "sort([3,1,4,5,2], function(x,y) 10)", r#"[3,1,4,5,2]"#);
}

#[test]
fn _0015() {
  // the ordering function is invoked like any function: its arguments are converted and checked (Table 64)
  let scope = &te_scope("{}");
  te_be_value(false, scope, "sort([3, 1, 2], function(x: number, y: number) x < y)", r#"[1,2,3]"#);
  te_be_value(false, scope, r#"sort([[3], [1], [2]], function(x: number, y: number) x < y)"#, r#"[[1],[2],[3]]"#);
  // not conforming: the invocation is null, so no element precedes another
  te_be_value(false, scope, r#"sort(["b", "a"], function(x: number, y: number) x < y)"#, r#"["b","a"]"#);
}

/// The input lists of the predicate tests: 0 to 200 items, with many equal items.
fn sort_inputs() -> Vec<String> {
  [0, 1, 2, 20, 21, 33, 64, 100, 200]
    .iter()
    .map(|&n| if n == 0 { "[]".to_string() } else { format!("for i in 1..{n} return modulo(i, 7)") })
    .collect()
}

#[test]
fn _0016() {
  // DMN 1.6 §10.3.4.9 states no condition on `precedes`: for every predicate the result is a
  // permutation of the list, the same each time.
  let scope = &te_scope("{}");
  let predicates = [
    "function(x, y) true",
    "function(x, y) false",
    "function(x, y) x <= y",
    "function(x, y) x >= y",
    "function(x, y) x != y",
    "function(x, y) modulo(x * 7, 5) < modulo(y * 3, 5)",
    "function(x, y) modulo(x + y, 3) = 0",
  ];
  for input in sort_inputs() {
    for predicate in predicates {
      let expression = format!(
        "{{input: {input}, result: sort(input, {predicate}), \
         check: count(result) = count(input) and sort(result, function(x, y) x < y) = sort(input, function(x, y) x < y) \
         and sort(input, {predicate}) = result}}.check"
      );
      te_bool(false, scope, &expression, true);
    }
  }
}

#[test]
fn _0017() {
  // For a predicate that is not a strict order the order is still fixed by its answers: a
  // predicate that is always true reverses the list, `<=` puts equal items in reverse order, and
  // one that is always false keeps the list.
  let scope = &te_scope("{}");
  te_value(false, scope, "sort(for i in 1..64 return i, function(x, y) true)", "for i in 0..63 return 64 - i");
  te_value(false, scope, "sort(for i in 1..20 return i, function(x, y) true)", "for i in 0..19 return 20 - i");
  te_value(
    false,
    scope,
    "sort(for i in 1..64 return {k: modulo(i, 4), i: i}, function(x, y) x.k <= y.k)",
    "flatten(for m in 0..3 return (for i in 64..1 return {k: modulo(i, 4), i: i})[item.k = m])",
  );
  te_value(false, scope, "sort(for i in 1..64 return i, function(x, y) false)", "for i in 1..64 return i");
}

#[test]
fn _0018() {
  // For a strict order the result is the stable sort: equal items keep their order.
  let scope = &te_scope("{}");
  te_value(
    false,
    scope,
    "sort(for i in 1..100 return {k: modulo(i, 3), i: i}, function(x, y) x.k < y.k)",
    "flatten(for m in 0..2 return (for i in 1..100 return {k: modulo(i, 3), i: i})[item.k = m])",
  );
  te_value(
    false,
    scope,
    "sort(for i in 1..200 return {k: modulo(i, 5), i: i}, function(x, y) x.k > y.k)",
    "flatten(for m in 4..0 return (for i in 1..200 return {k: modulo(i, 5), i: i})[item.k = m])",
  );
  te_value(
    false,
    scope,
    "sort(for i in 1..1000 return modulo(i * 7919, 1001), function(x, y) x < y)",
    "for i in 1..1000 return i",
  );
}
