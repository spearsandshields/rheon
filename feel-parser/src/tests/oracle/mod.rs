//! The prototype's lexer and parser (perf prototype, 19664f8 plus steps 1-8), frozen as the
//! reference of the differential tests (see [super::differential]). Kept until the next release.
//!
//! The files are verbatim copies of `lexer.rs`, `parser.rs`, `lalr.rs`, `scope.rs`, `index.rs` and
//! `context.rs`, except for the module paths (`super::` instead of `crate::`) and the test-only
//! token log of the parser and lexer (`token_start`, `with_token_log`, marked `oracle:`), and for
//! `list_item_context_type` in `index.rs`, which takes the item type of a list from
//! `Value::type_of`, the only list typing rule rheon-feel publishes.
#![allow(dead_code)]

pub mod context;
pub mod index;
pub mod lalr;
pub mod lexer;
pub mod parser;
pub mod scope;
