//! Procedural macros for reinhardt-query.
//!
//! Provides `#[derive(Iden)]` for automatic SQL identifier name generation.

use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

/// Derive the `Iden` trait for enums and structs.
///
/// For enums:
/// - `Table` variant uses the enum name converted to snake_case
/// - Other variants use their name converted to snake_case
/// - `#[iden = "custom_name"]` overrides the generated name
///
/// For structs:
/// - Uses the struct name converted to snake_case
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_query::Iden;
///
/// #[derive(Iden)]
/// enum Users {
///     Table,           // -> "users"
///     Id,              // -> "id"
///     FirstName,       // -> "first_name"
///     #[iden = "email_address"]
///     Email,           // -> "email_address"
/// }
/// ```
#[proc_macro_derive(Iden, attributes(iden))]
pub fn derive_iden(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	let name = &input.ident;

	let expanded = match &input.data {
		Data::Enum(data_enum) => {
			let enum_name_snake = name.to_string().to_snake_case();

			let match_arms = data_enum.variants.iter().map(|variant| {
				let variant_ident = &variant.ident;

				// Check for #[iden = "custom_name"] attribute
				let custom_name = variant.attrs.iter().find_map(|attr| {
					if attr.path().is_ident("iden") {
						attr.parse_args::<syn::LitStr>().ok().map(|lit| lit.value())
					} else {
						None
					}
				});

				let iden_name = if let Some(custom) = custom_name {
					custom
				} else if variant_ident == "Table" {
					enum_name_snake.clone()
				} else {
					variant_ident.to_string().to_snake_case()
				};

				quote! {
					Self::#variant_ident => write!(s, "{}", #iden_name).unwrap(),
				}
			});

			quote! {
				impl reinhardt_query::types::Iden for #name {
					fn unquoted(&self, s: &mut dyn ::std::fmt::Write) {
						match self {
							#(#match_arms)*
						}
					}
				}
			}
		}
		Data::Struct(data_struct) => {
			let struct_name_snake = name.to_string().to_snake_case();

			// Check for #[iden = "custom_name"] attribute on the struct
			let custom_name = input.attrs.iter().find_map(|attr| {
				if attr.path().is_ident("iden") {
					attr.parse_args::<syn::LitStr>().ok().map(|lit| lit.value())
				} else {
					None
				}
			});

			let iden_name = custom_name.unwrap_or(struct_name_snake);

			// Support unit structs, named structs, and tuple structs
			match &data_struct.fields {
				Fields::Unit | Fields::Named(_) | Fields::Unnamed(_) => {
					quote! {
						impl reinhardt_query::types::Iden for #name {
							fn unquoted(&self, s: &mut dyn ::std::fmt::Write) {
								write!(s, "{}", #iden_name).unwrap();
							}
						}
					}
				}
			}
		}
		Data::Union(_) => {
			return syn::Error::new_spanned(name, "Iden cannot be derived for unions")
				.to_compile_error()
				.into();
		}
	};

	TokenStream::from(expanded)
}
