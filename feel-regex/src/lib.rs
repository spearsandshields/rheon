#![doc = include_str!("../docs/README.md")]
#![deny(rustdoc::broken_intra_doc_links)]

mod errors;
mod flags;
mod impl_regex;
mod replacement;
mod unicode_blocks;
mod utils;
mod xsd;

pub use errors::{Limit, RegexError};
pub use impl_regex::{Patterns, is_match, is_match_with, replace, replace_with, split, split_with};
