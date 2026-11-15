use super::*;

#[test]
fn _0001() {
  let node = AstNode::Path(Box::new(AstNode::Name("a".into())), Box::new(AstNode::Name("b".into())));
  let scope = te_scope(r#"{ a: { b: 10 } }"#);
  assert_eq!(r#"10"#, crate::tests::eval(&scope, &node).to_string());
}

#[test]
fn _0002() {
  let node = AstNode::Path(Box::new(AstNode::Name("a".into())), Box::new(AstNode::Name("b".into())));
  let scope = te_scope(r#"{ a: { c: 10 } }"#);
  assert_eq!(r#"null(build_path: no entry b in context: {c: 10})"#, crate::tests::eval(&scope, &node).to_string());
}

#[test]
fn _0003() {
  let node = AstNode::Path(Box::new(AstNode::Name("a".into())), Box::new(AstNode::Name("b".into())));
  let scope = te_scope(r#"{ a: [{ b: 10 }, { b: 11 }, { b: 12 }] }"#);
  assert_eq!(r#"[10, 11, 12]"#, crate::tests::eval(&scope, &node).to_string());
}

#[test]
fn _0004() {
  let node = AstNode::Path(Box::new(AstNode::Name("a".into())), Box::new(AstNode::Name("b".into())));
  let scope = te_scope(r#"{ a: [{ b: 10 }, { b: 11 }, 12 ] }"#);
  assert_eq!("[10, 11, null(unexpected type: number, for property: b)]", crate::tests::eval(&scope, &node).to_string());
}

#[test]
fn _0005() {
  let node = AstNode::Path(
    Box::new(AstNode::Path(Box::new(AstNode::Name("a".into())), Box::new(AstNode::Name("b".into())))),
    Box::new(AstNode::Name("days".into())),
  );
  let scope = te_scope(r#"{ a: [{ b: @"P1DT5H" }, { b: @"P2DT6H" }, { b: @"P3DT7H" }] }"#);
  assert_eq!(r#"[1, 2, 3]"#, crate::tests::eval(&scope, &node).to_string());
}

#[test]
fn _0006() {
  let node = AstNode::Path(
    Box::new(AstNode::Name("a".into())),
    Box::new(AstNode::Path(Box::new(AstNode::Boolean(false)), Box::new(AstNode::Name("days".into())))),
  );
  let scope = te_scope(r#"{ a: [{ b: @"P1DT5H" }, { b: @"P2DT6H" }, { b: @"P3DT7H" }] }"#);
  assert_eq!(
    r#"null(expected name node, actual node is: Path(Boolean(false), Name(Name("days"))))"#,
    crate::tests::eval(&scope, &node).to_string()
  );
}

#[test]
fn _0007() {
  let node = AstNode::Path(Box::new(AstNode::Name("a".into())), Box::new(AstNode::Boolean(false)));
  let scope = te_scope(r#"{ a: [{ b: @"P1DT5H" }, { b: @"P2DT6H" }, { b: @"P3DT7H" }] }"#);
  assert_eq!("null(expected name node, actual node is: Boolean(false))", crate::tests::eval(&scope, &node).to_string());
}

/// DMN 1.6 §10.3.2.6: "Selecting a value by key from context m in the semantic domain is denoted
/// as m.key1 or get value(m, "key1")", and Table 82 defines `get value({key1 : "value 1"},
/// "unexistent-key") = null`: a missing entry is a null the specification defines, no error. The
/// null keeps the reason as a note.
#[test]
fn a_missing_context_entry_is_a_null_with_a_note() {
  te_errors("{a: 1}.b", "null(build_path: no entry b in context: {a: 1})", &[]);
  // a null operand of an addition is an error of the addition's own
  let reason = "[addition] incompatible types: null(build_path: no entry c in context: {b: 1}) + 1";
  te_errors("({a: {b: 1}}.a.c) + 1", &format!("null({reason})"), &[reason]);
  // an error beside it is still counted
  te_errors(
    "if {a: 1}.b = null then 1 / 0 else 0",
    "null([division] division by zero)",
    &["[division] division by zero"],
  );
}
