use super::*;

from_examples!(DMN_3_1152);

static_context!(CTX, "{}");

fn eq(invocable: &str, ctx: &FeelContext, expected: &str) {
  assert_decision(&MODEL_EVALUATOR, &MODEL_NAMESPACE, &MODEL_NAME, invocable, ctx, expected);
}

#[test]
fn _0001() {
  eq("decision001", &CTX, "[2, 4, 6, 8, 10]");
}

#[test]
fn _0002() {
  eq("decision002", &CTX, "[]");
}

#[test]
fn _0003() {
  // DMN 1.6 (DMN16-94): a single value in the boxed for's "in" row is a singleton list; the official
  // TCK 1152 still expects the DMN 1.5 result (no coercion), see tck-addendum/MANIFEST.md
  eq("decision003", &CTX, r#"["not a list"]"#);
}

#[test]
fn _0004() {
  // DMN 1.6 (DMN16-94): a single value in the boxed for's "in" row is a singleton list; the official
  // TCK 1152 still expects the DMN 1.5 result (no coercion), see tck-addendum/MANIFEST.md
  eq("decision004", &CTX, r#"[2]"#);
}
