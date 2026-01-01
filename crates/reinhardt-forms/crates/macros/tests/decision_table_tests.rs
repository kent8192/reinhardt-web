//! Decision table tests for the `form!` macro.
//!
//! Tests various combinations of conditions using decision tables.

use quote::quote;
use reinhardt_forms_macros_ast::{FormFieldProperty, FormMacro};
use rstest::rstest;

/// Helper to check if a field is required in the parsed form.
fn is_field_required(form: &FormMacro, field_name: &str) -> bool {
	form.fields
		.iter()
		.find(|f| f.name == field_name)
		.map(|f| f.is_required())
		.unwrap_or(false)
}

/// Helper to get a property value from a parsed form field.
fn has_property(form: &FormMacro, field_name: &str, property_name: &str) -> bool {
	form.fields
		.iter()
		.find(|f| f.name == field_name)
		.map(|f| {
			f.properties.iter().any(|p| match p {
				FormFieldProperty::Named { name, .. } => name == property_name,
				FormFieldProperty::Flag { name, .. } => name == property_name,
				FormFieldProperty::Widget { .. } => property_name == "widget",
			})
		})
		.unwrap_or(false)
}

/// DT-001 to DT-006: Required field decision table.
///
/// Tests the parsing of required/optional field configurations.
/// Runtime validation behavior will be tested in integration tests.
///
/// | Case | Required | Initial | Expected |
/// |------|----------|---------|----------|
/// | DT-001 | true | Some | valid parse |
/// | DT-002 | true | None | valid parse |
/// | DT-003 | false | Some | valid parse |
/// | DT-004 | false | None | valid parse |
/// | DT-005 | true | empty | valid parse |
/// | DT-006 | false | empty | valid parse |
#[rstest]
#[case(true, Some("value"), true)] // DT-001
#[case(true, None, true)] // DT-002
#[case(false, Some("value"), true)] // DT-003
#[case(false, None, true)] // DT-004
#[case(true, Some(""), true)] // DT-005
#[case(false, Some(""), true)] // DT-006
fn test_required_initial_decision_table(
	#[case] required: bool,
	#[case] initial: Option<&str>,
	#[case] should_parse: bool,
) {
	let tokens = match (required, initial) {
		(true, Some(val)) => quote! {
			fields: {
				field: CharField {
					required,
					initial: #val,
				},
			}
		},
		(true, None) => quote! {
			fields: {
				field: CharField {
					required,
				},
			}
		},
		(false, Some(val)) => quote! {
			fields: {
				field: CharField {
					initial: #val,
				},
			}
		},
		(false, None) => quote! {
			fields: {
				field: CharField {},
			}
		},
	};

	let result: syn::Result<FormMacro> = syn::parse2(tokens);
	assert_eq!(
		result.is_ok(),
		should_parse,
		"required={}, initial={:?} should {} parse",
		required,
		initial,
		if should_parse {
			"successfully"
		} else {
			"fail to"
		}
	);

	if result.is_ok() {
		let form = result.unwrap();
		assert_eq!(is_field_required(&form, "field"), required);
		assert_eq!(has_property(&form, "field", "initial"), initial.is_some());
	}
}

/// Decision table for validators combinations.
///
/// | Case | Field Val | Form Val | Client Val | Expected |
/// |------|-----------|----------|------------|----------|
/// | 1 | true | true | true | valid |
/// | 2 | true | true | false | valid |
/// | 3 | true | false | false | valid |
/// | 4 | false | false | false | valid |
/// | 5 | false | true | true | valid |
#[rstest]
#[case(true, true, true, true)]
#[case(true, true, false, true)]
#[case(true, false, false, true)]
#[case(false, false, false, true)]
#[case(false, true, true, true)]
fn test_validators_decision_table(
	#[case] field_validators: bool,
	#[case] form_validators: bool,
	#[case] client_validators: bool,
	#[case] should_parse: bool,
) {
	let field_val_tokens = if field_validators {
		quote! {
			validators: {
				username: [
					|v| v.len() >= 3 => "Too short",
				],
			},
		}
	} else {
		quote! {}
	};

	let form_val_tokens = if form_validators {
		quote! {
			validators: {
				@form: [
					|data| true => "Form validation",
				],
			},
		}
	} else {
		quote! {}
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

	// Note: we need to handle the case where both field and form validators are present
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
		(true, false) => field_val_tokens,
		(false, true) => form_val_tokens,
		(false, false) => quote! {},
	};

	let tokens = quote! {
		fields: {
			username: CharField {},
		},
		#validators_tokens
		#client_val_tokens
	};

	let result: syn::Result<FormMacro> = syn::parse2(tokens);
	assert_eq!(
		result.is_ok(),
		should_parse,
		"field_val={}, form_val={}, client_val={} should {} parse",
		field_validators,
		form_validators,
		client_validators,
		if should_parse {
			"successfully"
		} else {
			"fail to"
		}
	);

	if result.is_ok() {
		let _form = result.unwrap();
		// Validator counts depend on the specific combination
	}
}

/// Decision table for widget and field type compatibility.
///
/// | Case | Field Type | Widget | Expected |
/// |------|------------|--------|----------|
/// | 1 | CharField | TextInput | valid |
/// | 2 | CharField | PasswordInput | valid |
/// | 3 | CharField | TextArea | valid |
/// | 4 | IntegerField | NumberInput | valid |
/// | 5 | BooleanField | Checkbox | valid |
/// | 6 | ChoiceField | Select | valid |
/// | 7 | ChoiceField | RadioSelect | valid |
/// | 8 | FileField | FileInput | valid |
#[rstest]
#[case("CharField", "TextInput", true)]
#[case("CharField", "PasswordInput", true)]
#[case("CharField", "TextArea", true)]
#[case("IntegerField", "NumberInput", true)]
#[case("BooleanField", "Checkbox", true)]
#[case("ChoiceField", "Select", true)]
#[case("ChoiceField", "RadioSelect", true)]
#[case("FileField", "FileInput", true)]
fn test_widget_field_type_decision_table(
	#[case] field_type: &str,
	#[case] widget: &str,
	#[case] should_parse: bool,
) {
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
	assert_eq!(
		result.is_ok(),
		should_parse,
		"{} with {} widget should {} parse",
		field_type,
		widget,
		if should_parse {
			"successfully"
		} else {
			"fail to"
		}
	);

	if result.is_ok() {
		let form = result.unwrap();
		assert_eq!(form.fields[0].field_type.to_string(), field_type);
		assert!(form.fields[0].get_widget().is_some());
		assert_eq!(form.fields[0].get_widget().unwrap().to_string(), widget);
	}
}
