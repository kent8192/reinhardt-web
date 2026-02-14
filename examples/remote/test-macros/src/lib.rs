//! Custom Test Macros
//!
//! Provides test macros with version verification using Cargo version specifiers.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Ident, ItemFn, LitStr, Token, parse::ParseStream, parse_macro_input};

/// Arguments for the `example_test` attribute macro.
///
/// Parses `version = "..."` syntax.
struct ExampleTestArgs {
	version: LitStr,
}

impl syn::parse::Parse for ExampleTestArgs {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let ident: Ident = input.parse()?;
		if ident != "version" {
			return Err(syn::Error::new(ident.span(), "expected `version`"));
		}
		let _: Token![=] = input.parse()?;
		let version: LitStr = input.parse()?;
		Ok(Self { version })
	}
}

/// Test macro using Cargo version specifiers
///
/// # Examples
///
/// ```rust,no_run
/// # use example_test_macros::example_test;
/// #[example_test(version = "0.1.0-alpha.1")]
/// fn test_exact_version() {
///     // Run only on reinhardt 0.1.0
/// }
///
/// #[example_test(version = "^0.1")]
/// fn test_caret_requirement() {
///     // Run only on reinhardt ^0.1 (0.1.x)
/// }
///
/// #[example_test(version = ">=0.1.0, <0.2.0")]
/// fn test_version_range() {
///     // Run only on reinhardt 0.1.x
/// }
///
/// #[example_test(version = "*")]
/// fn test_latest() {
///     // Run on latest version
/// }
/// ```
///
/// # Supported Version Specifiers
///
/// - `"0.1.0"` - Exact version
/// - `"^0.1"` - Caret requirement (0.1.x)
/// - `"~0.1.2"` - Tilde requirement (0.1.2 <= version < 0.2.0)
/// - `">=0.1, <0.2"` - Range specification
/// - `"*"` - Wildcard (latest)
#[proc_macro_attribute]
pub fn example_test(attr: TokenStream, item: TokenStream) -> TokenStream {
	// Extract version specifier from named argument syntax: version = "..."
	let args = parse_macro_input!(attr as ExampleTestArgs);
	let version_spec = args.version.value();
	let test_fn = parse_macro_input!(item as ItemFn);

	let fn_name = &test_fn.sig.ident;
	let fn_block = &test_fn.block;
	let fn_attrs = &test_fn.attrs;
	let fn_async = &test_fn.sig.asyncness;

	// Generate code differently based on async/sync
	let expanded = if fn_async.is_some() {
		quote! {
			#(#fn_attrs)*
			#[tokio::test]
			async fn #fn_name() {
				// Version check
				if !example_common::version::check_version(#version_spec) {
					eprintln!(
						"⏭️  Skipping test '{}': version mismatch",
						stringify!(#fn_name)
					);
					eprintln!(
						"   Required: {}, Actual: {}",
						#version_spec,
						example_common::version::get_reinhardt_version()
					);
					return; // Skip test
				}

				// crates.io availability check
				if !example_common::availability::is_reinhardt_available() {
					eprintln!(
						"⏭️  Skipping test '{}': reinhardt not available from crates.io",
						stringify!(#fn_name)
					);
					return; // Skip test
				}

				// Execute actual test
				#fn_block
			}
		}
	} else {
		quote! {
			#(#fn_attrs)*
			#[test]
			fn #fn_name() {
				// Version check
				if !example_common::version::check_version(#version_spec) {
					eprintln!(
						"⏭️  Skipping test '{}': version mismatch",
						stringify!(#fn_name)
					);
					eprintln!(
						"   Required: {}, Actual: {}",
						#version_spec,
						example_common::version::get_reinhardt_version()
					);
					return; // Skip test
				}

				// crates.io availability check
				if !example_common::availability::is_reinhardt_available() {
					eprintln!(
						"⏭️  Skipping test '{}': reinhardt not available from crates.io",
						stringify!(#fn_name)
					);
					return; // Skip test
				}

				// Execute actual test
				#fn_block
			}
		}
	};

	TokenStream::from(expanded)
}

#[cfg(test)]
mod tests {
	use semver::{Version, VersionReq};

	/// Test that alpha version specifications work correctly
	#[test]
	fn test_alpha_version_matching() {
		let alpha_version = Version::parse("0.1.0-alpha.1").unwrap();

		// Exact match should work
		let exact = VersionReq::parse("0.1.0-alpha.1").unwrap();
		assert!(
			exact.matches(&alpha_version),
			"Exact alpha version match should succeed"
		);

		// Range specification with alpha
		let alpha_range = VersionReq::parse(">=0.1.0-alpha.1, <0.1.0").unwrap();
		assert!(
			alpha_range.matches(&alpha_version),
			"Alpha range specification should match"
		);

		// Caret with alpha base
		let alpha_caret = VersionReq::parse("^0.1.0-alpha").unwrap();
		assert!(
			alpha_caret.matches(&alpha_version),
			"Caret with alpha base should match alpha.1"
		);

		// Minimum alpha version
		let alpha_min = VersionReq::parse(">=0.1.0-alpha.1").unwrap();
		assert!(
			alpha_min.matches(&alpha_version),
			"Minimum alpha version should match"
		);
	}

	/// Test that stable version specs do NOT match alpha versions
	#[test]
	fn test_stable_specs_do_not_match_alpha() {
		let alpha_version = Version::parse("0.1.0-alpha.1").unwrap();

		// Exact stable version should NOT match alpha
		let exact_stable = VersionReq::parse("0.1.0").unwrap();
		assert!(
			!exact_stable.matches(&alpha_version),
			"Exact stable version 0.1.0 should NOT match 0.1.0-alpha.1"
		);

		// Caret ^0.1 means >=0.1.0, <0.2.0 which does NOT include alpha
		let caret = VersionReq::parse("^0.1").unwrap();
		assert!(
			!caret.matches(&alpha_version),
			"Caret ^0.1 should NOT match 0.1.0-alpha.1 (alpha is before 0.1.0)"
		);

		// Range >=0.1.0 does NOT include alpha
		let range = VersionReq::parse(">=0.1.0").unwrap();
		assert!(
			!range.matches(&alpha_version),
			"Range >=0.1.0 should NOT match 0.1.0-alpha.1"
		);
	}

	/// Test version precedence according to SemVer 2.0.0
	#[test]
	fn test_version_precedence() {
		let alpha1 = Version::parse("0.1.0-alpha.1").unwrap();
		let alpha2 = Version::parse("0.1.0-alpha.2").unwrap();
		let beta = Version::parse("0.1.0-beta.1").unwrap();
		let stable = Version::parse("0.1.0").unwrap();

		// Verify precedence order
		assert!(alpha1 < alpha2, "alpha.1 < alpha.2");
		assert!(alpha2 < beta, "alpha.2 < beta.1");
		assert!(beta < stable, "beta.1 < 0.1.0");
		assert!(alpha1 < stable, "alpha.1 < 0.1.0");
	}
}
