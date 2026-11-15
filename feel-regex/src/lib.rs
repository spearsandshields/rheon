#![doc = include_str!("../docs/README.md")]
#![deny(rustdoc::broken_intra_doc_links)]

mod errors;
mod flags;
mod impl_regex;
mod unicode_blocks;
mod utils;

pub use errors::{Limit, RegexError};
pub use impl_regex::{is_match, replace, split};
