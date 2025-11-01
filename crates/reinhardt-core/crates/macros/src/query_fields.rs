//! Procedural macro for generating type-safe field lookups
//!
//! This macro automatically generates field accessor methods for models,
//! enabling compile-time validated field lookups.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Type};
/// Implementation of the `QueryFields` derive macro
///
/// This function is used internally by the `#[derive(QueryFields)]` macro.
/// Users should not call this function directly.
pub fn derive_query_fields_impl(input: DeriveInput) -> TokenStream {
	let struct_name = &input.ident;

	// Extract struct fields
	let fields = match &input.data {
		Data::Struct(data) => match &data.fields {
			Fields::Named(fields) => &fields.named,
			_ => {
				return quote! {
					compile_error!("QueryFields can only be derived for structs with named fields");
				};
			}
		},
		_ => {
			return quote! {
				compile_error!("QueryFields can only be derived for structs");
			};
		}
	};

	// Generate field accessor methods
	let field_methods: Vec<TokenStream> = fields
		.iter()
		.map(|field| {
			let field_name = field.ident.as_ref().unwrap();
			let field_name_str = field_name.to_string();
			let field_type = &field.ty;

			// Map Rust types to field lookup types
			let lookup_type = map_type_to_lookup_type(field_type);

			quote! {
				#[doc = concat!("Field accessor for `", #field_name_str, "`")]
				pub fn #field_name() -> ::reinhardt_orm::query_fields::Field<#struct_name, #lookup_type> {
					::reinhardt_orm::query_fields::Field::new(vec![#field_name_str])
				}
			}
		})
		.collect();

	// Generate the impl block
	quote! {
		impl #struct_name {
			#(#field_methods)*
		}
	}
}

/// Map Rust types to field lookup types
fn map_type_to_lookup_type(ty: &Type) -> TokenStream {
	// Handle the type - this is a simplified version
	// In a real implementation, you'd want more sophisticated type mapping
	match ty {
		Type::Path(type_path) => {
			let last_segment = type_path.path.segments.last().unwrap();
			let type_ident = &last_segment.ident;
			let type_name = type_ident.to_string();

			match type_name.as_str() {
				"String" => quote! { String },
				"i32" => quote! { i32 },
				"i64" => quote! { i64 },
				"f32" => quote! { f32 },
				"f64" => quote! { f64 },
				"bool" => quote! { bool },
				"DateTime" => quote! { ::reinhardt_orm::query_fields::DateTime },
				"Date" => quote! { ::reinhardt_orm::query_fields::Date },
				"Option" => {
					// Handle Option<T>
					if let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments
						&& let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
							let inner_lookup = map_type_to_lookup_type(inner_type);
							return quote! { Option<#inner_lookup> };
						}
					quote! { #ty }
				}
				_ => {
					// For unknown types, use as-is
					quote! { #ty }
				}
			}
		}
		_ => quote! { #ty },
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use syn::parse_quote;

	#[test]
	fn test_derive_query_fields() {
		let input: DeriveInput = parse_quote! {
			struct User {
				id: i64,
				email: String,
				age: i32,
				created_at: DateTime,
			}
		};

		let output = derive_query_fields_impl(input);
		let output_str = output.to_string();

		// Verify that field accessor methods are generated
		assert!(output_str.contains("pub fn id"));
		assert!(output_str.contains("pub fn email"));
		assert!(output_str.contains("pub fn age"));
		assert!(output_str.contains("pub fn created_at"));
	}
}
