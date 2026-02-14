//! Implementation of the `#[document(...)]` attribute macro.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, GenericArgument, PathArguments, Type, parse_macro_input};

pub(crate) mod attr_parser;

use attr_parser::DocumentAttrs;

/// Implementation of the `#[document(...)]` attribute macro.
pub(crate) fn document_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
	// Parse attributes
	let attrs = parse_macro_input!(attr as DocumentAttrs);

	// Parse the struct
	let mut input = parse_macro_input!(item as DeriveInput);

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

		// Extract inner type from Option<T> -> T
		let inner_type = extract_option_inner_type(ty).unwrap_or_else(|| ty.clone());

		(quote! { #inner_type }, quote! { #name })
	} else {
		return syn::Error::new_spanned(
			&input,
			"No primary key field found. Add #[field(primary_key)] to one field.",
		)
		.to_compile_error()
		.into();
	};

	// TODO: [PR#31] Inject serde attributes: primary_key→#[serde(rename="_id")], rename→#[serde(rename=...)], default→#[serde(default=...)]
	// Strip #[field(...)] attributes from fields before output
	// This is necessary because #[field] is not a real attribute macro that Rust can process
	if let Data::Struct(ref mut data) = input.data
		&& let Fields::Named(ref mut fields) = data.fields
	{
		for field in fields.named.iter_mut() {
			field.attrs.retain(|attr| !attr.path().is_ident("field"));
		}
	}

	// TODO: [PR#31] Generate indexes(), validate(), validation_schema(), backend_type(), references() methods
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

/// Extracts the inner type from `Option<T>`, returning `Some(T)` if successful.
/// Returns `None` if the type is not an `Option`.
fn extract_option_inner_type(ty: &Type) -> Option<Type> {
	let Type::Path(type_path) = ty else {
		return None;
	};

	let last_segment = type_path.path.segments.last()?;

	// Check if this is Option (or std::option::Option, core::option::Option)
	if last_segment.ident != "Option" {
		return None;
	}

	let PathArguments::AngleBracketed(args) = &last_segment.arguments else {
		return None;
	};

	if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
		return Some(inner_ty.clone());
	}

	None
}
