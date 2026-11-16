//! The frozen reference front end of the differential tests ([super::differential] and
//! [super::names_index]): a copy of a lexer and parser that read the same tokens and build the same
//! ASTs as the production front end, except for the bug fixes the differential tests register, by
//! simpler means. The reference lexer scans the name chain again for every name token and looks
//! names up in its own parsing scope; in its flattened-key mode (`Lexer::set_oracle`,
//! `Parser::with_oracle_lexer`) it searches the set of all flattened keys of the scope, built for
//! every name token.
//!
//! The files are copies of `lexer.rs`, `parser.rs`, `lalr.rs`, `scope.rs`, `index.rs` and
//! `context.rs`, frozen except for the module paths (`super::` instead of `crate::`), the token
//! log of the parser and lexer (`token_start`, `with_token_log`, marked `oracle:`), and
//! `list_item_context_type` in `index.rs`, which takes the item type of a list from
//! `Value::type_of`.
//!
//! One change of behaviour is written into the reference lexer as well (marked `oracle:`): a name
//! that is not in scope ends before a keyword that can follow an operand and before a symbol that
//! is an operator. It changes the tokens
//! after the name in most texts with such a name, so as a registered difference it would leave the
//! rest of these texts uncompared. The tests of `super::expr::name` state the behaviour itself.
//!
//! The reference shares code with the production front end, which the differential tests therefore
//! do not compare and which its own unit tests cover: from rheon-feel, `Name` and the joining of
//! name parts, the built-in property and function names and `Value::type_of`; from this crate,
//! `crate::ast` and `crate::errors`.
#![allow(dead_code)]

pub mod context;
pub mod index;
pub mod lalr;
pub mod lexer;
pub mod parser;
pub mod scope;
