//! Common utilities for injectable macros
//!
//! This module contains shared logic for both function-based (#[injectable] on functions)
//! and struct-based (#[injectable] on structs) dependency injection.

use syn::{Expr, Token, punctuated::Punctuated};

/// Scope for dependency injection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InjectionScope {
	/// Request scope - dependencies are created per request (default)
	#[default]
	Request,
	/// Singleton scope - dependencies are created once and shared
	Singleton,
}

/// Parsed options from #[inject(...)] attribute
#[derive(Debug, Clone)]
pub struct InjectOptions {
	/// Whether to use caching for dependency resolution
	pub use_cache: bool,
	/// The scope for dependency injection
	pub scope: InjectionScope,
}

impl Default for InjectOptions {
	fn default() -> Self {
		Self {
			use_cache: true,
			scope: InjectionScope::Request,
		}
	}
}

/// Default value specification for #[no_inject] fields
#[derive(Debug, Clone)]
pub enum DefaultValue {
	/// Use Default::default()
	DefaultTrait,
	/// Use a specific expression
	Expression(Expr),
	/// No default value specified - field must be Option<T>
	None,
}

/// Parsed options from #[no_inject(...)] attribute
#[derive(Debug, Clone)]
pub struct NoInjectOptions {
	pub default: DefaultValue,
}

/// Check if an attribute is #[inject]
pub fn is_inject_attr(attr: &syn::Attribute) -> bool {
	attr.path().is_ident("inject")
}

/// Check if an attribute is #[no_inject]
pub fn is_no_inject_attr(attr: &syn::Attribute) -> bool {
	attr.path().is_ident("no_inject")
}

/// Parse #[inject] or #[inject(cache = false, scope = Singleton)] attributes
///
/// Returns `InjectOptions` with parsed settings. If no #[inject] attribute is found,
/// returns default options.
pub fn parse_inject_options(attrs: &[syn::Attribute]) -> InjectOptions {
	let mut options = InjectOptions::default();

	for attr in attrs {
		if !is_inject_attr(attr) {
			continue;
		}

		// Try to parse as Meta::List: #[inject(cache = false, scope = Singleton)]
		if let syn::Meta::List(meta_list) = &attr.meta
			&& let Ok(nested) =
				meta_list.parse_args_with(Punctuated::<syn::Meta, Token![,]>::parse_terminated)
		{
			for meta in nested {
				if let syn::Meta::NameValue(nv) = meta {
					if nv.path.is_ident("cache") {
						if let syn::Expr::Lit(syn::ExprLit {
							lit: syn::Lit::Bool(lit_bool),
							..
						}) = &nv.value
						{
							options.use_cache = lit_bool.value;
						}
					} else if nv.path.is_ident("scope")
						&& let syn::Expr::Path(path_expr) = &nv.value
					{
						if path_expr.path.is_ident("Singleton") {
							options.scope = InjectionScope::Singleton;
						} else if path_expr.path.is_ident("Request") {
							options.scope = InjectionScope::Request;
						}
					}
				}
			}
		}
	}

	options
}

/// Parse #[no_inject] or #[no_inject(default = ...)] attributes
///
/// Returns `Some(NoInjectOptions)` if `#[no_inject]` attribute is found, `None` otherwise.
pub fn parse_no_inject_options(attrs: &[syn::Attribute]) -> Option<NoInjectOptions> {
	for attr in attrs {
		if !is_no_inject_attr(attr) {
			continue;
		}

		// #[no_inject] without arguments -> None default
		if matches!(&attr.meta, syn::Meta::Path(_)) {
			return Some(NoInjectOptions {
				default: DefaultValue::None,
			});
		}

		// Try to parse as Meta::List: #[no_inject(default = ...)]
		if let syn::Meta::List(meta_list) = &attr.meta
			&& let Ok(nested) =
				meta_list.parse_args_with(Punctuated::<syn::Meta, Token![,]>::parse_terminated)
		{
			for meta in nested {
				if let syn::Meta::NameValue(nv) = meta
					&& nv.path.is_ident("default")
				{
					// Check if the value is the special "Default" keyword
					if let Expr::Path(path_expr) = &nv.value
						&& path_expr.path.is_ident("Default")
					{
						return Some(NoInjectOptions {
							default: DefaultValue::DefaultTrait,
						});
					} else {
						// Any other expression
						return Some(NoInjectOptions {
							default: DefaultValue::Expression(nv.value.clone()),
						});
					}
				}
			}
		}

		// If we found #[no_inject(...)] but couldn't parse it, return None default
		return Some(NoInjectOptions {
			default: DefaultValue::None,
		});
	}

	None
}
