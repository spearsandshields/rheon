//! Allocation counts of evaluations: work that shares values must not copy them, and a message
//! never builds a whole value or type.
//!
//! This test binary installs a counting global allocator. The counters are per thread, so tests
//! that run in parallel do not see each other's allocations.

use rheon_feel::values::Value;
use rheon_feel::{FeelScope, scope};
use rheon_feel_evaluator::prepare;
use rheon_feel_parser::parse_expression;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct CountingAllocator;

thread_local! {
  static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
  /// The bytes requested by those allocations (a reallocation requests its new size).
  static BYTES: Cell<u64> = const { Cell::new(0) };
}

fn count_allocation(bytes: usize) {
  // the counters are gone while the thread shuts down; nothing is measured then
  let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
  let _ = BYTES.try_with(|count| count.set(count.get() + bytes as u64));
}

unsafe impl GlobalAlloc for CountingAllocator {
  unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    count_allocation(layout.size());
    unsafe { System.alloc(layout) }
  }

  unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
    count_allocation(layout.size());
    unsafe { System.alloc_zeroed(layout) }
  }

  unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
    count_allocation(new_size);
    unsafe { System.realloc(ptr, layout, new_size) }
  }

  unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
    unsafe { System.dealloc(ptr, layout) }
  }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Returns the result of an expression and the number of allocations its evaluation made
/// (parsing and building the evaluator are not counted).
fn evaluate_counting(scope: &FeelScope, expression: &str) -> (Value, u64) {
  let node = parse_expression(scope, expression, false).expect("the expression parses");
  let evaluator = prepare(&node);
  let before = ALLOCATIONS.with(Cell::get);
  let value = evaluator(scope);
  let after = ALLOCATIONS.with(Cell::get);
  (value, after - before)
}

fn assert_list_of(len: usize, item: &Value, value: &Value) {
  let Value::List(items) = value else { panic!("expected a list, got {value}") };
  assert_eq!(len, items.len());
  assert!(items.iter().all(|value| value == item), "expected only {item}");
}

#[test]
fn a_string_literal_in_a_loop_allocates_no_string_per_iteration() {
  let scope = scope!();
  let (strings, string_allocations) = evaluate_counting(&scope, r#"for i in 1..1000 return "a string literal""#);
  let (booleans, boolean_allocations) = evaluate_counting(&scope, "for i in 1..1000 return true");
  assert_list_of(1_000, &Value::String("a string literal".into()), &strings);
  assert_list_of(1_000, &Value::Boolean(true), &booleans);
  assert!(
    string_allocations <= boolean_allocations + 8,
    "1,000 strings from a literal: {string_allocations} allocations, 1,000 booleans: {boolean_allocations}"
  );
}

/// Returns the result of `f` and the bytes it allocated.
fn bytes_allocated_by<T>(f: impl FnOnce() -> T) -> (T, u64) {
  let before = BYTES.with(Cell::get);
  let value = f();
  (value, BYTES.with(Cell::get) - before)
}

/// A context of `entries` number entries.
fn wide_context(entries: i64) -> rheon_feel::context::FeelContext {
  let mut context = rheon_feel::context::FeelContext::default();
  for i in 0..entries {
    context.set_entry(&rheon_feel::Name::from(format!("entry{i}")), Value::Number(i.into()));
  }
  context
}

#[test]
fn a_message_quoting_the_type_of_a_wide_context_allocates_little() {
  use rheon_feel::message::{arg, compose};
  use rheon_feel::values::TypeOf;
  let value = Value::Context(wide_context(10_000));
  let (message, bytes) = bytes_allocated_by(|| compose(format_args!("actual type is '{}'", arg(&TypeOf(&value)))));
  assert!(message.ends_with("… (10000 entries)'"), "{message}");
  assert!(bytes <= 4 * 1024, "{bytes} bytes allocated");
}

/// The allocations and the bytes the evaluation of `expression` makes over a list of 10,000
/// `item`s, less those it makes over a list of the one item `0`: the cost that grows with the list.
fn cost_of_a_long_list(expression: &str, item: impl Fn(i64) -> Value) -> (i64, i64) {
  let measure = |items: Vec<Value>| {
    let len = items.len();
    let mut input = rheon_feel::context::FeelContext::default();
    input.set_entry(&rheon_feel::Name::from("items"), Value::List(items.into()));
    let scope = FeelScope::from(input);
    let evaluator = prepare(&parse_expression(&scope, expression, false).expect("the expression parses"));
    let allocations = ALLOCATIONS.with(Cell::get);
    let (value, bytes) = bytes_allocated_by(|| evaluator(&scope));
    let allocations = ALLOCATIONS.with(Cell::get) - allocations;
    assert_eq!(Value::Number(len.into()), value, "{expression}");
    (allocations as i64, bytes as i64)
  };
  let (long_allocations, long_bytes) = measure((0..10_000).map(&item).collect());
  let (one_allocations, one_bytes) = measure(vec![item(0)]);
  (long_allocations - one_allocations, long_bytes - one_bytes)
}

/// A context with a number and a string entry.
fn record(n: i64) -> Value {
  let mut context = rheon_feel::context::FeelContext::default();
  context.set_entry(&rheon_feel::Name::from("n"), Value::Number(n.into()));
  context.set_entry(&rheon_feel::Name::from("s"), Value::String(format!("item {n}").into()));
  Value::Context(context)
}

#[test]
fn a_list_read_by_name_and_passed_on_is_not_copied() {
  let number = |n: i64| Value::Number(n.into());
  for expression in ["count(items)", "{f: function(l) count(l), r: f(items)}.r"] {
    for (kind, (allocations, bytes)) in [("numbers", cost_of_a_long_list(expression, number)), ("contexts", cost_of_a_long_list(expression, record))] {
      assert!(
        allocations <= 8 && bytes < 4 * 1024,
        "{expression} over 10,000 {kind}: {allocations} allocations and {bytes} bytes more than over one"
      );
    }
  }
}

#[test]
fn a_reason_quoting_the_type_of_a_wide_context_allocates_little() {
  let mut input = rheon_feel::context::FeelContext::default();
  input.set_entry(&rheon_feel::Name::from("c"), Value::Context(wide_context(10_000)));
  let scope = FeelScope::from(input);
  let evaluate = |text: &str| {
    let evaluator = prepare(&parse_expression(&scope, text, false).expect("the expression parses"));
    bytes_allocated_by(|| evaluator(&scope))
  };
  let (_, read) = evaluate("c");
  let (value, invoked) = evaluate("abs(c)");
  assert!(matches!(&value, Value::Null(Some(reason)) if reason.ends_with("… (10000 entries)")), "{value}");
  assert!(invoked <= read + 64 * 1024, "abs(c) allocated {invoked} bytes, c {read}");
}
