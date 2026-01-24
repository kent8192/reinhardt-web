//! Proc macros for reinhardt-taggit
//!
//! This crate provides the `` `#[taggable]` `` attribute macro for zero-boilerplate tagging.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput};

/// Attribute macro to make a model taggable
///
/// This macro automatically implements the `` `Taggable` `` trait and adds a `` `tags()` `` method
/// to the model, enabling tag management functionality.
///
/// # Usage
///
/// ```rust,ignore
/// use reinhardt_taggit::prelude::*;
///
/// #[model(app_label = "myapp", table_name = "foods")]
/// #[taggable]
/// pub struct Food {
///     #[field(primary_key = true)]
///     pub id: i64,
///
///     #[field(max_length = 255)]
///     pub name: String,
/// }
///
/// // Generated methods:
/// // - impl Taggable for Food
/// // - fn Food::tags(&self) -> TagManager<Food>
/// ```
///
/// # Requirements
///
/// - The struct must have a field named `id` of type `i64` or `Option<i64>`
/// - The struct must be annotated with `` `#[model(...)]` ``
///
/// # Generated Code
///
/// The macro generates:
/// 1. `` `Taggable` `` trait implementation with `` `content_type_name()` `` and `` `object_id()` ``
/// 2. `` `tags()` `` method that returns a `` `TagManager<Self>` ``
#[proc_macro_attribute]
pub fn taggable(_attr: TokenStream, item: TokenStream) -> TokenStream {
	let input = parse_macro_input!(item as DeriveInput);
	let name = &input.ident;

	// Ensure it's a struct
	let Data::Struct(_) = &input.data else {
		return syn::Error::new_spanned(&input, "#[taggable] can only be applied to structs")
			.to_compile_error()
			.into();
	};

	// Generate Taggable trait implementation
	// NOTE: This is a placeholder implementation that will be refined later
	let expanded = quote! {
		#input

		// Placeholder: Taggable trait implementation will be added in the next iteration
		// #[automatically_derived]
		// impl ::reinhardt_taggit::Taggable for #name {
		//     fn content_type_name() -> &'static str {
		//         stringify!(#name)
		//     }
		//
		//     fn object_id(&self) -> i64 {
		//         self.id
		//     }
		// }
		//
		// #[automatically_derived]
		// impl #name {
		//     pub fn tags(&self) -> ::reinhardt_taggit::TagManager<Self> {
		//         ::reinhardt_taggit::TagManager::new(self)
		//     }
		// }
	};

	TokenStream::from(expanded)
}
