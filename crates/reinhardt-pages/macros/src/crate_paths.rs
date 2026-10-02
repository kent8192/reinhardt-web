//! Helper functions for dynamic crate path resolution using proc_macro_crate
//!
//! These functions resolve crate paths at compile time, supporting various
//! dependency configurations (direct, via facade crate, etc.).

use proc_macro2::TokenStream;
use quote::quote;

/// Information about how to reference the reinhardt_pages crate.
pub(crate) struct CratePathInfo {
	/// Whether conditional compilation is needed (both reinhardt and reinhardt-pages are dependencies)
	pub needs_conditional: bool,
	/// The use statement(s) to emit (may include `#[cfg(...)]` attributes)
	pub use_statement: TokenStream,
	/// The identifier to use when referencing the crate (e.g., `__reinhardt_pages`)
	pub ident: TokenStream,
}

/// Resolves the path to the reinhardt_pages crate dynamically.
///
/// Since proc macros cannot detect the target architecture at runtime (they run on the host),
/// this function generates conditional code using `#[cfg(all(target_family = "wasm", target_os = "unknown"))]` that the
/// Rust compiler will select at compile time.
///
/// # Strategy
///
/// 1. Internal crate usage (`Itself`): Use `::reinhardt_pages` absolute path (doc test compatible)
/// 2. Both `reinhardt` and `reinhardt-pages` are dependencies: Generate conditional code
///    - WASM: `use ::reinhardt_pages`
///    - Server: `use ::reinhardt::pages`
/// 3. Only `reinhardt-pages`: Use it directly
/// 4. Only `reinhardt`: Use `::reinhardt::pages`
/// 5. Fallback: Use `::reinhardt_pages`
pub(crate) fn get_reinhardt_pages_crate_info() -> CratePathInfo {
	let direct = named_dependency("reinhardt-pages", "reinhardt_pages");
	let facade = named_dependency("reinhardt", "reinhardt")
		.or_else(|| named_dependency("reinhardt-web", "reinhardt"));
	match (direct, facade) {
		(Some(direct), Some(facade)) => CratePathInfo {
			needs_conditional: true,
			use_statement: quote! {
				#[cfg(all(target_family = "wasm", target_os = "unknown"))]
				use #direct as __reinhardt_pages;
				#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
				use #facade::pages as __reinhardt_pages;
			},
			ident: quote!(__reinhardt_pages),
		},
		(direct, facade) => CratePathInfo {
			needs_conditional: false,
			use_statement: quote!(),
			ident: direct
				.or_else(|| facade.map(|facade| quote!(#facade::pages)))
				.unwrap_or_else(|| quote!(::reinhardt_pages)),
		},
	}
}

fn named_dependency(package: &str, internal_name: &str) -> Option<TokenStream> {
	use proc_macro_crate::{FoundCrate, crate_name};
	let name = match crate_name(package).ok()? {
		FoundCrate::Itself => internal_name.to_owned(),
		FoundCrate::Name(name) => name,
	};
	let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
	Some(quote!(::#ident))
}

/// Legacy function for backwards compatibility.
/// Use `get_reinhardt_pages_crate_info()` for new code that needs conditional compilation.
pub(crate) fn get_reinhardt_pages_crate() -> TokenStream {
	let info = get_reinhardt_pages_crate_info();
	if info.needs_conditional {
		// For legacy callers that can't handle conditional compilation,
		// prefer the server path (most common case for non-page! macro usage)
		let facade = named_dependency("reinhardt", "reinhardt")
			.or_else(|| named_dependency("reinhardt-web", "reinhardt"))
			.expect("conditional Pages resolution has a facade dependency");
		quote!(#facade::pages)
	} else {
		info.ident
	}
}

/// Resolves the path to the reinhardt_di crate dynamically.
///
/// Uses the same strategy order as [`get_reinhardt_pages_crate`] to avoid
/// conditional dependency resolution issues.
pub(crate) fn get_reinhardt_di_crate() -> TokenStream {
	use proc_macro_crate::{FoundCrate, crate_name};

	// Try via reinhardt crate first (prioritized to avoid conditional dependency issues)
	match crate_name("reinhardt") {
		Ok(FoundCrate::Itself) => return quote!(::reinhardt::reinhardt_di),
		Ok(FoundCrate::Name(name)) => {
			let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
			return quote!(::#ident::reinhardt_di);
		}
		Err(_) => {}
	}

	// Try via reinhardt-web (published package name)
	match crate_name("reinhardt-web") {
		Ok(FoundCrate::Itself) => return quote!(::reinhardt::reinhardt_di),
		Ok(FoundCrate::Name(name)) => {
			let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
			return quote!(::#ident::reinhardt_di);
		}
		Err(_) => {}
	}

	// Try direct crate (for internal usage within reinhardt-di crate)
	match crate_name("reinhardt-di") {
		Ok(FoundCrate::Itself) => return quote!(::reinhardt_di),
		Ok(FoundCrate::Name(name)) => {
			let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
			return quote!(::#ident);
		}
		Err(_) => {}
	}

	// Final fallback - use reinhardt facade crate (re-exported module)
	quote!(::reinhardt::reinhardt_di)
}

/// Resolves the path to the reinhardt_http crate dynamically.
///
/// Uses the same strategy order as [`get_reinhardt_pages_crate`] to avoid
/// conditional dependency resolution issues.
pub(crate) fn get_reinhardt_http_crate() -> TokenStream {
	// Native server-function handlers use the HTTP types already owned by Pages.
	// This also works when the consumer enables only the facade's pages feature.
	let pages = get_reinhardt_pages_crate();
	quote!(#pages::__private::reinhardt_http)
}

/// Resolves the path to the reinhardt_core crate dynamically.
pub(crate) fn get_reinhardt_core_crate() -> TokenStream {
	use proc_macro_crate::{FoundCrate, crate_name};

	// Try via reinhardt crate first
	match crate_name("reinhardt") {
		Ok(FoundCrate::Itself) => return quote!(::reinhardt::reinhardt_core),
		Ok(FoundCrate::Name(name)) => {
			let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
			return quote!(::#ident::reinhardt_core);
		}
		Err(_) => {}
	}

	// Try via reinhardt-web (published package name)
	match crate_name("reinhardt-web") {
		Ok(FoundCrate::Itself) => return quote!(::reinhardt::reinhardt_core),
		Ok(FoundCrate::Name(name)) => {
			let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
			return quote!(::#ident::reinhardt_core);
		}
		Err(_) => {}
	}

	// Try direct crate
	match crate_name("reinhardt-core") {
		Ok(FoundCrate::Itself) => return quote!(::reinhardt_core),
		Ok(FoundCrate::Name(name)) => {
			let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
			return quote!(::#ident);
		}
		Err(_) => {}
	}

	// Final fallback
	quote!(::reinhardt_core)
}
