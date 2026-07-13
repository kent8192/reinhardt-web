use std::collections::HashMap;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Data, DeriveInput, Expr, ExprLit, ExprUnary, Fields, Lit, LitInt, LitStr, Result, UnOp};

use crate::crate_paths::get_reinhardt_db_orm_crate;

#[derive(Clone, Copy)]
enum EnumRepr {
	String,
	I32,
}

enum EnumValue {
	String(LitStr),
	I32(i32, Span),
}

struct VariantValue {
	ident: syn::Ident,
	value: EnumValue,
}

struct GeneratedParts {
	storage: TokenStream,
	repr: TokenStream,
	max_chars: TokenStream,
	encode_arms: Vec<TokenStream>,
	decode_body: TokenStream,
	values: Vec<TokenStream>,
	domain_values: Vec<TokenStream>,
}

/// Generates explicit database codecs and metadata for a model enum.
pub(crate) fn model_enum_derive_impl(input: DeriveInput) -> Result<TokenStream> {
	let repr = parse_repr(&input)?;
	let variants = parse_variants(&input, repr)?;
	let orm = get_reinhardt_db_orm_crate();
	let enum_ident = &input.ident;
	let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

	let GeneratedParts {
		storage,
		repr: repr_token,
		max_chars,
		encode_arms,
		decode_body,
		values,
		domain_values,
	} = match repr {
		EnumRepr::String => generate_string_parts(&orm, &variants),
		EnumRepr::I32 => generate_i32_parts(&orm, &variants),
	};

	Ok(quote! {
		impl #impl_generics #orm::DatabaseField for #enum_ident #type_generics #where_clause {
			type Storage = #storage;
			const MAX_STRING_VALUE_CHARS: ::core::option::Option<usize> = #max_chars;

			fn encode_database(
				&self,
			) -> ::core::result::Result<Self::Storage, #orm::FieldCodecError> {
				match self {
					#(#encode_arms),*
				}
			}

			fn decode_database(
				value: Self::Storage,
				context: &#orm::FieldCodecContext,
			) -> ::core::result::Result<Self, #orm::FieldCodecError> {
				#decode_body
			}

			fn domain() -> ::core::option::Option<#orm::FieldDomain> {
				::core::option::Option::Some(#orm::FieldDomain::Enum {
					repr: #repr_token,
					values: ::std::vec![#(#domain_values),*],
				})
			}
		}

		impl #impl_generics #orm::ModelEnum for #enum_ident #type_generics #where_clause {
			const REPR: #orm::ModelEnumRepr = #repr_token;
			const VALUES: &'static [#orm::ModelEnumValueRef] = &[#(#values),*];
		}
	})
}

fn parse_repr(input: &DeriveInput) -> Result<EnumRepr> {
	let mut repr = None;
	for attribute in input
		.attrs
		.iter()
		.filter(|attribute| attribute.path().is_ident("model_enum"))
	{
		attribute.parse_nested_meta(|meta| {
			if meta.path.is_ident("repr") {
				if repr.is_some() {
					return Err(meta.error("duplicate `repr` setting"));
				}
				let literal: LitStr = meta.value()?.parse()?;
				repr = Some(match literal.value().as_str() {
					"string" => EnumRepr::String,
					"i32" => EnumRepr::I32,
					_ => {
						return Err(syn::Error::new(
							literal.span(),
							"`repr` must be either \"string\" or \"i32\"",
						));
					}
				});
				Ok(())
			} else {
				Err(meta.error("unsupported model enum setting; expected `repr`"))
			}
		})?;
	}

	repr.ok_or_else(|| {
		syn::Error::new(
			input.ident.span(),
			"ModelEnum requires #[model_enum(repr = \"string\" | \"i32\")]",
		)
	})
}

fn parse_variants(input: &DeriveInput, repr: EnumRepr) -> Result<Vec<VariantValue>> {
	let data = match &input.data {
		Data::Enum(data) => data,
		_ => {
			return Err(syn::Error::new_spanned(
				&input.ident,
				"ModelEnum can only be derived for enums",
			));
		}
	};
	if data.variants.is_empty() {
		return Err(syn::Error::new_spanned(
			&input.ident,
			"ModelEnum requires at least one variant",
		));
	}

	let mut parsed = Vec::with_capacity(data.variants.len());
	let mut seen = HashMap::<String, String>::new();
	for variant in &data.variants {
		if !matches!(variant.fields, Fields::Unit) {
			return Err(syn::Error::new_spanned(
				&variant.fields,
				"ModelEnum variants must be unit variants",
			));
		}
		if variant.discriminant.is_some() {
			return Err(syn::Error::new_spanned(
				&variant.ident,
				"ModelEnum variants must not have explicit discriminants; use #[model_enum(value = ...)]",
			));
		}

		let value = parse_variant_value(variant, repr)?;
		let key = match &value {
			EnumValue::String(value) => format!("string:{}", value.value()),
			EnumValue::I32(value, _) => format!("i32:{value}"),
		};
		let variant_name = variant.ident.to_string();
		if let Some(first_variant) = seen.insert(key, variant_name.clone()) {
			return Err(syn::Error::new_spanned(
				&variant.ident,
				format!(
					"duplicate model enum value on variants `{first_variant}` and `{variant_name}`"
				),
			));
		}
		parsed.push(VariantValue {
			ident: variant.ident.clone(),
			value,
		});
	}
	Ok(parsed)
}

fn parse_variant_value(variant: &syn::Variant, repr: EnumRepr) -> Result<EnumValue> {
	let mut expression = None;
	for attribute in variant
		.attrs
		.iter()
		.filter(|attribute| attribute.path().is_ident("model_enum"))
	{
		attribute.parse_nested_meta(|meta| {
			if !meta.path.is_ident("value") {
				return Err(meta.error("unsupported model enum variant setting; expected `value`"));
			}
			if expression.is_some() {
				return Err(meta.error("duplicate `value` setting"));
			}
			expression = Some(meta.value()?.parse::<Expr>()?);
			Ok(())
		})?;
	}
	let expression = expression.ok_or_else(|| {
		syn::Error::new_spanned(
			&variant.ident,
			"ModelEnum variants require #[model_enum(value = ...)]",
		)
	})?;

	match repr {
		EnumRepr::String => match expression {
			Expr::Lit(ExprLit {
				lit: Lit::Str(value),
				..
			}) => Ok(EnumValue::String(value)),
			value => Err(syn::Error::new_spanned(
				value,
				"string model enum values must be string literals",
			)),
		},
		EnumRepr::I32 => parse_i32_expression(expression),
	}
}

fn parse_i32_expression(expression: Expr) -> Result<EnumValue> {
	let (literal, negative, span) = match expression {
		Expr::Lit(ExprLit {
			lit: Lit::Int(literal),
			..
		}) => {
			let span = literal.span();
			(literal, false, span)
		}
		Expr::Unary(ExprUnary {
			op: UnOp::Neg(_),
			expr,
			..
		}) => match *expr {
			Expr::Lit(ExprLit {
				lit: Lit::Int(literal),
				..
			}) => {
				let span = literal.span();
				(literal, true, span)
			}
			value => {
				return Err(syn::Error::new_spanned(
					value,
					"i32 model enum values must be integer literals",
				));
			}
		},
		value => {
			return Err(syn::Error::new_spanned(
				value,
				"i32 model enum values must be integer literals",
			));
		}
	};
	if !matches!(literal.suffix(), "" | "i32") {
		return Err(syn::Error::new_spanned(
			literal,
			"i32 model enum values may only use the `i32` suffix",
		));
	}
	let magnitude = literal.base10_parse::<i128>()?;
	let signed = if negative { -magnitude } else { magnitude };
	let value = i32::try_from(signed)
		.map_err(|_| syn::Error::new(span, "model enum value is outside the i32 range"))?;
	Ok(EnumValue::I32(value, span))
}

fn generate_string_parts(orm: &TokenStream, variants: &[VariantValue]) -> GeneratedParts {
	let max_chars = variants
		.iter()
		.map(|variant| match &variant.value {
			EnumValue::String(value) => value.value().chars().count(),
			EnumValue::I32(_, _) => unreachable!("representation validated during parsing"),
		})
		.max()
		.expect("non-empty enum validated during parsing");
	let encode_arms = variants
		.iter()
		.map(|variant| {
			let ident = &variant.ident;
			let EnumValue::String(value) = &variant.value else {
				unreachable!("representation validated during parsing")
			};
			quote!(
				Self::#ident => ::core::result::Result::Ok(
					::std::string::String::from(#value)
				)
			)
		})
		.collect();
	let decode_arms = variants.iter().map(|variant| {
		let ident = &variant.ident;
		let EnumValue::String(value) = &variant.value else {
			unreachable!("representation validated during parsing")
		};
		quote!(#value => ::core::result::Result::Ok(Self::#ident))
	});
	let values = variants
		.iter()
		.map(|variant| {
			let EnumValue::String(value) = &variant.value else {
				unreachable!("representation validated during parsing")
			};
			quote!(#orm::ModelEnumValueRef::String(#value))
		})
		.collect();
	let domain_values = variants
		.iter()
		.map(|variant| {
			let EnumValue::String(value) = &variant.value else {
				unreachable!("representation validated during parsing")
			};
			quote!(#orm::ModelEnumValue::String(::std::string::String::from(#value)))
		})
		.collect();

	GeneratedParts {
		storage: quote!(::std::string::String),
		repr: quote!(#orm::ModelEnumRepr::String),
		max_chars: quote!(::core::option::Option::Some(#max_chars)),
		encode_arms,
		decode_body: quote! {
			match value.as_str() {
				#(#decode_arms),*,
				_ => ::core::result::Result::Err(#orm::FieldCodecError::invalid_enum(
					::core::clone::Clone::clone(context),
					#orm::ModelEnumRepr::String,
					#orm::ModelEnumValue::String(value),
				)),
			}
		},
		values,
		domain_values,
	}
}

fn generate_i32_parts(orm: &TokenStream, variants: &[VariantValue]) -> GeneratedParts {
	let encode_arms = variants
		.iter()
		.map(|variant| {
			let ident = &variant.ident;
			let EnumValue::I32(value, span) = variant.value else {
				unreachable!("representation validated during parsing")
			};
			let value = LitInt::new(&format!("{value}i32"), span);
			quote!(Self::#ident => ::core::result::Result::Ok(#value))
		})
		.collect();
	let decode_arms = variants.iter().map(|variant| {
		let ident = &variant.ident;
		let EnumValue::I32(value, span) = variant.value else {
			unreachable!("representation validated during parsing")
		};
		let value = LitInt::new(&format!("{value}i32"), span);
		quote!(#value => ::core::result::Result::Ok(Self::#ident))
	});
	let values = variants
		.iter()
		.map(|variant| {
			let EnumValue::I32(value, span) = variant.value else {
				unreachable!("representation validated during parsing")
			};
			let value = LitInt::new(&format!("{value}i32"), span);
			quote!(#orm::ModelEnumValueRef::I32(#value))
		})
		.collect();
	let domain_values = variants
		.iter()
		.map(|variant| {
			let EnumValue::I32(value, span) = variant.value else {
				unreachable!("representation validated during parsing")
			};
			let value = LitInt::new(&format!("{value}i32"), span);
			quote!(#orm::ModelEnumValue::I32(#value))
		})
		.collect();

	GeneratedParts {
		storage: quote!(i32),
		repr: quote!(#orm::ModelEnumRepr::I32),
		max_chars: quote!(::core::option::Option::None),
		encode_arms,
		decode_body: quote! {
			match value {
				#(#decode_arms),*,
				value => ::core::result::Result::Err(#orm::FieldCodecError::invalid_enum(
					::core::clone::Clone::clone(context),
					#orm::ModelEnumRepr::I32,
					#orm::ModelEnumValue::I32(value),
				)),
			}
		},
		values,
		domain_values,
	}
}

#[cfg(test)]
mod tests {
	use syn::{DeriveInput, parse_quote};

	use super::model_enum_derive_impl;

	#[test]
	fn generates_string_database_codec_tokens() {
		let input: DeriveInput = parse_quote! {
			#[model_enum(repr = "string")]
			enum Status {
				#[model_enum(value = "queued")]
				Queued,
				#[model_enum(value = "completed")]
				Completed,
			}
		};

		let tokens = model_enum_derive_impl(input).unwrap().to_string();

		assert!(tokens.contains("type Storage = :: std :: string :: String"));
		assert!(tokens.contains(
			"MAX_STRING_VALUE_CHARS : :: core :: option :: Option < usize > = :: core :: option :: Option :: Some (9usize)"
		));
		assert!(tokens.contains(
			"Self :: Queued => :: core :: result :: Result :: Ok (:: std :: string :: String :: from (\"queued\"))"
		));
		assert!(
			tokens
				.contains("\"completed\" => :: core :: result :: Result :: Ok (Self :: Completed)")
		);
		assert!(tokens.contains("-> :: core :: result :: Result < Self :: Storage"));
		assert!(tokens.contains(":: std :: vec ! ["));
		assert!(tokens.contains(":: core :: result :: Result :: Err ("));
		assert!(tokens.contains(":: core :: clone :: Clone :: clone (context)"));
		assert!(tokens.contains(
			"ModelEnumValue :: String (:: std :: string :: String :: from (\"queued\"))"
		));
		assert!(tokens.contains("FieldCodecError :: invalid_enum"));
		assert!(tokens.contains("ModelEnumValueRef :: String (\"queued\")"));
	}

	#[test]
	fn generates_i32_database_codec_tokens() {
		let input: DeriveInput = parse_quote! {
			#[model_enum(repr = "i32")]
			enum Priority {
				#[model_enum(value = -1)]
				Low,
				#[model_enum(value = 2)]
				High,
			}
		};

		let tokens = model_enum_derive_impl(input).unwrap().to_string();

		assert!(tokens.contains("type Storage = i32"));
		assert!(tokens.contains(
			"MAX_STRING_VALUE_CHARS : :: core :: option :: Option < usize > = :: core :: option :: Option :: None"
		));
		assert!(tokens.contains("Self :: Low => :: core :: result :: Result :: Ok (- 1i32)"));
		assert!(tokens.contains("2i32 => :: core :: result :: Result :: Ok (Self :: High)"));
		assert!(tokens.contains("ModelEnumValueRef :: I32 (- 1i32)"));
	}
}
