//! # Value kernels
//!
//! The functions that compute FEEL values from values: what an expression gives once its operands
//! are evaluated. They read no scope and evaluate nothing; the evaluator calls them in the order
//! the expression evaluates its parts, so every error, warning and limit they record is recorded
//! in that order.

mod arithmetic;
mod compare;
mod functions;
mod literals;
mod membership;
mod paths;
mod temporal_value;
mod types;

pub(crate) use arithmetic::{Arithmetic, finite};
pub(crate) use compare::{between, compare_ge, compare_gt, compare_le, compare_lt, equal, not_equal};
pub(crate) use functions::{external_body, function_definition};
pub(crate) use literals::{at_literal, context_entry, context_literal_entry, err_duplicated_context_entry_key, list_of, number_literal, range_of};
pub(crate) use membership::in_operator;
pub(crate) use paths::{descendants, free_name, free_qualified_name, path};
pub(crate) use types::{add_context_type_entry, add_formal_parameter, context_type_entry, formal_parameter, function_type, instance_of, list_type, range_type};
