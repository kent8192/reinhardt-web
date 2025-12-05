//! Implementation of the `#[injectable]` macro

use crate::utils::extract_scope_from_args;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Fields, Result};

/// Check if a field has #[inject] attribute
fn has_inject_attr(field: &syn::Field) -> bool {
	field
		.attrs
		.iter()
		.any(|attr| attr.path().is_ident("inject"))
}

/// Check if a field has #[no_inject] attribute
fn has_no_inject_attr(field: &syn::Field) -> bool {
	field
		.attrs
		.iter()
		.any(|attr| attr.path().is_ident("no_inject"))
}

/// Check if field should use cache (always true by default)
fn should_use_cache(_field: &syn::Field) -> bool {
	// Always use cache for injected fields
	// Future: Could support per-field cache control with additional attributes
	true
}

/// Implementation of the `#[injectable]` attribute macro
///
/// This macro:
/// 1. Implements the `Injectable` trait for the struct (with field injection support)
/// 2. Registers the type with the global dependency registry using inventory
pub fn injectable_impl(args: TokenStream, input: DeriveInput) -> Result<TokenStream> {
	let struct_name = &input.ident;
	let generics = &input.generics;
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

	// Extract scope from macro arguments (currently unused, but kept for future use)
	let _scope = extract_scope_from_args(args)?;

	// Validate that this is a struct and extract fields
	let fields = match &input.data {
		syn::Data::Struct(data_struct) => match &data_struct.fields {
			Fields::Named(fields) => Some(&fields.named),
			Fields::Unit => None, // Unit struct (no fields)
			Fields::Unnamed(_) => {
				return Err(syn::Error::new_spanned(
					struct_name,
					"#[injectable] cannot be applied to tuple structs",
				));
			}
		},
		_ => {
			return Err(syn::Error::new_spanned(
				struct_name,
				"#[injectable] can only be applied to structs",
			));
		}
	};

	// Process fields: require explicit #[inject] or #[no_inject]
	let mut has_inject_fields = false;
	let mut inject_stmts = Vec::new();
	let mut field_inits = Vec::new();

	if let Some(fields) = fields {
		for field in fields {
			let name = &field.ident;
			let ty = &field.ty;
			let has_inject = has_inject_attr(field);
			let has_no_inject = has_no_inject_attr(field);

			// Validate: field must have either #[inject] or #[no_inject]
			if has_inject && has_no_inject {
				return Err(syn::Error::new_spanned(
					field,
					"Field cannot have both #[inject] and #[no_inject] attributes",
				));
			}

			if !has_inject && !has_no_inject {
				return Err(syn::Error::new_spanned(
					field,
					"Field must have either #[inject] or #[no_inject] attribute. Use #[inject] for dependency injection, or #[no_inject] for default initialization.",
				));
			}

			if has_inject {
				// Inject this field
				has_inject_fields = true;
				let use_cache = should_use_cache(field);

				// Generate Depends::<T>::resolve() call
				let resolve_call = if use_cache {
					quote! {
						{
							let __depends = ::reinhardt_di::Depends::<#ty>::resolve(__di_ctx, true)
								.await
								.map_err(|e| {
									eprintln!("Dependency injection failed for {} in {}: {:?}",
										stringify!(#name), stringify!(#struct_name), e);
									e
								})?;
							(*__depends).clone()
						}
					}
				} else {
					quote! {
						{
							let __depends = ::reinhardt_di::Depends::<#ty>::resolve(__di_ctx, false)
								.await
								.map_err(|e| {
									eprintln!("Dependency injection failed for {} in {}: {:?}",
										stringify!(#name), stringify!(#struct_name), e);
									e
								})?;
							(*__depends).clone()
						}
					}
				};

				inject_stmts.push(quote! {
					let #name = #resolve_call;
				});
				field_inits.push(quote! { #name });
			} else {
				// Use Default::default() for fields marked with #[no_inject]
				field_inits.push(quote! { #name: Default::default() });
			}
		}
	}

	// Remove #[inject] and #[no_inject] attributes from fields to avoid "unknown attribute" errors
	let mut cleaned_input = input.clone();
	if let syn::Data::Struct(ref mut data_struct) = cleaned_input.data
		&& let syn::Fields::Named(ref mut fields_named) = data_struct.fields
	{
		for field in fields_named.named.iter_mut() {
			field.attrs.retain(|attr| {
				!attr.path().is_ident("inject") && !attr.path().is_ident("no_inject")
			});
		}
	}

	// Generate the Injectable implementation
	let injectable_impl = if has_inject_fields {
		// With field injection
		quote! {
			#[async_trait::async_trait]
			impl #impl_generics ::reinhardt_di::Injectable for #struct_name #ty_generics #where_clause {
				async fn inject(__di_ctx: &::reinhardt_di::InjectionContext) -> ::reinhardt_di::DiResult<Self> {
					#(#inject_stmts)*

					Ok(Self {
						#(#field_inits),*
					})
				}
			}
		}
	} else {
		// Without field injection - use Default::default()
		quote! {
			#[async_trait::async_trait]
			impl #impl_generics ::reinhardt_di::Injectable for #struct_name #ty_generics #where_clause {
				async fn inject(_ctx: &::reinhardt_di::InjectionContext) -> ::reinhardt_di::DiResult<Self> {
					// Use Default::default() for types without field injection
					Ok(<Self as ::std::default::Default>::default())
				}
			}
		}
	};

	// Combine cleaned struct (without #[inject] attributes) and Injectable impl
	// Note: Global registry registration is removed to avoid const context issues
	let expanded = quote! {
		#cleaned_input

		#injectable_impl
	};

	Ok(expanded)
}
