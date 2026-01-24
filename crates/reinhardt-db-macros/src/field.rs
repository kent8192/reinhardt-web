//! Implementation of the `#[field(...)]` attribute macro.

use proc_macro::TokenStream;

pub(crate) mod attr_parser;

use attr_parser::FieldAttrs;

/// Implementation of the `#[field(...)]` attribute macro.
///
/// Parses field attributes and validates them.
/// The attributes are preserved for later processing by the `#[document(...)]` macro.
pub(crate) fn field_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
	// Parse and validate attributes
	let _attrs = syn::parse_macro_input!(attr as FieldAttrs);

	// For now, we just validate the attributes and pass through the field unchanged.
	// The #[document(...)] macro will extract and process these attributes.
	item
}
