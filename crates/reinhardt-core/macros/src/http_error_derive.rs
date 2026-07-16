use crate::crate_paths::{get_reinhardt_core_crate, get_reinhardt_http_crate};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Attribute, Data, DeriveInput, Expr, Fields, Ident, Lit, Result, Variant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BodyMode {
	Safe,
	Error,
}

#[derive(Debug, Clone)]
enum MessageSource {
	Fixed(String),
	Method(Ident),
}

#[derive(Debug, Clone)]
struct EnumConfig {
	response: bool,
	body: BodyMode,
}

#[derive(Debug, Clone)]
struct VariantConfig {
	status: Ident,
	message: MessageSource,
}

pub(crate) fn derive_http_error_impl(input: DeriveInput) -> Result<TokenStream> {
	let name = &input.ident;
	let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
	let enum_config = parse_enum_config(&input.attrs)?;
	let data_enum = match &input.data {
		Data::Enum(data_enum) => data_enum,
		_ => {
			return Err(syn::Error::new_spanned(
				&input.ident,
				"#[derive(HttpError)] can only be used on enums",
			));
		}
	};

	let core_crate = get_reinhardt_core_crate();
	let status_arms = data_enum
		.variants
		.iter()
		.map(|variant| {
			let pattern = variant_pattern(variant);
			let cfg = parse_variant_config(variant)?;
			let status = cfg.status;
			Ok(quote! {
				#pattern => #core_crate::exception::StatusCode::#status,
			})
		})
		.collect::<Result<Vec<_>>>()?;

	let message_arms = data_enum
		.variants
		.iter()
		.map(|variant| {
			let pattern = variant_pattern(variant);
			let cfg = parse_variant_config(variant)?;
			Ok(match cfg.message {
				MessageSource::Fixed(message) => quote! {
					#pattern => ::std::borrow::Cow::Borrowed(#message),
				},
				MessageSource::Method(method) => quote! {
					#pattern => ::std::convert::Into::<::std::borrow::Cow<'static, str>>::into(self.#method()),
				},
			})
		})
		.collect::<Result<Vec<_>>>()?;

	let message_fn_checks = data_enum
		.variants
		.iter()
		.map(|variant| {
			let cfg = parse_variant_config(variant)?;
			let MessageSource::Method(method) = cfg.message else {
				return Ok(quote! {});
			};
			let variant_ident = &variant.ident;
			let check_mod = format_ident!(
				"__reinhardt_http_error_{}_{}_message_fn_check",
				name,
				variant_ident
			);
			Ok(quote! {
				const _: () = {
					// This generated module exists only to type-check the configured method.
					#[allow(dead_code, non_snake_case)]
					mod #check_mod {
						// The check function is compiled for its type constraints, not called.
						#[allow(dead_code)]
						fn check #impl_generics(error: &super::#name #ty_generics) #where_clause {
							let _ = super::#name::#method(error);
						}
					}
				};
			})
		})
		.collect::<Result<Vec<_>>>()?;

	let trait_impl = quote! {
		impl #impl_generics #core_crate::exception::HttpError for #name #ty_generics #where_clause {
			fn status_code(&self) -> #core_crate::exception::StatusCode {
				match self {
					#(#status_arms)*
				}
			}

			#[deny(unconditional_recursion)]
			fn client_message(&self) -> ::std::borrow::Cow<'static, str> {
				match self {
					#(#message_arms)*
				}
			}
		}
	};

	let response_impl = if enum_config.response {
		let http_crate = get_reinhardt_http_crate();
		let body = match enum_config.body {
			BodyMode::Safe => quote! { #http_crate::Response::from_http_error(error) },
			BodyMode::Error => quote! { #http_crate::Response::from_http_error_body(error) },
		};
		quote! {
			impl #impl_generics ::std::convert::From<#name #ty_generics> for #http_crate::Response #where_clause {
				fn from(error: #name #ty_generics) -> Self {
					#body
				}
			}
		}
	} else {
		quote! {}
	};

	Ok(quote! {
		#(#message_fn_checks)*
		#trait_impl
		#response_impl
	})
}

fn parse_enum_config(attrs: &[Attribute]) -> Result<EnumConfig> {
	let mut config = EnumConfig {
		response: false,
		body: BodyMode::Safe,
	};

	for attr in attrs
		.iter()
		.filter(|attr| attr.path().is_ident("http_error"))
	{
		attr.parse_nested_meta(|meta| {
			if meta.path.is_ident("response") {
				config.response = true;
				return Ok(());
			}
			if meta.path.is_ident("body") {
				let value = meta.value()?;
				let lit: Lit = value.parse()?;
				let Lit::Str(lit_str) = lit else {
					return Err(meta.error("body must be \"safe\" or \"error\""));
				};
				config.body = match lit_str.value().as_str() {
					"safe" => BodyMode::Safe,
					"error" => BodyMode::Error,
					_ => return Err(meta.error("body must be \"safe\" or \"error\"")),
				};
				return Ok(());
			}
			Err(meta.error("unknown #[http_error(...)] enum option"))
		})?;
	}

	if !config.response && config.body != BodyMode::Safe {
		return Err(syn::Error::new(
			proc_macro2::Span::call_site(),
			"`body` requires `response`",
		));
	}

	Ok(config)
}

fn parse_variant_config(variant: &Variant) -> Result<VariantConfig> {
	let mut status = None;
	let mut message = None;

	for attr in variant
		.attrs
		.iter()
		.filter(|attr| attr.path().is_ident("http_error"))
	{
		attr.parse_nested_meta(|meta| {
			if meta.path.is_ident("status") {
				if status.is_some() {
					return Err(meta.error("duplicate status option"));
				}
				status = Some(parse_status_ident(meta.value()?.parse()?)?);
				return Ok(());
			}
			if meta.path.is_ident("message") {
				if message.is_some() {
					return Err(meta.error("message and message_fn are mutually exclusive"));
				}
				let lit: Lit = meta.value()?.parse()?;
				let Lit::Str(lit_str) = lit else {
					return Err(meta.error("message must be a string literal"));
				};
				message = Some(MessageSource::Fixed(lit_str.value()));
				return Ok(());
			}
			if meta.path.is_ident("message_fn") {
				if message.is_some() {
					return Err(meta.error("message and message_fn are mutually exclusive"));
				}
				message = Some(MessageSource::Method(parse_message_fn_ident(
					meta.value()?.parse()?,
				)?));
				return Ok(());
			}
			Err(meta.error("unknown #[http_error(...)] variant option"))
		})?;
	}

	let status = status
		.ok_or_else(|| syn::Error::new_spanned(variant, "missing #[http_error(status = ...)]"))?;
	let message = message.ok_or_else(|| {
		syn::Error::new_spanned(variant, "missing `message = \"...\"` or `message_fn = ...`")
	})?;

	Ok(VariantConfig { status, message })
}

fn parse_status_ident(expr: Expr) -> Result<Ident> {
	let Expr::Path(path) = expr else {
		return Err(syn::Error::new_spanned(
			expr,
			"status must be a bare StatusCode constant",
		));
	};
	if path.path.segments.len() != 1 {
		return Err(syn::Error::new_spanned(
			path,
			"status must be a bare StatusCode constant like BAD_REQUEST",
		));
	}
	Ok(path.path.segments.first().unwrap().ident.clone())
}

fn parse_message_fn_ident(expr: Expr) -> Result<Ident> {
	let Expr::Path(path) = expr else {
		return Err(syn::Error::new_spanned(
			expr,
			"message_fn must be a method name",
		));
	};
	if path.path.segments.len() != 1 {
		return Err(syn::Error::new_spanned(
			path,
			"message_fn must be a method name",
		));
	}
	Ok(path.path.segments.first().unwrap().ident.clone())
}

fn variant_pattern(variant: &Variant) -> TokenStream {
	let ident = &variant.ident;
	match &variant.fields {
		Fields::Unit => quote! { Self::#ident },
		Fields::Unnamed(_) => quote! { Self::#ident(..) },
		Fields::Named(_) => quote! { Self::#ident { .. } },
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;
	use syn::parse_quote;

	#[rstest]
	fn parse_enum_response_error_body() {
		// Arrange
		let input: DeriveInput = parse_quote! {
			#[http_error(response, body = "error")]
			enum Example {
				#[http_error(status = BAD_REQUEST, message = "Invalid")]
				Invalid,
			}
		};

		// Act
		let config = parse_enum_config(&input.attrs).unwrap();

		// Assert
		assert!(config.response);
		assert_eq!(config.body, BodyMode::Error);
	}

	#[rstest]
	fn parse_variant_fixed_message() {
		// Arrange
		let variant: Variant = parse_quote! {
			#[http_error(status = NOT_FOUND, message = "Missing")]
			Missing(String)
		};

		// Act
		let config = parse_variant_config(&variant).unwrap();

		// Assert
		assert_eq!(config.status, "NOT_FOUND");
		assert!(matches!(config.message, MessageSource::Fixed(message) if message == "Missing"));
	}

	#[rstest]
	fn parse_variant_method_message() {
		// Arrange
		let variant: Variant = parse_quote! {
			#[http_error(status = CONFLICT, message_fn = client_message)]
			Conflict { id: i64 }
		};

		// Act
		let config = parse_variant_config(&variant).unwrap();

		// Assert
		assert_eq!(config.status, "CONFLICT");
		assert!(
			matches!(config.message, MessageSource::Method(method) if method == "client_message")
		);
	}

	#[rstest]
	fn reject_duplicate_message_sources() {
		// Arrange
		let variant: Variant = parse_quote! {
			#[http_error(status = BAD_REQUEST, message = "Invalid", message_fn = client_message)]
			Invalid
		};

		// Act
		let error = parse_variant_config(&variant).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"message and message_fn are mutually exclusive"
		);
	}

	#[rstest]
	fn reject_body_without_response() {
		// Arrange
		let input: DeriveInput = parse_quote! {
			#[http_error(body = "error")]
			enum Example {
				#[http_error(status = BAD_REQUEST, message = "Invalid")]
				Invalid,
			}
		};

		// Act
		let error = parse_enum_config(&input.attrs).unwrap_err();

		// Assert
		assert_eq!(error.to_string(), "`body` requires `response`");
	}
}
