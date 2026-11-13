//! Compile-time checks that the parser's public types can be moved to another thread: the AST
//! and the closure builder can also be shared between threads (`Send + Sync`); a parsing scope
//! keeps its state in `Cell`s and `RefCell`s, so it can be moved (`Send`) but is not `Sync`.

use crate::{AstNode, ClosureBuilder, ParsingScope};

fn send_sync<T: Send + Sync>() {}

fn send<T: Send>() {}

#[test]
fn public_types_are_send() {
  send_sync::<AstNode>();
  send_sync::<ClosureBuilder>();
  send::<ParsingScope<'static>>();
}
