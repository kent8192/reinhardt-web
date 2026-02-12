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
			let match_arms = data_enum.variants.iter().map(|variant| {
				let variant_ident = &variant.ident;

				// Check for #[iden = "custom_name"] or #[iden("custom_name")] attribute
				let custom_name = variant.attrs.iter().find_map(|attr| {
					if !attr.path().is_ident("iden") {
						return None;
					}

					// Parse #[iden = "..."] or #[iden("...")]
					match &attr.meta {
						syn::Meta::NameValue(name_value) => {
							// Handle #[iden = "..."] syntax
							if name_value.path.is_ident("iden") {
								// Extract string literal from Expr
								if let syn::Expr::Lit(lit) = &name_value.value {
									match lit {
										syn::Lit::Str(lit_str) => Some(lit_str.clone()),
										_ => None,
									}
								} else {
									None
								}
							} else {
								None
							}
						}
						syn::Meta::List(list) => {
							// Handle #[iden("...")] syntax - skip as it's not standard
							None
						}
						_ => None,
					}
				});

				// Use custom name if provided, otherwise use variant name in snake_case
				let default_name = variant_ident.to_string().to_snake_case();
				let iden_name = match &custom_name {
					Some(lit) => lit.value(),
					None => default_name.as_str(),
				};

				quote! {
					#name::#variant_ident => write!(s, "{}", #iden_name).unwrap(),
				}
			});

			quote! {
				impl ::std::fmt::Display for #name {
					fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
						let mut s = String::new();
						match self {
							#(#match_arms)*
						}
						write!(f, "{}", s)
					}
				}
			}
		}

		Data::Struct(data_struct) => {
			let struct_name_snake = name.to_string().to_snake_case();

			// Generate match pattern based on struct type
			let match_pattern = match &data_struct.fields {
				syn::Fields::Unit => quote! { #name },
				syn::Fields::Unnamed(unnamed) => {
					let fields = unnamed.unnamed.iter().map(|_| quote! { _ });
					quote! { #name ( #(#fields),* ) }
				}
				syn::Fields::Named(named) => {
					let fields = named.named.iter().map(|field| {
						let field_ident = &field.ident;
						quote! { #field_ident: _ }
					});
					quote! { #name { #(#fields),* } }
				}
			};

			// Check for custom #[iden = "..."] or #[iden("...")] on first field
			let custom_name = data_struct.fields.iter().next().and_then(|field| {
				field.attrs.iter().find_map(|attr| {
					if !attr.path().is_ident("iden") {
						return None;
					}
					match &attr.meta {
						syn::Meta::NameValue(name_value) => {
							// Handle #[iden = "..."] syntax
							if name_value.path.is_ident("iden") {
								// Extract string literal from Expr
								if let syn::Expr::Lit(lit) = &name_value.value {
									if let syn::Lit::Str(lit_str) = lit {
										Some(lit_str.clone())
									} else {
										None
									}
								} else {
									None
								}
							} else {
								None
							}
						}
						syn::Meta::List(list) => {
							// Handle #[iden("...")] syntax - skip as it's not standard
							None
						}
						_ => None,
					}
				})
			});

			let iden_name = match &custom_name {
				Some(lit) => lit.value(),
				None => struct_name_snake.as_str(),
			};

			quote! {
				impl ::std::fmt::Display for #name {
					fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
						write!(f, "{}", #iden_name)
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
