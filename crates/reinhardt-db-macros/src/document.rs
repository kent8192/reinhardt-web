//! Implementation of the `#[document(...)]` attribute macro.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

pub(crate) mod attr_parser;

use attr_parser::DocumentAttrs;

/// Implementation of the `#[document(...)]` attribute macro.
pub(crate) fn document_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
	// Parse attributes
	let attrs = parse_macro_input!(attr as DocumentAttrs);

	// Parse the struct
	let input = parse_macro_input!(item as DeriveInput);

	// Extract struct information
	let struct_name = &input.ident;
	let collection = &attrs.collection;
	let database = attrs.database.as_deref().unwrap_or("default");

	// Extract fields and find primary key
	let fields = match &input.data {
		Data::Struct(data) => match &data.fields {
			Fields::Named(fields) => &fields.named,
			_ => {
				return syn::Error::new_spanned(
					&input,
					"#[document(...)] only supports structs with named fields",
				)
				.to_compile_error()
				.into();
			}
		},
		_ => {
			return syn::Error::new_spanned(&input, "#[document(...)] only supports structs")
				.to_compile_error()
				.into();
		}
	};

	// Find primary key field
	let mut primary_key_field = None;
	for field in fields {
		for attr in &field.attrs {
			if attr.path().is_ident("field") {
				// Parse field attributes to check for primary_key
				if let Ok(field_attrs) = attr.parse_args::<crate::field::attr_parser::FieldAttrs>()
				{
					if field_attrs.primary_key {
						primary_key_field = Some(field);
						break;
					}
				} else if let Ok(meta) = attr.parse_args::<syn::Ident>() {
					// Support simple #[field(primary_key)] syntax
					if meta == "primary_key" {
						primary_key_field = Some(field);
						break;
					}
				}
			}
		}
		if primary_key_field.is_some() {
			break;
		}
	}

	let (id_type, id_field_name) = if let Some(field) = primary_key_field {
		let ty = &field.ty;
		let name = field.ident.as_ref().unwrap();
		(quote! { #ty }, quote! { #name })
	} else {
		return syn::Error::new_spanned(
			&input,
			"No primary key field found. Add #[field(primary_key)] to one field.",
		)
		.to_compile_error()
		.into();
	};

	// Generate Document trait implementation
	let expanded = quote! {
		#input

		#[automatically_derived]
		impl reinhardt_db::nosql::document::Document for #struct_name {
			type Id = #id_type;

			const COLLECTION_NAME: &'static str = #collection;
			const DATABASE_NAME: &'static str = #database;

			fn id(&self) -> Option<&Self::Id> {
				self.#id_field_name.as_ref()
			}

			fn set_id(&mut self, id: Self::Id) {
				self.#id_field_name = Some(id);
			}
		}
	};

	TokenStream::from(expanded)
}
