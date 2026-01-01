//! Combination tests for the `form!` macro.
//!
//! Tests various combinations of features to ensure they work together.

use quote::quote;
use reinhardt_forms_macros_ast::{FormMacro, FormValidator};
use rstest::rstest;

/// CB-001: Field type × property combinations.
///
/// Tests that each field type works with all applicable properties.
#[rstest]
#[case("CharField", vec!["required", "max_length", "min_length", "label", "help_text", "initial"])]
#[case("EmailField", vec!["required", "label", "help_text"])]
#[case("IntegerField", vec!["required", "min_value", "max_value", "label"])]
#[case("BooleanField", vec!["label", "help_text", "initial"])]
#[case("DateField", vec!["required", "label", "help_text"])]
#[case("FileField", vec!["required", "label", "help_text"])]
fn test_field_type_property_combinations(#[case] field_type: &str, #[case] properties: Vec<&str>) {
	let field_type_ident = syn::Ident::new(field_type, proc_macro2::Span::call_site());

	let mut property_tokens = Vec::new();
	for prop in &properties {
		match *prop {
			"required" => property_tokens.push(quote! { required }),
			"max_length" => property_tokens.push(quote! { max_length: 100 }),
			"min_length" => property_tokens.push(quote! { min_length: 1 }),
			"max_value" => property_tokens.push(quote! { max_value: 1000 }),
			"min_value" => property_tokens.push(quote! { min_value: 0 }),
			"label" => property_tokens.push(quote! { label: "Label" }),
			"help_text" => property_tokens.push(quote! { help_text: "Help" }),
			"initial" => match field_type {
				"BooleanField" => property_tokens.push(quote! { initial: true }),
				"IntegerField" => property_tokens.push(quote! { initial: 0 }),
				_ => property_tokens.push(quote! { initial: "default" }),
			},
			_ => {}
		}
	}

	let tokens = quote! {
		fields: {
			field: #field_type_ident {
				#(#property_tokens,)*
			},
		}
	};

	let result: syn::Result<FormMacro> = syn::parse2(tokens);
	assert!(
		result.is_ok(),
		"{} with properties {:?} should parse successfully",
		field_type,
		properties
	);
}

/// CB-002: CSRF × validators combination.
///
/// Tests that all validator types work correctly together.
#[rstest]
#[case(true, true, true)] // field + form + client validators
#[case(true, false, false)] // field validators only
#[case(false, true, false)] // form validators only
#[case(true, true, false)] // field + form validators
fn test_validators_combination(
	#[case] field_validators: bool,
	#[case] form_validators: bool,
	#[case] client_validators: bool,
) {
	let validators_tokens = match (field_validators, form_validators) {
		(true, true) => quote! {
			validators: {
				username: [
					|v| v.len() >= 3 => "Too short",
				],
				@form: [
					|data| true => "Form validation",
				],
			},
		},
		(true, false) => quote! {
			validators: {
				username: [
					|v| v.len() >= 3 => "Too short",
				],
			},
		},
		(false, true) => quote! {
			validators: {
				@form: [
					|data| true => "Form validation",
				],
			},
		},
		(false, false) => quote! {},
	};

	let client_val_tokens = if client_validators {
		quote! {
			client_validators: {
				username: [
					"value.length >= 3" => "Too short",
				],
			},
		}
	} else {
		quote! {}
	};

	let tokens = quote! {
		fields: {
			username: CharField {
				required,
			},
		},
		#validators_tokens
		#client_val_tokens
	};

	let result: syn::Result<FormMacro> = syn::parse2(tokens);
	assert!(
		result.is_ok(),
		"field_val={}, form_val={}, client_val={} should parse",
		field_validators,
		form_validators,
		client_validators
	);

	let _form = result.unwrap();
}

/// CB-003: Required × initial combination.
///
/// Tests that required fields can have initial values.
#[rstest]
#[case(true, true)] // Required with initial
#[case(true, false)] // Required without initial
#[case(false, true)] // Optional with initial
#[case(false, false)] // Optional without initial
fn test_required_initial_combination(#[case] required: bool, #[case] has_initial: bool) {
	let tokens = match (required, has_initial) {
		(true, true) => quote! {
			fields: {
				username: CharField {
					required,
					initial: "default",
				},
			}
		},
		(true, false) => quote! {
			fields: {
				username: CharField {
					required,
				},
			}
		},
		(false, true) => quote! {
			fields: {
				username: CharField {
					initial: "default",
				},
			}
		},
		(false, false) => quote! {
			fields: {
				username: CharField {},
			}
		},
	};

	let result: syn::Result<FormMacro> = syn::parse2(tokens);
	assert!(
		result.is_ok(),
		"required={}, initial={} should parse",
		required,
		has_initial
	);

	let form = result.unwrap();
	assert_eq!(form.fields[0].is_required(), required);
	assert_eq!(
		form.fields[0].get_property("initial").is_some(),
		has_initial
	);
}

/// CB-004: Multiple validators per field.
///
/// Tests that a field can have multiple validators of different types.
#[rstest]
fn test_multiple_validators_per_field() {
	let tokens = quote! {
		fields: {
			username: CharField {
				required,
				max_length: 150,
			},
		},
		validators: {
			username: [
				|v| v.len() >= 3 => "Too short",
				|v| v.len() <= 150 => "Too long",
				|v| !v.contains(' ') => "No spaces allowed",
				|v| v.chars().all(|c| c.is_alphanumeric() || c == '_') => "Invalid characters",
			],
		},
		client_validators: {
			username: [
				"value.length >= 3" => "Too short",
				"!/\\s/.test(value)" => "No spaces allowed",
			],
		}
	};

	let result: syn::Result<FormMacro> = syn::parse2(tokens);
	assert!(result.is_ok(), "Multiple validators per field should parse");

	let form = result.unwrap();

	// Check server-side validators
	let server_validator = form
		.validators
		.iter()
		.find(|v| matches!(v, FormValidator::Field { field_name, .. } if field_name == "username"));
	assert!(server_validator.is_some());

	if let Some(FormValidator::Field { rules, .. }) = server_validator {
		assert_eq!(rules.len(), 4, "Should have 4 server-side validator rules");
	}

	// Check client-side validators
	let client_validator = form
		.client_validators
		.iter()
		.find(|v| v.field_name == "username");
	assert!(client_validator.is_some());
	assert_eq!(
		client_validator.unwrap().rules.len(),
		2,
		"Should have 2 client-side validator rules"
	);
}

/// CB-005: Widget × field type compatibility.
///
/// Tests compatible widget and field type combinations.
#[rstest]
#[case("CharField", "TextInput")]
#[case("CharField", "PasswordInput")]
#[case("CharField", "TextArea")]
#[case("CharField", "HiddenInput")]
#[case("EmailField", "EmailInput")]
#[case("IntegerField", "NumberInput")]
#[case("BooleanField", "Checkbox")]
#[case("ChoiceField", "Select")]
#[case("ChoiceField", "RadioSelect")]
#[case("MultipleChoiceField", "CheckboxSelectMultiple")]
#[case("DateField", "DateInput")]
#[case("TimeField", "TimeInput")]
#[case("DateTimeField", "DateTimeInput")]
#[case("FileField", "FileInput")]
#[case("URLField", "UrlInput")]
fn test_widget_field_type_compatibility(#[case] field_type: &str, #[case] widget: &str) {
	let field_type_ident = syn::Ident::new(field_type, proc_macro2::Span::call_site());
	let widget_ident = syn::Ident::new(widget, proc_macro2::Span::call_site());

	let tokens = quote! {
		fields: {
			field: #field_type_ident {
				widget: #widget_ident,
			},
		}
	};

	let result: syn::Result<FormMacro> = syn::parse2(tokens);
	assert!(
		result.is_ok(),
		"{} with {} widget should be compatible",
		field_type,
		widget
	);

	let form = result.unwrap();
	assert_eq!(form.fields[0].field_type.to_string(), field_type);
	assert!(form.fields[0].get_widget().is_some());
	assert_eq!(form.fields[0].get_widget().unwrap().to_string(), widget);
}
