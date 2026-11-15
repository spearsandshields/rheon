use super::*;

from_examples!(DMN_3_0041);

#[test]
fn _0001() {
  let ctx = context(r#"{Principal: 600000,Term: 360}"#);
  assert_decision(
    &MODEL_EVALUATOR,
    &MODEL_NAMESPACE,
    &MODEL_NAME,
    "Boxed Context",
    &ctx,
    r#"2778.693549432766768088520383236299"#,
  );
}

#[test]
fn _0002() {
  let ctx = context(r#"{Principal: 30000,Term: 60}"#);
  assert_decision(
    &MODEL_EVALUATOR,
    &MODEL_NAMESPACE,
    &MODEL_NAME,
    "Boxed Context",
    &ctx,
    r#"549.117549836400293492700014885945"#,
  );
}

#[test]
fn _0003() {
  let ctx = context(r#"{Principal: 600000,Term: 365}"#);
  assert_decision(
    &MODEL_EVALUATOR,
    &MODEL_NAMESPACE,
    &MODEL_NAME,
    "Boxed Context",
    &ctx,
    r#"2758.116099896591400871418893289041"#,
  );
}
