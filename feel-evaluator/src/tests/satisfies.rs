use super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  satisfies(false, &scope!(), r#"1"#, r#""#, r#"-"#, true);
}

#[test]
fn _0002() {
  satisfies(false, &scope!(), r#" "a" "#, r#""#, r#"-"#, true);
}

#[test]
fn _0003() {
  satisfies(false, &scope!(), r#" @"2023-02-06" "#, r#""#, r#"-"#, true);
}

#[test]
fn _0004() {
  satisfies(false, &scope!(), r#" @"2023-02-06T10:11:12" "#, r#""#, r#"-"#, true);
}

#[test]
fn _0005() {
  satisfies(false, &scope!(), r#" @"10:11:12" "#, r#""#, r#"-"#, true);
}

#[test]
fn _0006() {
  satisfies(false, &scope!(), r#" @"P1D" "#, r#""#, r#"-"#, true);
}

#[test]
fn _0007() {
  satisfies(false, &scope!(), r#" @"P1Y" "#, r#""#, r#"-"#, true);
}

#[test]
fn _0008() {
  satisfies(false, &scope!(), r#" false "#, r#""#, r#"-"#, true);
}

#[test]
fn _0009() {
  satisfies(false, &scope!(), r#" true "#, r#""#, r#"-"#, true);
}

#[test]
fn _0010() {
  satisfies(false, &scope!(), r#"1"#, r#""#, r#"[]"#, false);
}

#[test]
fn _0011() {
  satisfies(false, &scope!(), r#"1"#, r#""#, r#"[1]"#, true);
}

#[test]
fn _0012() {
  satisfies(false, &scope!(), r#"10"#, r#""#, r#"[9,10,11]"#, true);
}

#[test]
fn _0013() {
  satisfies(false, &scope!(), "8", r#""#, r#"[9,10,11]"#, false);
}

#[test]
fn _0014() {
  satisfies(false, &scope!(), "12", r#""#, r#"[9,10,11]"#, false);
}

#[test]
fn _0015() {
  satisfies(false, &scope!(), "10", r#""#, r#"[9],[10],[11]"#, true);
}

#[test]
fn _0016() {
  satisfies(false, &scope!(), "8", r#""#, r#"[9],[10],[11]"#, false);
}

#[test]
fn _0017() {
  satisfies(false, &scope!(), "12", r#""#, r#"[9],[10],[11]"#, false);
}

#[test]
fn _0018() {
  satisfies(false, &scope!(), "3", r#""#, r#"not([2,3,4])"#, false);
}

#[test]
fn _0019() {
  satisfies(false, &scope!(), "1", r#""#, r#"not([2,3,4])"#, true);
}

#[test]
fn _0020() {
  satisfies(false, &scope!(), "3", r#""#, r#"not([2..4])"#, false);
}

#[test]
fn _0021() {
  satisfies(false, &scope!(), "1", r#""#, r#"not([2..4])"#, true);
}

#[test]
fn _0022() {
  satisfies(false, &scope!(), "1", r#""#, r#"not(2,3,4)"#, true);
}

#[test]
fn _0023() {
  satisfies(false, &scope!(), "3", r#""#, r#"not(2,3,4)"#, false);
}

#[test]
fn _0024() {
  satisfies(false, &scope!(), "null", r#""#, r#"not(2,"a",null,3,4)"#, false);
}

#[test]
fn _0025() {
  satisfies(false, &scope!(), "null", r#""#, r#"not(2,"a",3,4)"#, true);
}

#[test]
fn _0026() {
  satisfies(false, &scope!(), "true", r#""#, r#"not(2,"a",null,true,3,4)"#, false);
}

#[test]
fn _0027() {
  // no test is true and `false = 2` has no answer: the tests are null, and so is their negation
  satisfies_null(false, &scope!(), "false", r#""#, r#"not(2,"a",true,3,4)"#, "equal err 'false' =?= '2'");
  satisfies(false, &scope!(), "false", r#""#, r#"not(true)"#, true);
}

#[test]
fn _0028() {
  satisfies(false, &scope!(), "10", r#""#, r#"not(<20)"#, false);
}

#[test]
fn _0029() {
  satisfies(false, &scope!(), "10", r#""#, r#"not(<10)"#, true);
}

#[test]
fn _0030() {
  satisfies(false, &scope!(), "10", r#""#, r#"not(<=20)"#, false);
}

#[test]
fn _0031() {
  satisfies(false, &scope!(), "10", r#""#, r#"not(<=9)"#, true);
}

#[test]
fn _0032() {
  satisfies(false, &scope!(), "10", r#""#, r#"not(>9)"#, false);
}

#[test]
fn _0033() {
  satisfies(false, &scope!(), "10", r#""#, r#"not(>11)"#, true);
}

#[test]
fn _0034() {
  satisfies(false, &scope!(), "10", r#""#, r#"not(>=10)"#, false);
}

#[test]
fn _0035() {
  satisfies(false, &scope!(), "10", r#""#, r#"not(>=11)"#, true);
}

#[test]
fn _0036() {
  satisfies(false, &scope!(), r#" "a" "#, r#""#, r#"[ "a", "b", "c" ]"#, true);
}

#[test]
fn _0037() {
  satisfies(false, &scope!(), r#" "d" "#, r#""#, r#"[ "a", "b", "c" ]"#, false);
}

#[test]
fn _0038() {
  satisfies(false, &scope!(), r#" "a" "#, r#""#, r#"not("a","b","c")"#, false);
}

#[test]
fn _0039() {
  satisfies(false, &scope!(), r#" "k" "#, r#""#, r#"not("a","b","c")"#, true);
}

#[test]
fn _0040() {
  satisfies(false, &scope!(), r#" @"2023-02-06" "#, r#""#, r#"not(@"2023-02-05",@"2023-02-06",@"2023-02-07")"#, false);
}

#[test]
fn _0041() {
  satisfies(false, &scope!(), r#" @"2023-02-08" "#, r#""#, r#"not(@"2023-02-05",@"2023-02-06",@"2023-02-07")"#, true);
}

#[test]
fn _0042() {
  satisfies(
    false,
    &scope!(),
    r#" @"2023-02-06T10:11:12" "#,
    r#""#,
    r#"not(@"2023-02-05T10:11:12",@"2023-02-06T10:11:12",@"2023-02-07T10:11:12")"#,
    false,
  );
}

#[test]
fn _0043() {
  satisfies(
    false,
    &scope!(),
    r#" @"2023-02-06T10:11:13" "#,
    r#""#,
    r#"not(@"2023-02-05T10:11:12",@"2023-02-06T10:11:12",@"2023-02-07T10:11:12")"#,
    true,
  );
}

#[test]
fn _0044() {
  satisfies(false, &scope!(), r#" @"10:11:12" "#, r#""#, r#"not(@"10:11:12",@"10:11:12",@"10:11:12")"#, false);
}

#[test]
fn _0045() {
  satisfies(false, &scope!(), r#" @"10:11:13" "#, r#""#, r#"not(@"10:11:12",@"10:11:12",@"10:11:12")"#, true);
}

#[test]
fn _0046() {
  satisfies(false, &scope!(), r#" @"P2D" "#, r#""#, r#"not(@"P1D",@"P2D",@"P3D")"#, false);
}

#[test]
fn _0047() {
  satisfies(false, &scope!(), r#" @"P4D" "#, r#""#, r#"not(@"P1D",@"P2D",@"P3D")"#, true);
}

#[test]
fn _0048() {
  satisfies(false, &scope!(), r#" @"P2Y" "#, r#""#, r#"not(@"P1Y",@"P2Y",@"P3Y")"#, false);
}

#[test]
fn _0049() {
  satisfies(false, &scope!(), r#" @"P4Y" "#, r#""#, r#"not(@"P1Y",@"P2Y",@"P3Y")"#, true);
}

#[test]
fn _0050() {
  satisfies_null(
    false,
    &scope!(),
    "1",
    r#""#,
    r#"not(2,3,function() 1.23)"#,
    r#"unexpected type in negated list: function<>->Any"#,
  );
}

#[test]
fn _0051() {
  satisfies(false, &scope!(), "5", "", "not(=4,=5,=6)", false);
}

#[test]
fn _0052() {
  satisfies(false, &scope!(), "5", "", "not(!=4,!=5,!=6)", true);
}

#[test]
fn _0053() {
  satisfies(false, &scope!(), "5", "", "not(!=4,=5,!=6)", false);
}

#[test]
fn _0054() {
  satisfies(false, &scope!(), "5", "", "not(!=5,=5)", false);
}

#[test]
fn _0055() {
  satisfies(false, &scope!(), "4", "", "not(!=5)", false);
}

#[test]
fn _0056() {
  satisfies(false, &scope!(), "5", "", "not(!=5)", true);
}

#[test]
fn _0057() {
  satisfies(false, &scope!(), "5", "", "not(=5)", false);
}

#[test]
fn _0058() {
  satisfies(false, &scope!(), "4", "", "not(=5)", true);
}

/// A test that uses `?` and gives a boolean is satisfied when it is true (DMN 1.6 clause
/// 7.3.2 d, Table 56); with another value it is compared with the tested value.
#[test]
fn a_boolean_test_using_the_tested_value_is_its_own_answer() {
  // the tested value is bound to `?`, as a decision table and the unary tests of a host bind it
  let yes = |input: &str, tests: &str, expected: bool| satisfies(false, &te_scope(&format!("{{?: {input}}}")), "?", "", tests, expected);
  let null = |input: &str, tests: &str, expected: &str| satisfies_null(false, &te_scope(&format!("{{?: {input}}}")), "?", "", tests, expected);
  yes("7", "? > 5", true);
  yes("4", "? > 5", false);
  yes("7", "? > 5 and ? < 10", true);
  yes("7", "? > 9, 7", true);
  yes("7", "not(? > 5)", false);
  yes("4", "not(? > 5)", true);
  yes(r#""ab""#, r#"starts with(?, "a")"#, true);
  yes(r#""b""#, r#"starts with(?, "a")"#, false);
  yes("[1, 2]", "count(?) > 1", true);
  // no boolean: the value is compared
  yes("7", "(? * 0) + 7", true);
  yes("7", "abs(?)", true);
  yes("false", "?", true);
  // the expression has no answer for the tested value
  null(r#""a""#, "? > 5", "comparison '>' expected string on the right, as on the left, found number");
  null(
    "null",
    "? > 5",
    "comparison '>' expected a number, string, date, time, date and time, or duration on the left, found Null",
  );
}

/// The tests of a list are joined by `or` (DMN 1.6 Table 56): a tested value of another kind
/// than every test has no answer, and neither has the negation (clause 8.3.3 matches on
/// `false` there).
#[test]
fn tests_of_another_kind_than_the_tested_value_have_no_answer() {
  let scope = scope!();
  satisfies_null(false, &scope, r#""a""#, "", "1, 2", r#"equal err '"a"' =?= '1'"#);
  satisfies_null(false, &scope, r#""a""#, "", "< 10", "eval_in_unary_less");
  satisfies_null(false, &scope, r#""a""#, "", "not(1, 2)", r#"equal err '"a"' =?= '1'"#);
  satisfies(false, &scope, r#""a""#, "", r#"1, "a""#, true);
  satisfies(false, &scope, "3", "", "1, 2", false);
  satisfies(false, &scope, "3", "", "not(1, 2)", true);
  satisfies(false, &scope, "null", "", "not(1, 2)", true);
  satisfies(false, &scope, "null", "", "1, null", true);
}
