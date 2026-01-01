//! Parser implementation for the `form!` macro DSL.
//!
//! This module implements `syn::Parse` for the form macro AST structures.

use syn::parse::{Parse, ParseStream};
use syn::token::Brace;
use syn::{Expr, ExprClosure, Ident, LitStr, Result, Token, braced};

use crate::{
	ClientValidator, ClientValidatorRule, FormFieldDef, FormFieldProperty, FormMacro,
	FormValidator, ValidatorRule,
};

impl Parse for FormMacro {
	fn parse(input: ParseStream) -> Result<Self> {
		let span = input.span();
		let mut form = FormMacro::new(span);

		// Parse optional sections: name, csrf, fields, validators, client_validators
		while !input.is_empty() {
			let section_name: Ident = input.parse()?;
			input.parse::<Token![:]>()?;

			match section_name.to_string().as_str() {
				"name" => {
					form.name = Some(input.parse()?);
				}
				"fields" => {
					form.fields = parse_fields(input)?;
				}
				"validators" => {
					form.validators = parse_validators(input)?;
				}
				"client_validators" => {
					form.client_validators = parse_client_validators(input)?;
				}
				other => {
					return Err(syn::Error::new(
						section_name.span(),
						format!(
							"unknown section '{}', expected one of: name, fields, validators, client_validators",
							other
						),
					));
				}
			}

			// Optional trailing comma
			if input.peek(Token![,]) {
				input.parse::<Token![,]>()?;
			}
		}

		Ok(form)
	}
}

/// Parses the fields section.
fn parse_fields(input: ParseStream) -> Result<Vec<FormFieldDef>> {
	let content;
	braced!(content in input);

	let mut fields = Vec::new();

	while !content.is_empty() {
		let field = parse_field_def(&content)?;
		fields.push(field);

		if content.peek(Token![,]) {
			content.parse::<Token![,]>()?;
		}
	}

	Ok(fields)
}

/// Parses a single field definition.
fn parse_field_def(input: ParseStream) -> Result<FormFieldDef> {
	let span = input.span();
	let name: Ident = input.parse()?;
	input.parse::<Token![:]>()?;
	let field_type: Ident = input.parse()?;

	let mut field = FormFieldDef::new(name, field_type, span);

	// Parse field properties if present
	if input.peek(Brace) {
		let content;
		braced!(content in input);

		while !content.is_empty() {
			let property = parse_field_property(&content)?;
			field.properties.push(property);

			if content.peek(Token![,]) {
				content.parse::<Token![,]>()?;
			}
		}
	}

	Ok(field)
}

/// Parses a single field property.
fn parse_field_property(input: ParseStream) -> Result<FormFieldProperty> {
	let span = input.span();
	let name: Ident = input.parse()?;

	// Check if this is a flag (no value) or named property
	if input.peek(Token![:]) {
		input.parse::<Token![:]>()?;

		// Special case: widget property
		if name == "widget" {
			let widget_type: Ident = input.parse()?;
			return Ok(FormFieldProperty::Widget { widget_type, span });
		}

		// Regular named property
		let value: Expr = input.parse()?;
		Ok(FormFieldProperty::Named { name, value, span })
	} else {
		// Flag property (e.g., `required`)
		Ok(FormFieldProperty::Flag { name, span })
	}
}

/// Parses the validators section.
fn parse_validators(input: ParseStream) -> Result<Vec<FormValidator>> {
	let content;
	braced!(content in input);

	let mut validators = Vec::new();

	while !content.is_empty() {
		let validator = parse_validator(&content)?;
		validators.push(validator);

		if content.peek(Token![,]) {
			content.parse::<Token![,]>()?;
		}
	}

	Ok(validators)
}

/// Parses a single validator definition.
fn parse_validator(input: ParseStream) -> Result<FormValidator> {
	let span = input.span();

	// Check for form-level validator: @form
	if input.peek(Token![@]) {
		input.parse::<Token![@]>()?;
		let keyword: Ident = input.parse()?;
		if keyword != "form" {
			return Err(syn::Error::new(
				keyword.span(),
				format!("expected 'form' after '@', got '{}'", keyword),
			));
		}
		input.parse::<Token![:]>()?;
		let rules = parse_validator_rules(input)?;
		return Ok(FormValidator::Form { rules, span });
	}

	// Field-level validator: field_name: [...]
	let field_name: Ident = input.parse()?;
	input.parse::<Token![:]>()?;
	let rules = parse_validator_rules(input)?;

	Ok(FormValidator::Field {
		field_name,
		rules,
		span,
	})
}

/// Parses validator rules: `[|v| ... => "message", ...]`
fn parse_validator_rules(input: ParseStream) -> Result<Vec<ValidatorRule>> {
	let content;
	syn::bracketed!(content in input);

	let mut rules = Vec::new();

	while !content.is_empty() {
		let span = content.span();
		let expr: ExprClosure = content.parse()?;
		content.parse::<Token![=>]>()?;
		let message: LitStr = content.parse()?;

		rules.push(ValidatorRule {
			expr,
			message,
			span,
		});

		if content.peek(Token![,]) {
			content.parse::<Token![,]>()?;
		}
	}

	Ok(rules)
}

/// Parses the client_validators section.
fn parse_client_validators(input: ParseStream) -> Result<Vec<ClientValidator>> {
	let content;
	braced!(content in input);

	let mut validators = Vec::new();

	while !content.is_empty() {
		let validator = parse_client_validator(&content)?;
		validators.push(validator);

		if content.peek(Token![,]) {
			content.parse::<Token![,]>()?;
		}
	}

	Ok(validators)
}

/// Parses a single client validator definition.
fn parse_client_validator(input: ParseStream) -> Result<ClientValidator> {
	let span = input.span();
	let field_name: Ident = input.parse()?;
	input.parse::<Token![:]>()?;

	let rules = parse_client_validator_rules(input)?;

	Ok(ClientValidator {
		field_name,
		rules,
		span,
	})
}

/// Parses client validator rules: `["js_expr" => "message", ...]`
fn parse_client_validator_rules(input: ParseStream) -> Result<Vec<ClientValidatorRule>> {
	let content;
	syn::bracketed!(content in input);

	let mut rules = Vec::new();

	while !content.is_empty() {
		let span = content.span();
		let js_expr: LitStr = content.parse()?;
		content.parse::<Token![=>]>()?;
		let message: LitStr = content.parse()?;

		rules.push(ClientValidatorRule {
			js_expr,
			message,
			span,
		});

		if content.peek(Token![,]) {
			content.parse::<Token![,]>()?;
		}
	}

	Ok(rules)
}

#[cfg(test)]
mod tests {
	use super::*;
	use quote::quote;

	#[test]
	fn test_parse_basic_form() {
		let tokens = quote! {
			fields: {
				name: CharField {},
			}
		};

		let form: FormMacro = syn::parse2(tokens).unwrap();
		assert_eq!(form.fields.len(), 1);
		assert_eq!(form.fields[0].name.to_string(), "name");
		assert_eq!(form.fields[0].field_type.to_string(), "CharField");
	}

	#[test]
	fn test_parse_field_with_properties() {
		let tokens = quote! {
			fields: {
				username: CharField {
					required,
					max_length: 100,
					label: "Username",
				},
			}
		};

		let form: FormMacro = syn::parse2(tokens).unwrap();
		assert_eq!(form.fields.len(), 1);

		let field = &form.fields[0];
		assert!(field.is_required());
		assert_eq!(field.properties.len(), 3);
	}

	#[test]
	fn test_parse_multiple_fields() {
		let tokens = quote! {
			fields: {
				username: CharField {
					required,
					max_length: 150,
				},
				email: EmailField {
					required,
				},
				age: IntegerField {},
			}
		};

		let form: FormMacro = syn::parse2(tokens).unwrap();
		assert_eq!(form.fields.len(), 3);
		assert_eq!(form.fields[0].name.to_string(), "username");
		assert_eq!(form.fields[1].name.to_string(), "email");
		assert_eq!(form.fields[2].name.to_string(), "age");
	}

	#[test]
	fn test_parse_validators() {
		let tokens = quote! {
			fields: {
				username: CharField {},
			},
			validators: {
				username: [
					|v| v.len() >= 3 => "Username must be at least 3 characters",
				],
			}
		};

		let form: FormMacro = syn::parse2(tokens).unwrap();
		assert_eq!(form.validators.len(), 1);

		if let FormValidator::Field {
			field_name, rules, ..
		} = &form.validators[0]
		{
			assert_eq!(field_name.to_string(), "username");
			assert_eq!(rules.len(), 1);
		} else {
			panic!("Expected field validator");
		}
	}

	#[test]
	fn test_parse_form_level_validator() {
		let tokens = quote! {
			fields: {
				password: CharField {},
				confirm: CharField {},
			},
			validators: {
				@form: [
					|data| data["password"] == data["confirm"] => "Passwords must match",
				],
			}
		};

		let form: FormMacro = syn::parse2(tokens).unwrap();
		assert_eq!(form.validators.len(), 1);

		if let FormValidator::Form { rules, .. } = &form.validators[0] {
			assert_eq!(rules.len(), 1);
		} else {
			panic!("Expected form validator");
		}
	}

	#[test]
	fn test_parse_client_validators() {
		let tokens = quote! {
			fields: {
				username: CharField {},
			},
			client_validators: {
				username: [
					"value.length >= 3" => "Username must be at least 3 characters",
				],
			}
		};

		let form: FormMacro = syn::parse2(tokens).unwrap();
		assert_eq!(form.client_validators.len(), 1);
		assert_eq!(form.client_validators[0].rules.len(), 1);
	}

	#[test]
	fn test_parse_widget_property() {
		let tokens = quote! {
			fields: {
				password: CharField {
					widget: PasswordInput,
				},
			}
		};

		let form: FormMacro = syn::parse2(tokens).unwrap();
		let widget = form.fields[0].get_widget();
		assert!(widget.is_some());
		assert_eq!(widget.unwrap().to_string(), "PasswordInput");
	}

	#[test]
	fn test_parse_unknown_section_error() {
		let tokens = quote! {
			unknown_section: {},
			fields: {
				name: CharField {},
			}
		};

		let result: Result<FormMacro> = syn::parse2(tokens);
		assert!(result.is_err());
		let err = result.unwrap_err();
		assert!(err.to_string().contains("unknown section"));
	}
}
