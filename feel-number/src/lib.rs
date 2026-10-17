#![doc = include_str!("../docs/README.md")]
#![deny(rustdoc::broken_intra_doc_links)]

#[macro_use]
extern crate dsntk_macros;

#[cfg(all(feature = "intel", feature = "dashu"))]
compile_error!("features `intel` and `dashu` are mutually exclusive: enable exactly one of them (use `default-features = false` to drop the default `dashu`)");

#[cfg(not(any(feature = "intel", feature = "dashu")))]
compile_error!("exactly one of the features `intel` or `dashu` must be enabled");

mod errors;

#[cfg(feature = "intel")]
mod dfp_number;

#[cfg(all(feature = "dashu", not(feature = "intel")))]
mod backend_dashu;
#[cfg(all(feature = "dashu", not(feature = "intel")))]
mod decimal128;

#[cfg(feature = "intel")]
pub use dfp_number::FeelNumber;

#[cfg(all(feature = "dashu", not(feature = "intel")))]
pub use decimal128::FeelNumber;
