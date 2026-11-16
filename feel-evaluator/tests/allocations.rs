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
  let value = Value::from(wide_context(10_000));
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
  Value::from(context)
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
fn a_list_read_by_name_and_filtered_is_not_copied() {
  let number = |n: i64| Value::Number(n.into());
  let expression = "count(items[item = null]) + count(items)";
  for (kind, (allocations, bytes)) in [("numbers", cost_of_a_long_list(expression, number)), ("contexts", cost_of_a_long_list(expression, record))] {
    assert!(
      allocations <= 8 && bytes < 4 * 1024,
      "{expression} over 10,000 {kind}: {allocations} allocations and {bytes} bytes more than over one"
    );
  }
}

#[test]
fn a_reason_quoting_the_type_of_a_wide_context_allocates_little() {
  let mut input = rheon_feel::context::FeelContext::default();
  input.set_entry(&rheon_feel::Name::from("c"), Value::from(wide_context(10_000)));
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

/// The allocations and the bytes the evaluation of `expression` makes with the input `data` a
/// context of 10,000 entries, less those it makes with `data` a context of one entry: the cost
/// that grows with the context. The expression answers the entry `entry0`, the number 0.
fn cost_of_a_wide_context(expression: &str) -> (i64, i64) {
  let measure = |entries: i64| {
    let mut input = rheon_feel::context::FeelContext::default();
    input.set_entry(&rheon_feel::Name::from("data"), Value::from(wide_context(entries)));
    let scope = FeelScope::from(input);
    let evaluator = prepare(&parse_expression(&scope, expression, false).expect("the expression parses"));
    let allocations = ALLOCATIONS.with(Cell::get);
    let (value, bytes) = bytes_allocated_by(|| evaluator(&scope));
    let allocations = ALLOCATIONS.with(Cell::get) - allocations;
    assert_eq!(Value::Number(0.into()), value, "{expression}");
    (allocations as i64, bytes as i64)
  };
  let (wide_allocations, wide_bytes) = measure(10_000);
  let (one_allocations, one_bytes) = measure(1);
  (wide_allocations - one_allocations, wide_bytes - one_bytes)
}

#[test]
fn a_context_read_by_name_and_passed_on_is_not_copied() {
  for expression in [
    "data.entry0",
    "get value(data, \"entry0\")",
    "{f: function(c) c.entry0, r: f(data)}.r",
    "{d: data, r: d.entry0}.r",
  ] {
    let (allocations, bytes) = cost_of_a_wide_context(expression);
    assert!(
      allocations <= 8 && bytes < 4 * 1024,
      "{expression} over 10,000 entries: {allocations} allocations and {bytes} bytes more than over one"
    );
  }
}

/// The allocations of `calls` calls of a function that closes over `names` names of a context
/// literal and answers 0 without reading them.
fn calls_of_a_closure(names: usize, calls: usize) -> i64 {
  let entries: Vec<String> = (0..names).map(|i| format!("a{i}: {i}")).collect();
  let read: Vec<String> = (0..names).map(|i| format!("a{i}")).collect();
  let expression = format!(
    "{{{}, f: function() if false then [{}] else 0, r: count(for i in 1..{calls} return f())}}.r",
    entries.join(", "),
    read.join(", ")
  );
  let (value, allocations) = evaluate_counting(&scope!(), &expression);
  assert_eq!(Value::Number(calls.into()), value, "{expression}");
  allocations as i64
}

/// The allocations that 1,000 more calls cost do not grow with the closure.
#[test]
fn a_call_puts_the_closure_in_scope_without_a_copy() {
  let more_calls = |names: usize| calls_of_a_closure(names, 2_000) - calls_of_a_closure(names, 1_000);
  let (many, one) = (more_calls(50), more_calls(1));
  assert!(many <= one + 8, "1,000 more calls closing over 50 names: {many} allocations more, over 1 name: {one}");
}

/// The allocations and the bytes of the evaluation of `expression` with `{n}` replaced by 10,000,
/// less those with it replaced by 1: the cost that grows with the number of iterations.
fn cost_of_iterations(expression: &str) -> (i64, i64) {
  let measure = |n: usize| {
    let scope = scope!();
    let node = parse_expression(&scope, &expression.replace("{n}", &n.to_string()), false).expect("the expression parses");
    let evaluator = prepare(&node);
    let allocations = ALLOCATIONS.with(Cell::get);
    let (_, bytes) = bytes_allocated_by(|| evaluator(&scope));
    ((ALLOCATIONS.with(Cell::get) - allocations) as i64, bytes as i64)
  };
  let (many, one) = (measure(10_000), measure(1));
  (many.0 - one.0, many.1 - one.1)
}

/// The results of 10,000 iterations grow a vector of 10,000 values by doubling: 12 allocations of
/// 2,096,640 bytes in all. The caps are those figures and a tenth.
const RESULTS_ALLOCATIONS: i64 = 13;
const RESULTS_BYTES: i64 = 2_306_304;

/// A walk allocates nothing per iteration: only its results grow.
#[test]
fn loops_allocate_only_for_their_results() {
  for expression in [
    "count(for i in 1..{n} return i)",
    "(for i in 1..{n} return if i = 1 then 1 else partial[-1] + 1)[-1]",
    "some i in for j in 1..{n} return j satisfies i < 0",
    "every i in for j in 1..{n} return j satisfies i > 0",
  ] {
    let (allocations, bytes) = cost_of_iterations(expression);
    assert!(
      allocations <= RESULTS_ALLOCATIONS && bytes <= RESULTS_BYTES,
      "{expression}: {allocations} allocations and {bytes} bytes more over 10,000 than over one"
    );
  }
  // two variables: the domain of the second is made again for each value of the first
  let (allocations, _) = cost_of_iterations("count(for i in 1..2, j in 1..{n} return j)");
  assert!(allocations <= RESULTS_ALLOCATIONS + 1, "{allocations} allocations");
}

/// `partial` is the walk's results, not a copy of them: an iteration that reads it and invokes a
/// built-in allocates for the invocation, never the results again.
#[test]
fn partial_is_read_without_a_copy_of_the_results() {
  let (_, bytes) = cost_of_iterations("count(for i in 1..{n} return count(partial))");
  // 2,816,568 bytes measured, and a tenth
  assert!(bytes <= 3_098_225, "{bytes} bytes more over 10,000 iterations than over one");
}

/// A filter allocates nothing per item: only the items it keeps grow.
#[test]
fn filters_allocate_only_for_the_items_they_keep() {
  let number = |n: i64| Value::Number(n.into());
  for (kind, (allocations, bytes)) in [
    ("numbers", cost_of_a_long_list("count(items[item >= 0])", number)),
    ("contexts", cost_of_a_long_list("count(items[n >= 0])", record)),
    ("contexts", cost_of_a_long_list("count(items[item.n >= 0])", record)),
    ("contexts", cost_of_a_long_list("count(items[s != null])", record)),
  ] {
    assert!(
      allocations <= RESULTS_ALLOCATIONS && bytes <= RESULTS_BYTES,
      "over 10,000 {kind}: {allocations} allocations and {bytes} bytes more than over one"
    );
  }
  let (allocations, bytes) = cost_of_a_long_list("count(items[item >= 0][item >= 0])", number);
  assert!(
    allocations <= 2 * RESULTS_ALLOCATIONS && bytes <= 2 * RESULTS_BYTES,
    "{allocations} allocations, {bytes} bytes"
  );
}

/// A function of the host invoked in a walk gets the walk's variables and `partial` in its scope,
/// and they are not held after the invocation: the results grow without a copy.
#[test]
fn a_function_of_the_host_invoked_in_a_walk_holds_no_copy_of_its_results() {
  let measure = |n: usize| {
    let input = parse_expression(&scope!(), "{f: function(x) x}", false).expect("the context parses");
    let Value::Context(input) = prepare(&input)(&scope!()) else { panic!("not a context") };
    let scope = FeelScope::from(std::sync::Arc::unwrap_or_clone(input));
    let evaluator = prepare(&parse_expression(&scope, &format!("count(for i in 1..{n} return f(i))"), false).expect("the expression parses"));
    let allocations = ALLOCATIONS.with(Cell::get);
    let (value, bytes) = bytes_allocated_by(|| evaluator(&scope));
    assert_eq!(Value::Number(n.into()), value);
    ((ALLOCATIONS.with(Cell::get) - allocations) as i64, bytes as i64)
  };
  let (many, one) = (measure(10_000), measure(1));
  let (allocations, bytes) = (many.0 - one.0, many.1 - one.1);
  // 30,009 allocations and 12,665,583 bytes measured (3 allocations and about 1 KiB an
  // invocation), and a tenth; a copy of the results in each iteration would take gigabytes
  assert!(
    allocations <= 33_010 && bytes <= 13_932_141,
    "{allocations} allocations and {bytes} bytes more over 10,000 than over one"
  );
}

/// An invocation of a function of FEEL allocates nothing: its arguments are bound in place and its
/// frame is a window of slots that the run keeps; positional, named, typed and recursive.
#[test]
fn invocations_of_functions_of_feel_allocate_nothing() {
  for expression in [
    "{f: function(x) x + 1, r: count(for i in 1..{n} return f(i))}.r",
    "{f: function(a, b) a - b, r: count(for i in 1..{n} return f(b: 1, a: i))}.r",
    "{f: function(a: number) a + 1, r: count(for i in 1..{n} return f(i))}.r",
    "{f: function(n) if n = 0 then 0 else f(n - 1), r: count(for i in 1..{n} return f(3))}.r",
  ] {
    let (allocations, bytes) = cost_of_iterations(expression);
    assert!(
      allocations <= RESULTS_ALLOCATIONS && bytes <= RESULTS_BYTES,
      "{expression}: {allocations} allocations and {bytes} bytes more over 10,000 than over one"
    );
  }
}

/// In FEEL a built-in function reads its positional arguments where they are: an invocation
/// allocates only what the function itself allocates (`abs` one value).
#[test]
fn invocations_of_built_in_functions_allocate_no_arguments() {
  let (allocations, bytes) = cost_of_iterations("count(for i in 1..{n} return abs(i))");
  // 10,011 allocations and 2,176,632 bytes measured, and a tenth of the bytes
  assert!(
    allocations <= 10_000 + RESULTS_ALLOCATIONS && bytes <= 2_394_295,
    "{allocations} allocations and {bytes} bytes more over 10,000 than over one"
  );
}

/// Functions of the host invoked from the entries of a context literal get the entries before
/// them in one scope that the literal keeps and extends: what the invocations allocate grows
/// with the entries, not with their square.
#[test]
fn functions_of_the_host_invoked_from_context_entries_share_one_scope() {
  use rheon_feel::closure::Closure;
  use rheon_feel::context::FeelContext;
  use rheon_feel::{Evaluator, FeelType, FunctionBody, FunctionDefinition, Name};
  use std::sync::Arc;
  // a function whose body is the host's: it reads its parameter from the scope it is given
  let body: Evaluator = Box::new(|scope: &FeelScope| scope.get_value(&Name::from("x")).unwrap_or(Value::Null(None)));
  let function = FunctionDefinition::new(
    vec![(Name::from("x"), FeelType::Any)],
    FunctionBody::Context(Arc::new(body)),
    false,
    Closure::default(),
    FeelContext::default(),
    FeelType::Any,
  );
  let mut input = FeelContext::default();
  input.set_entry(&Name::from("f"), Value::FunctionDefinition(Arc::new(function)));
  let scope = FeelScope::from(input);
  let measure = |n: usize| {
    let entries: Vec<String> = (1..=n).map(|i| format!("a{i}: f({i})")).collect();
    let evaluator = prepare(&parse_expression(&scope, &format!("{{{}}}.a{n}", entries.join(", ")), false).expect("the expression parses"));
    let (value, bytes) = bytes_allocated_by(|| evaluator(&scope));
    assert_eq!(Value::Number(n.into()), value);
    bytes as i64
  };
  let (small, large) = (measure(200), measure(400));
  // twice the entries allocate about twice the bytes; a scope for each entry would allocate
  // about four times as many
  assert!(large <= 3 * small, "{large} bytes for 400 entries, {small} bytes for 200");
}
