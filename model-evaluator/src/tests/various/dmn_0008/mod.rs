//! What a boxed context writes in the body of a boxed `for` or as the match of a boxed filter. A
//! boxed context writes each named entry into the context on the top of the scope: in a boxed
//! `for` that is the iteration context, in a boxed filter the context of the item. These tests pin
//! what the next iteration, the next item, the result and the enclosing scope see of those writes.

use super::super::*;

const DMN_0008: &str = include_str!("_0008.dmn");

model_evaluator!(DMN_0008);

const MODEL_NAMESPACE: &str = "https://dsntk.io/boxed-frames";
const MODEL_NAME: &str = "boxed frames";

fn decision(name: &str, expected: &str) {
  assert_decision(&MODEL_EVALUATOR, MODEL_NAMESPACE, MODEL_NAME, name, &context("{}"), expected);
}

#[test]
fn _0001() {
  // an entry written in one iteration is not seen by the next
  decision("for reads an earlier iteration", "[1, 2, 3]");
  // an entry named `partial` replaces the results of the iterations before it: with a list, by
  // that list, with any other value, by no results
  decision("for writes partial", "[3, 1]");
  decision("for writes partial a number", "[3]");
  // an entry is not seen by the entries before it
  decision("for reads a later entry", "[[0, 10], [0, 20], [0, 30]]");
}

#[test]
fn _0002() {
  // an entry written for one item is not seen for the next
  decision("filter reads an earlier item", "[{a: 2}, {a: 3}]");
  // an entry named `item` is read by the entries after it and leaves the items unchanged
  decision("filter writes item", "[2, 3]");
  // an entry named as an entry of the item is not read by the entries after it
  decision("filter writes an entry of the item", "[]");
  // an entry is not seen by the entries before it
  decision("filter reads a later entry", "[{a: 1}, {a: 2}, {a: 3}]");
}

#[test]
fn _0003() {
  // what the match writes when the filter evaluates it in the enclosing scope is not seen there
  decision("filter leaves the enclosing scope", "[[1, 2], null(context has no value for key 'seen')]");
}
