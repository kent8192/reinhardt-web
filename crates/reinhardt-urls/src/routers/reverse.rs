//! URL reverse resolution — inspired by Django's `django.urls.reverse()`.
//!
//! This module provides string-based (runtime) URL reversal, organized into
//! the following submodules:
//!
//! - `runtime`: free-function reversers
//! - `reverser`: the [`UrlReverser`] registry and top-level [`reverse()`] fn
//!
//! The top-level re-exports below preserve the public API surface that was
//! available when this module was a single file.

mod reverser;
mod runtime;

#[cfg(test)]
mod tests;

pub use reverser::{UrlReverser, reverse};
pub use runtime::{
	ReverseError, ReverseResult, extract_param_names, try_reverse_single_pass,
	try_reverse_with_aho_corasick,
};
