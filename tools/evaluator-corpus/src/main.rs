//! Reads JSON lines {"src","ctx","expr"} on stdin; evaluates each with the backend
//! selected at compile time; writes JSON lines {"src","display","debug"} to stdout.
use dsntk_feel::FeelScope;
use dsntk_feel::values::Value;
use std::io::{BufRead, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};

fn scope_from(ctx: &str) -> Result<FeelScope, String> {
  let scope = FeelScope::default();
  if ctx.trim().is_empty() {
    return Ok(scope);
  }
  let node = dsntk_feel_parser::parse_context(&scope, ctx, false).map_err(|e| format!("ctx parse error: {e}"))?;
  match dsntk_feel_evaluator::evaluate(&scope, &node) {
    Value::Context(c) => Ok(c.into()),
    other => Err(format!("ctx not a context: {other}")),
  }
}

fn eval(ctx: &str, expr: &str) -> (String, String) {
  let scope = match scope_from(ctx) {
    Ok(s) => s,
    Err(e) => return (e.clone(), e),
  };
  match dsntk_feel_parser::parse_textual_expression(&scope, expr, false) {
    Ok(node) => {
      let v = dsntk_feel_evaluator::evaluate(&scope, &node);
      let disp = catch_unwind(AssertUnwindSafe(|| v.to_string())).unwrap_or_else(|_| "<PANIC in Display>".to_string());
      let dbg = catch_unwind(AssertUnwindSafe(|| format!("{v:?}"))).unwrap_or_else(|_| "<PANIC in Debug>".to_string());
      (disp, dbg)
    }
    Err(e) => (format!("parse error: {e}"), format!("parse error: {e}")),
  }
}

fn main() {
  std::panic::set_hook(Box::new(|_| {}));
  let backend = if cfg!(feature = "dashu") { "dashu" } else { "intel" };
  let stdin = std::io::stdin();
  let mut out = std::io::BufWriter::new(std::io::stdout());
  for line in stdin.lock().lines() {
    let line = line.unwrap();
    if line.trim().is_empty() {
      continue;
    }
    let j: serde_json::Value = serde_json::from_str(&line).unwrap();
    let ctx = j["ctx"].as_str().unwrap_or("");
    let expr = j["expr"].as_str().unwrap();
    let t = std::time::Instant::now();
    let (display, debug) = match catch_unwind(AssertUnwindSafe(|| eval(ctx, expr))) {
      Ok(r) => r,
      Err(_) => ("<PANIC in eval>".into(), "<PANIC in eval>".into()),
    };
    let ns = t.elapsed().as_nanos() as u64;
    let o = serde_json::json!({"src": j["src"], "ctx": ctx, "expr": expr, "display": display, "debug": debug, "ns": ns, "backend": backend});
    writeln!(out, "{o}").unwrap();
  }
}
