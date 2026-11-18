//! The rules of name resolution, each shown on the names an expression reads.

use crate::flat::resolve::{Info, NONE, Resolved, Stop, resolve, uses};
use rheon_feel::{FeelScope, Name};
use rheon_feel_parser::AstNode;

fn parse(text: &str) -> AstNode {
  rheon_feel_parser::parse_expression(&FeelScope::default(), text, false).unwrap_or_else(|e| panic!("{text}: {e}"))
}

/// What each name of `resolved` reads, in the order of the text.
fn reads_of(resolved: &Resolved) -> Vec<String> {
  fn visit(resolved: &Resolved, node: u32, out: &mut Vec<String>) {
    let rnode = &resolved.nodes[node as usize];
    if let Info::Read(read) = &rnode.info {
      let name = match rnode.ast {
        AstNode::Name(name) => name.to_string(),
        AstNode::QualifiedName(segments) => segments
          .iter()
          .map(|segment| match segment {
            AstNode::QualifiedNameSegment(name) => name.to_string(),
            _ => "?".to_string(),
          })
          .collect::<Vec<_>>()
          .join("."),
        other => format!("{other:?}"),
      };
      out.push(format!("{name}: {}", resolved.read_text(read)));
    }
    for &kid in resolved.kids(node) {
      visit(resolved, kid, out);
    }
  }
  let mut out = vec![];
  visit(resolved, resolved.root(), &mut out);
  out
}

fn reads(text: &str) -> Vec<String> {
  reads_of(&resolve(&parse(text)))
}

/// The free names of `text` with where they are read.
fn frees(text: &str) -> Vec<String> {
  let ast = parse(text);
  let resolved = resolve(&ast);
  resolved
    .frees
    .iter()
    .map(|free| {
      let mut used = vec![];
      for (bit, label) in [
        (uses::ROOT, "root"),
        (uses::FUNCTION_BODY, "body"),
        (uses::FILTER_PREDICATE, "filter"),
        (uses::CALLEE, "callee"),
      ] {
        if free.uses & bit != 0 {
          used.push(label);
        }
      }
      format!(
        "{}{} {}",
        resolved.names[free.name as usize],
        if free.qualified { " qualified" } else { "" },
        used.join("+")
      )
    })
    .collect()
}

#[test]
fn an_entry_sees_the_other_entries_and_not_its_own_key() {
  // `e` is evaluated before `d`, which reads it: its slot is the fourth
  assert_eq!(vec!["a: slot 0", "c: free 0", "e: slot 3"], reads("{a: 1, b: a, c: c, d: e, e: 2}"));
  // entries that need each other in a circle read the names outside the literal
  assert_eq!(vec!["a: free 0", "b: slot 0"], reads("{a: b, b: a}"));
}

#[test]
fn nested_contexts_see_the_entries_of_the_outer_ones() {
  assert_eq!(vec!["a: slot 0", "c: slot 3", "b: slot 1"], reads("{a: 1, b: {c: a, d: c}, e: b}"));
}

#[test]
fn a_context_stops_at_a_repeated_key() {
  let ast = parse("{a: 1, a: a, b: a}");
  let resolved = resolve(&ast);
  assert!(matches!(resolved.nodes[resolved.root() as usize].info, Info::Context { stop: Stop::Duplicate(1), .. }));
  assert_eq!(vec!["a: slot 0", "a: slot 0"], reads_of(&resolved));
  assert_eq!(1, resolved.root.binders);
}

#[test]
fn a_context_of_other_entries_stops_at_the_first() {
  let ast = AstNode::Context(vec![
    AstNode::ContextEntry(Box::new(AstNode::ContextEntryKey(Name::from("a"))), Box::new(AstNode::Null)),
    AstNode::Name(Name::from("a")),
  ]);
  let resolved = resolve(&ast);
  assert!(matches!(resolved.nodes[resolved.root() as usize].info, Info::Context { stop: Stop::Irregular(1), .. }));
  assert_eq!(vec!["a: slot 0"], reads_of(&resolved));
}

#[test]
fn a_name_has_one_index_however_many_names_come_before_it() {
  // `partial` is the 17th name and `z` the first after it: the names before them are still found
  let keys: Vec<String> = (1..=14).map(|k| format!("k{k}: {k}")).collect();
  let text = format!("{{{}, r: for i in [1] return [z, k2]}}", keys.join(", "));
  assert_eq!(vec!["z: free 0", "k2: slot 1"], reads(&text));
  // a key repeated after `partial` and a new name is still a repeated key
  let keys: Vec<String> = (1..=16).map(|k| format!("k{k}: {k}")).collect();
  let ast = parse(&format!("{{{}, partial: 0, k17: 0, k1: 2}}", keys.join(", ")));
  let resolved = resolve(&ast);
  assert!(matches!(resolved.nodes[resolved.root() as usize].info, Info::Context { stop: Stop::Duplicate(18), .. }));
  // a free name read before and after `partial` is reported once
  let names: Vec<String> = (1..=16).map(|k| format!("a{k}")).collect();
  let text = format!("[{}, partial, b, a1]", names.join(", "));
  assert_eq!(18, frees(&text).len(), "{:?}", frees(&text));
}

#[test]
fn a_domain_sees_the_variables_before_it() {
  assert_eq!(
    vec!["x: slot 0", "x: slot 0", "y: slot 1", "x: slot 0", "y: slot 1", "z: slot 2", "partial: partial r0"],
    reads("for x in [1], y in [x], z in [x, y] return [x, y, z, partial]")
  );
}

#[test]
fn the_first_domain_is_outside_the_iteration() {
  assert_eq!(vec!["x: free 0", "x: slot 0"], reads("for x in [x] return x"));
}

#[test]
fn partial_is_hidden_from_later_domains_in_a_for() {
  // the walk removes `partial` before every later domain, a variable of that name as well
  assert_eq!(vec!["partial: free 0", "y: slot 1"], reads("for partial in [1, 2], y in [partial] return y"));
  assert_eq!(vec!["partial: partial r0"], reads("for partial in [1, 2] return partial"));
  // not in `some` and `every`, which have no results so far
  assert_eq!(vec!["partial: slot 0", "y: slot 1"], reads("some partial in [1, 2], y in [partial] satisfies y"));
}

#[test]
fn a_filter_probes_its_item_first() {
  assert_eq!(vec!["item: item r0, free 0", "a: item r0, free 1"], reads("[1][item > a]"));
  assert_eq!(vec!["item: item r0, free 0", "item: item r1, item r0, free 0"], reads("[[1]][item[item > 0] = []]"));
}

#[test]
fn a_binder_inside_a_predicate_hides_the_item() {
  assert_eq!(vec!["item: slot 0"], reads("[1][some item in [1] satisfies item]"));
  assert_eq!(vec!["x: slot 0", "x: item r1, slot 0"], reads("for x in [1] return [x][x > 0]"));
}

#[test]
fn parameters_of_one_name_share_a_slot() {
  let ast = parse("function(a, b, a) a + b + c");
  let resolved = resolve(&ast);
  assert_eq!(vec!["a: slot 0", "b: slot 1", "c: free 0"], reads_of(&resolved));
  assert_eq!([0, 1, 0], *resolved.functions[0].params);
  assert_eq!(vec!["c body"], frees("function(a, b, a) a + b + c"));
}

#[test]
fn a_body_reads_the_names_it_does_not_bind_from_outside() {
  // `f` is not seen by its own value: the body reads it from the scopes of its callers
  assert_eq!(vec!["f: free 0", "n: slot 0", "f: slot 0"], reads("{f: function(n) f(n - 1), r: f(2)}"));
  assert_eq!(vec!["f body+callee"], frees("{f: function(n) f(n - 1), r: f(2)}"));
}

#[test]
fn types_of_parameters_are_read_where_the_function_is_defined() {
  assert_eq!(vec!["t: slot 0", "x: slot 0"], reads("for t in [1] return function(x: t) x"));
}

#[test]
fn qualified_names_of_one_segment_read_as_names() {
  assert_eq!(vec!["t: free 0"], reads("2 instance of t"));
  assert_eq!(vec!["t qualified root"], frees("2 instance of t"));
  assert_eq!(vec!["t: slot 0"], reads("for t in [1] return 2 instance of t"));
}

#[test]
fn qualified_names_of_several_segments_are_searched() {
  let segment = |name: &str| AstNode::QualifiedNameSegment(Name::from(name));
  let ast = AstNode::QualifiedName(vec![segment("a"), segment("b")]);
  assert_eq!(vec!["a.b: search"], reads_of(&resolve(&ast)));
  assert_eq!(vec!["a.b: search"], reads("for a in [{b: 1}] return 2 instance of a.b"));
  // the report says where the first segment is read
  assert_eq!(vec!["a qualified root+filter"], frees("[{a: {b: 1}}][2 instance of a.b]"));
  let callee = AstNode::FunctionInvocation(Box::new(ast), Box::new(AstNode::PositionalParameters(vec![])));
  let resolved = resolve(&callee);
  assert_eq!(uses::ROOT | uses::CALLEE, resolved.frees[0].uses);
}

#[test]
fn paths_read_their_base_only() {
  assert_eq!(vec!["a: free 0"], reads("(a).b"));
  assert_eq!(vec!["a: free 0"], reads("(a)...b"));
}

#[test]
fn every_name_is_read_where_it_is_written() {
  let ast = AstNode::Add(Box::new(AstNode::Name(Name::from("?"))), Box::new(AstNode::Name(Name::from("item"))));
  assert_eq!(vec!["?: free 0", "item: free 1"], reads_of(&resolve(&ast)));
}

#[test]
fn shapes_that_are_evaluated_as_errors_read_nothing() {
  let ast = AstNode::Path(Box::new(AstNode::Name(Name::from("a"))), Box::new(AstNode::Null));
  assert!(reads_of(&resolve(&ast)).is_empty());
  let ast = AstNode::Every(Box::new(AstNode::Name(Name::from("a"))), Box::new(AstNode::Null));
  assert!(reads_of(&resolve(&ast)).is_empty());
  // a segment that is no segment is evaluated
  let ast = AstNode::QualifiedName(vec![AstNode::Name(Name::from("x"))]);
  assert_eq!(vec!["x: free 0"], reads_of(&resolve(&ast)));
}

#[test]
fn the_report_lists_free_names_in_order_of_first_read() {
  assert_eq!(
    vec!["x root+filter", "abs root+callee", "y root", "count root+callee", "z root+filter"],
    frees("if x > 1 then abs(y) else count([1][x > z])")
  );
}

#[test]
fn iteration_variables_named_twice_are_reported() {
  let ast = parse("for x in [1], x in [2] return x");
  let resolved = resolve(&ast);
  let Info::Iteration { duplicate, .. } = resolved.nodes[resolved.root() as usize].info else {
    panic!()
  };
  assert_ne!(NONE, duplicate);
  assert_eq!("x", resolved.names[duplicate as usize].to_string());
}

#[test]
fn the_dump_shows_nodes_and_tables() {
  let ast = parse("{f: function(x) x + y, y: 10, r: f(2)}.r");
  let expected = "\
Path: entry r @-
  Context: layout 0, stop None @-
    ContextEntry @0
      ContextEntryKey @0
      Numeric @0
    ContextEntry @1
      ContextEntryKey @1
      FunctionDefinition: proto 0 @1
        FormalParameters @1
          FormalParameter @1
            ParameterName @1
            FeelType @1
        FunctionBody @1
          Add @3
            Name x: slot 0 @3
            Name y: free 0 @3
    ContextEntry @4
      ContextEntryKey @4
      FunctionInvocation @4
        Name f: slot 1 @4
        Numeric @4
proto 0: params [0], binders 1, region 3, closure [y]
  Add @3
    Name x: slot 0 @3
    Name y: free 0 @3
root binders 3
layout 0 Context: y=slot 0 f=slot 1 r=slot 2
layout 1 Params: x=slot 0
region 0: layout 0 visible 0 in -
region 1: layout 0 visible 1 in -
region 2: closure in -
region 3: layout 1 visible 1 in 2
region 4: layout 0 visible 2 in -
free 0: y uses 0010
";
  assert_eq!(expected, resolve(&ast).dump());
}
