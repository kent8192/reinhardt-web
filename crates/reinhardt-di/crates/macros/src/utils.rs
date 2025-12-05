//! Common utilities for DI macros

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, MetaNameValue, Result, Token, parse::Parse, punctuated::Punctuated};

/// Dependency scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scope {
	#[default]
	Singleton,
	Request,
	Transient,
}

impl Scope {
	/// Convert to TokenStream for code generation
	pub fn into_tokens(self) -> TokenStream {
		match self {
			Scope::Singleton => quote! { ::reinhardt_di::DependencyScope::Singleton },
			Scope::Request => quote! { ::reinhardt_di::DependencyScope::Request },
			Scope::Transient => quote! { ::reinhardt_di::DependencyScope::Transient },
		}
	}
}

/// Check if an attribute is #[inject]
pub fn is_inject_attr(attr: &Attribute) -> bool {
	attr.path().is_ident("inject")
}

/// Macro arguments structure
pub struct MacroArgs {
	pub scope: Option<Scope>,
}

impl Parse for MacroArgs {
	fn parse(input: syn::parse::ParseStream) -> Result<Self> {
		let mut scope = None;

		// Parse key-value pairs separated by commas
		let parsed = Punctuated::<MetaNameValue, Token![,]>::parse_terminated(input)?;

		for pair in parsed {
			if pair.path.is_ident("scope") {
				if let syn::Expr::Lit(syn::ExprLit {
					lit: syn::Lit::Str(lit_str),
					..
				}) = &pair.value
				{
					let value = lit_str.value();
					scope = Some(match value.as_str() {
						"singleton" => Scope::Singleton,
						"request" => Scope::Request,
						"transient" => Scope::Transient,
						_ => {
							return Err(syn::Error::new_spanned(
								lit_str,
								"Invalid scope. Expected 'singleton', 'request', or 'transient'",
							));
						}
					});
				} else {
					return Err(syn::Error::new_spanned(
						&pair.value,
						"Expected string literal for scope",
					));
				}
			}
		}

		Ok(MacroArgs { scope })
	}
}

/// Extract scope from macro arguments
pub fn extract_scope_from_args(args: TokenStream) -> Result<Scope> {
	if args.is_empty() {
		return Ok(Scope::default());
	}

	let parsed: MacroArgs = syn::parse2(args)?;
	Ok(parsed.scope.unwrap_or_default())
}
