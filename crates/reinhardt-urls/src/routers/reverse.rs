//! URL reverse resolution — inspired by Django's `django.urls.reverse()`.
//!
//! This module provides both string-based (runtime) and type-safe
//! (compile-time) URL reversal mechanisms, organized into the following
//! submodules:
//!
//! - `runtime`: free-function reversers used by both paths
//! - `reverser`: the [`UrlReverser`] registry and top-level [`reverse()`] fn
//! - `typed`: [`UrlPattern`] / [`UrlPatternWithParams`] traits and helpers
//!
//! The top-level re-exports below preserve the public API surface that was
//! available when this module was a single file.

mod reverser;
mod runtime;
mod typed;

#[cfg(test)]
mod tests;

pub use reverser::{UrlReverser, reverse};
pub use runtime::{
	ReverseError, ReverseResult, extract_param_names, try_reverse_single_pass,
	try_reverse_with_aho_corasick,
};
#[allow(
	deprecated,
	reason = "re-export deprecated panicking helpers during the deprecation cycle"
)]
pub use runtime::{reverse_single_pass, reverse_with_aho_corasick};
pub use typed::{
	UrlParams, UrlPattern, UrlPatternWithParams, reverse_typed, reverse_typed_with_params,
};
