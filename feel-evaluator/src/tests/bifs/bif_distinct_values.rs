use super::super::*;
use rheon_feel::scope;

#[test]
fn _0001() {
  te_be_value(false, &scope!(), r#"distinct values([1,2,3,2,1])"#, r#"[1,2,3]"#);
}

#[test]
fn _0002() {
  te_be_value(false, &scope!(), r#"distinct values([1,"A",true,2,1,"A",true])"#, r#"[1,"A",true,2]"#);
}

#[test]
fn _0003() {
  te_be_value(false, &scope!(), r#"distinct values([])"#, r#"[]"#);
}

#[test]
fn _0004() {
  te_be_value(false, &scope!(), r#"distinct values(list: [1,2,3,2,1])"#, r#"[1,2,3]"#);
}

#[test]
fn _0005() {
  // a value that is no list is the list of it where a list is taken (10.3.2.9.4)
  te_be_value(false, &scope!(), r#"distinct values(8)"#, r#"[8]"#);
}

#[test]
fn _0006() {
  te_null(
    false,
    &scope!(),
    r#"distinct values(l:[1])"#,
    r#"expected the parameters of distinct values(list): list, found the parameter name 'l'"#,
  );
}

#[test]
fn _0007() {
  te_null(false, &scope!(), r#"distinct values()"#, r#"expected 1 parameters, actual number of parameters is 0"#);
}

#[test]
fn many_dates_and_times_are_kept_apart_by_their_keys() {
  // equal dates and times meet under their keys: 20,000 values are not compared pairwise
  let scope = &te_scope("{}");
  let started = std::time::Instant::now();
  te_number(
    false,
    scope,
    r#"count(distinct values(for i in 1..20000 return date and time("2020-01-01T00:00:00Z") + duration("PT" + string(modulo(i, 10000)) + "S")))"#,
    10000,
    0,
  );
  te_number(
    false,
    scope,
    r#"count(distinct values(for i in 1..20000 return time("00:00:00") + duration("PT" + string(modulo(i, 5000)) + "S")))"#,
    5000,
    0,
  );
  assert!(started.elapsed().as_secs() < 10, "took {:?}", started.elapsed());
  // a local value equals a zoned one of its wall-clock time, and zoned ones of one instant are equal
  te_number(
    false,
    scope,
    r#"count(distinct values(concatenate(for i in 1..20 return @"2020-01-01T10:00:00Z", [@"2020-01-01T12:00:00+02:00", @"2020-01-01T10:00:00"])))"#,
    1,
    0,
  );
}
