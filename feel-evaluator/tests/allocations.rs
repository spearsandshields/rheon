//! Allocation counts of evaluations: work that shares values must not copy them.
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
}

fn count_allocation() {
  // the counter is gone while the thread shuts down; nothing is measured then
  let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
}

unsafe impl GlobalAlloc for CountingAllocator {
  unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    count_allocation();
    unsafe { System.alloc(layout) }
  }

  unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
    count_allocation();
    unsafe { System.alloc_zeroed(layout) }
  }

  unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
    count_allocation();
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
