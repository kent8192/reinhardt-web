//! Implementation of GrpcGraphQLConvert derive macro

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Field, Fields, Ident, LitStr, Result};

/// Field configuration parsed from attributes
#[derive(Debug, Default)]
struct FieldConfig {
	/// Renamed field name for GraphQL/Proto
	rename: Option<String>,
	/// Skip this field if condition is true
	skip_if: Option<String>,
	/// Proto field name (if different from GraphQL)
	proto_name: Option<String>,
}

impl FieldConfig {
	fn from_field(field: &Field) -> Result<Self> {
		let mut config = FieldConfig::default();

		for attr in &field.attrs {
			if attr.path().is_ident("graphql") {
				attr.parse_nested_meta(|meta| {
					if meta.path.is_ident("rename") {
						let value = meta.value()?;
						let name: LitStr = value.parse()?;
						config.rename = Some(name.value());
					} else if meta.path.is_ident("skip_if") {
						let value = meta.value()?;
						let condition: LitStr = value.parse()?;
						config.skip_if = Some(condition.value());
					}
					Ok(())
				})?;
			} else if attr.path().is_ident("proto") {
				attr.parse_nested_meta(|meta| {
					if meta.path.is_ident("name") {
						let value = meta.value()?;
						let name: LitStr = value.parse()?;
						config.proto_name = Some(name.value());
					}
					Ok(())
				})?;
			}
		}

		Ok(config)
	}
}

pub fn expand_derive(input: DeriveInput) -> Result<TokenStream> {
	let name = &input.ident;

	// Only structs are supported
	let fields = match &input.data {
		Data::Struct(data) => match &data.fields {
			Fields::Named(fields) => &fields.named,
			_ => {
				return Err(syn::Error::new_spanned(
					input,
					"GrpcGraphQLConvert only supports structs with named fields",
				));
			}
		},
		_ => {
			return Err(syn::Error::new_spanned(
				input,
				"GrpcGraphQLConvert only supports structs",
			));
		}
	};

	// Parse field configurations
	let field_configs: Vec<_> = fields
		.iter()
		.map(FieldConfig::from_field)
		.collect::<Result<_>>()?;

	// Get list of field names with their configs
	let field_data: Vec<_> = fields
		.iter()
		.zip(field_configs.iter())
		.filter_map(|(f, config)| f.ident.as_ref().map(|ident| (ident, config)))
		.collect();

	// Get proto type name
	// Read from #[graphql(proto = "...")] attribute
	let mut proto_type: Option<TokenStream> = None;

	for attr in &input.attrs {
		if attr.path().is_ident("graphql") {
			attr.parse_nested_meta(|meta| {
				if meta.path.is_ident("proto") {
					let value = meta.value()?;
					let proto_path: syn::LitStr = value.parse()?;
					let proto_tokens: TokenStream = proto_path.value().parse().map_err(|_| {
						syn::Error::new_spanned(&proto_path, "Invalid proto type path")
					})?;
					proto_type = Some(proto_tokens);
				}
				Ok(())
			})?;
		}
	}

	let proto_type = proto_type.unwrap_or_else(|| {
		// Default is crate::proto::{name}
		quote! { crate::proto::#name }
	});

	// Build field conversions for From<proto> for GraphQL
	let from_proto_fields: Vec<TokenStream> = field_data
		.iter()
		.map(|(field_name, config)| {
			let proto_field = if let Some(proto_name) = &config.proto_name {
				let proto_ident = Ident::new(proto_name, field_name.span());
				quote! { #proto_ident }
			} else {
				quote! { #field_name }
			};

			if let Some(skip_condition) = &config.skip_if {
				// Parse skip condition as TokenStream
				let condition: TokenStream =
					skip_condition.parse().unwrap_or_else(|_| quote! { false });
				quote! {
					#field_name: if #condition(&proto.#proto_field) {
						Default::default()
					} else {
						proto.#proto_field.into()
					}
				}
			} else {
				quote! {
					#field_name: proto.#proto_field.into()
				}
			}
		})
		.collect();

	// Build field conversions for From<GraphQL> for proto
	let into_proto_fields: Vec<TokenStream> = field_data
		.iter()
		.map(|(field_name, config)| {
			let proto_field = if let Some(proto_name) = &config.proto_name {
				let proto_ident = Ident::new(proto_name, field_name.span());
				quote! { #proto_ident }
			} else {
				quote! { #field_name }
			};

			if let Some(skip_condition) = &config.skip_if {
				let condition: TokenStream =
					skip_condition.parse().unwrap_or_else(|_| quote! { false });
				quote! {
					#proto_field: if #condition(&graphql.#field_name) {
						Default::default()
					} else {
						graphql.#field_name.into()
					}
				}
			} else {
				quote! {
					#proto_field: graphql.#field_name.into()
				}
			}
		})
		.collect();

	// Implementation of From<proto> for GraphQL
	let from_proto = quote! {
		impl From<#proto_type> for #name {
			fn from(proto: #proto_type) -> Self {
				Self {
					#( #from_proto_fields ),*
				}
			}
		}
	};

	// Implementation of From<GraphQL> for proto
	let into_proto = quote! {
		impl From<#name> for #proto_type {
			fn from(graphql: #name) -> Self {
				Self {
					#( #into_proto_fields ),*
				}
			}
		}
	};

	Ok(quote! {
		#from_proto
		#into_proto
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use syn::parse_quote;

	#[test]
	fn test_basic_struct() {
		let input: DeriveInput = parse_quote! {
			struct User {
				id: String,
				name: String,
			}
		};

		let result = expand_derive(input);
		assert!(result.is_ok());

		let output = result.unwrap();
		let output_str = output.to_string();

		// Check existence of From<proto> implementation
		assert!(output_str.contains("impl From < crate :: proto :: User > for User"));
		// Check existence of From<GraphQL> implementation
		assert!(output_str.contains("impl From < User > for crate :: proto :: User"));
	}

	#[test]
	fn test_field_rename() {
		let input: DeriveInput = parse_quote! {
			struct User {
				id: String,
				#[proto(name = "user_name")]
				name: String,
			}
		};

		let result = expand_derive(input);
		assert!(result.is_ok());

		let output = result.unwrap();
		let output_str = output.to_string();

		// Check that proto field name is used
		assert!(output_str.contains("user_name"));
	}

	#[test]
	fn test_skip_if() {
		let input: DeriveInput = parse_quote! {
			struct User {
				id: String,
				#[graphql(skip_if = "Option::is_none")]
				email: Option<String>,
			}
		};

		let result = expand_derive(input);
		assert!(result.is_ok());

		let output = result.unwrap();
		let output_str = output.to_string();

		// Check that skip condition is present
		assert!(output_str.contains("Option :: is_none"));
	}

	#[test]
	fn test_enum_error() {
		let input: DeriveInput = parse_quote! {
			enum UserType {
				Admin,
				Regular,
			}
		};

		let result = expand_derive(input);
		assert!(result.is_err());
	}
}
