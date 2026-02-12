// Copyright 2024-2025 reinhardt-query authors
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file except in compliance with the License. You may obtain a copy of the
// License at
//
// Unless required by applicable law or agreed to in writing, software distributed under
// the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. See the License for the specific language governing
// permissions and limitations under the License.

//! Procedural macros for reinhardt-query
//!
//! Provides custom derive macros for SQL identifier name generation:
//! - `#[derive(Iden)]` - Automatically generates SQL-safe identifiers
//!
//! ## Features
//!
//! - Supports both list-style `#[iden("custom_name")]` and name-value style `#[iden = "custom_name"]` attributes
//! - Generates snake_case identifiers for SQL compatibility
//! - Supports unit variants, named variants, and tuple variants in enums
//! - Supports all struct types (unit, named fields, and tuple structs)
//!
//! For enums:
//! - `Table` variant uses enum name converted to snake_case
//! - Other variants use their name converted to snake_case
//!
//! For structs:
//! - Uses the struct name converted to snake_case
//! - `#[iden = "custom_name"]` overrides the generated name
//!
//! # Examples
//!
//! ## Enum with custom identifier
//!
//! ```rust,ignore
//! #[derive(Iden)]
//! enum Users {
//!     Table,           // -> "users"
//!     #[iden = "primary_key"]
//!     Id,              // -> "id"
//!     FirstName,       // -> "first_name"
//!     #[iden("email_address")]
//!     Email,           // -> "email_address"
//! }
//!
//! let users = Users::Table;
//! assert_eq!(users.iden(), "users");
//! assert_eq!(users.id(), "id");
//! assert_eq!(users.first_name.iden(), "first_name");
//! assert_eq!(users.email_address.iden(), "email_address");
//! ```
//!
//! ## Struct with custom identifier
//!
//! ```rust,ignore
//! #[derive(Iden)]
//! struct Customer {
//!     #[iden = "customer_id"]
//!     Id,
//!     Name,
//! }
//!
//! let customer = Customer::Iden;
//! assert_eq!(customer.iden(), "customer_id");
//! ```
//!
//! ## Struct with named fields
//!
//! ```rust,ignore
//! #[derive(Iden)]
//! struct User {
//!     Id,
//!     #[iden = "uuid"]
//!     ExternalId,
//!     Email,
//!     Verified,
//! }
//!
//! let user = User::Iden;
//! assert_eq!(user.iden(), "uuid");
//! assert_eq!(user.external_iden(), "external_id");
//! assert_eq!(user.emailen(), "email");
//! assert_eq!(user.verifieden(), "verified");
//! ```

use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input};

/// Extract custom identifier name from `#[iden = "..."]` attribute.
fn extract_custom_name(attrs: &[syn::Attribute]) -> Option<String> {
	attrs.iter().find_map(|attr| {
		if !attr.path().is_ident("iden") {
			return None;
		}
		match &attr.meta {
			syn::Meta::NameValue(name_value) => {
				if name_value.path.is_ident("iden")
					&& let syn::Expr::Lit(lit) = &name_value.value
					&& let syn::Lit::Str(lit_str) = &lit.lit
				{
					return Some(lit_str.value());
				}
				None
			}
			_ => None,
		}
	})
}

/// Derive `Iden` trait for enums and structs.
///
/// ## Features
///
/// - Supports both list-style `#[iden("custom_name")]` and name-value style `#[iden = "custom_name"]` attributes
/// - Generates snake_case identifiers for SQL compatibility
/// - Supports unit variants, named variants, and tuple variants in enums
/// - Supports all struct types (unit, named fields, and tuple structs)
///
/// For enums:
/// - `Table` variant uses enum name converted to snake_case
/// - Other variants use their name converted to snake_case
///
/// For structs:
/// - Uses the struct name converted to snake_case
/// - `#[iden = "custom_name"]` overrides the generated name
///
/// # Examples
///
/// ## Enum with custom identifier
///
/// ```rust,ignore
/// #[derive(Iden)]
/// enum Users {
///     Table,           // -> "users"
///     #[iden = "primary_key"]
///     Id,              // -> "id"
///     FirstName,       // -> "first_name"
///     #[iden("email_address")]
///     Email,           // -> "email_address"
/// }
///
/// let users = Users::Table;
/// assert_eq!(users.iden(), "users");
/// assert_eq!(users.id(), "id");
/// assert_eq!(users.first_name.iden(), "first_name");
/// assert_eq!(users.email_address.iden(), "email_address");
/// ```
///
/// ## Struct with custom identifier
///
/// ```rust,ignore
/// #[derive(Iden)]
/// struct Customer {
///     #[iden = "customer_id"]
///     Id,
///     Name,
/// }
///
/// let customer = Customer::Iden;
/// assert_eq!(customer.iden(), "customer_id");
/// ```
///
/// ## Struct with named fields
///
/// ```rust,ignore
/// #[derive(Iden)]
/// struct User {
///     Id,
///     #[iden = "uuid"]
///     ExternalId,
///     Email,
///     Verified,
/// }
///
/// let user = User::Iden;
/// assert_eq!(user.iden(), "uuid");
/// assert_eq!(user.external_iden(), "external_id");
/// assert_eq!(user.emailen(), "email");
/// assert_eq!(user.verifieden(), "verified");
/// ```
#[proc_macro_derive(Iden, attributes(iden))]
pub fn derive_iden(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);

	let name = &input.ident;

	let expanded = match &input.data {
		Data::Enum(data_enum) => {
			// Collect variant names and their identifier strings
			let variant_data: Vec<_> = data_enum
				.variants
				.iter()
				.map(|variant| {
					let variant_ident = &variant.ident;
					let iden_name = extract_custom_name(&variant.attrs).unwrap_or_else(|| {
						if variant_ident == "Table" {
							// Convention: Table variant uses enum name as table identifier
							name.to_string().to_snake_case()
						} else {
							variant_ident.to_string().to_snake_case()
						}
					});
					(variant_ident.clone(), iden_name)
				})
				.collect();

			// Generate Display match arms
			let display_arms = variant_data.iter().map(|(variant_ident, iden_name)| {
				quote! {
					#name::#variant_ident => f.write_str(#iden_name),
				}
			});

			// Generate Iden::unquoted match arms
			let iden_arms = variant_data.iter().map(|(variant_ident, iden_name)| {
				quote! {
					#name::#variant_ident => s.write_str(#iden_name).unwrap(),
				}
			});

			quote! {
				impl ::std::fmt::Display for #name {
					fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
						match self {
							#(#display_arms)*
						}
					}
				}

				impl reinhardt_query::types::Iden for #name {
					fn unquoted(&self, s: &mut dyn ::std::fmt::Write) {
						match self {
							#(#iden_arms)*
						}
					}
				}
			}
		}

		Data::Struct(data_struct) => {
			let iden_name = data_struct
				.fields
				.iter()
				.next()
				.and_then(|field| extract_custom_name(&field.attrs))
				.unwrap_or_else(|| name.to_string().to_snake_case());

			quote! {
				impl ::std::fmt::Display for #name {
					fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
						f.write_str(#iden_name)
					}
				}

				impl reinhardt_query::types::Iden for #name {
					fn unquoted(&self, s: &mut dyn ::std::fmt::Write) {
						s.write_str(#iden_name).unwrap();
					}
				}
			}
		}

		_ => {
			quote! {
				compile_error!("`#[derive(Iden)]` only supports enums and structs");
			}
		}
	};

	TokenStream::from(expanded)
}
