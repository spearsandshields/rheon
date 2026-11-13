//! Compile-time checks that the value types can be moved to and shared between threads
//! (`Send + Sync`), and that a scope can be moved to another thread (`Send`; its stack is a
//! `RefCell`, so a scope is not `Sync`). Function bodies are `Send + Sync` because an
//! [crate::Evaluator] is.

use crate::closure::Closure;
use crate::context::FeelContext;
use crate::values::{FeelString, Value};
use crate::{FeelNumber, FeelScope, FeelType, FunctionBody, FunctionDefinition, Name, QualifiedName};

fn send_sync<T: Send + Sync>() {}

fn send<T: Send>() {}

#[test]
fn value_types_are_send_and_sync() {
  send_sync::<Value>();
  send_sync::<FeelString>();
  send_sync::<FeelContext>();
  send_sync::<FeelType>();
  send_sync::<FeelNumber>();
  send_sync::<Name>();
  send_sync::<QualifiedName>();
  send_sync::<Closure>();
  send_sync::<FunctionBody>();
  send_sync::<FunctionDefinition>();
  send::<FeelScope>();
}
