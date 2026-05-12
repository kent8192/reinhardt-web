//! Path pattern parsing, matching, and radix-tree routing.
//!
//! This module is split by responsibility into the following submodules:
//!
//! - [`validation`]: shared length/segment limits and parameter validators
//! - [`path_pattern`]: [`PathPattern`] — the parsed, reversible URL pattern
//! - [`matcher`]: [`PathMatcher`] / [`MatchingMode`] — pattern dispatch
//! - [`radix`]: [`RadixRouter`] / [`RadixRouterError`] — radix-tree routing
//!
//! The top-level re-exports below preserve the public API surface that was
//! available when this module was a single file.

mod matcher;
mod path_pattern;
mod radix;
mod validation;

#[cfg(test)]
mod tests;

pub use matcher::{MatchingMode, PathMatcher};
pub use path_pattern::PathPattern;
pub use radix::{RadixRouter, RadixRouterError};
pub(crate) use validation::validate_reverse_param;
