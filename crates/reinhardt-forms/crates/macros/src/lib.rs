//! Procedural macros for reinhardt-forms.
//!
//! This crate provides the `form!` macro for declaratively defining forms.
//!
//! ## Example
//!
//! ```ignore
//! use reinhardt_forms::form;
//!
//! let login_form = form! {
//!     fields: {
//!         username: CharField {
//!             required,
//!             max_length: 150,
//!         },
//!         password: CharField {
//!             required,
//!             widget: PasswordInput,
//!         },
//!     },
//! };
//! ```

use proc_macro::TokenStream;
use quote::quote;
use reinhardt_forms_macros_ast::{FormFieldProperty, FormMacro, FormValidator};

/// Creates a form with the declarative DSL.
///
/// # Syntax
///
/// ```ignore
/// form! {
///     name: "form_name",           // Optional
///     fields: {                     // Required
///         field_name: FieldType {
///             property: value,
///         },
///     },
///     validators: {                 // Optional
///         field_name: [
///             |v| expr => "error message",
///         ],
///         @form: [
///             |data| expr => "error message",
///         ],
///     },
///     client_validators: {          // Optional
///         field_name: [
///             "js_expr" => "error message",
///         ],
///     },
/// }
/// ```
///
/// # Field Types
///
/// - `CharField` - Text input
/// - `EmailField` - Email input with validation
/// - `IntegerField` - Integer input
/// - `FloatField` - Float input
/// - `BooleanField` - Checkbox
/// - `DateField` - Date picker
/// - `DateTimeField` - Date and time picker
/// - `TimeField` - Time picker
/// - `ChoiceField` - Select dropdown
/// - `FileField` - File upload
/// - `ImageField` - Image upload
/// - `PasswordField` - Password input
/// - And more...
///
/// # Field Properties
///
/// - `required` - Field is required (flag)
/// - `max_length: usize` - Maximum length constraint
/// - `min_length: usize` - Minimum length constraint
/// - `label: "text"` - Display label
/// - `help_text: "text"` - Help text
/// - `initial: value` - Initial value
/// - `widget: WidgetType` - Widget specification
#[proc_macro]
pub fn form(input: TokenStream) -> TokenStream {
	let input = proc_macro2::TokenStream::from(input);

	// Parse input using FormMacro from reinhardt-forms-macros-ast
	let ast: FormMacro = match syn::parse2(input) {
		Ok(ast) => ast,
		Err(e) => {
			return TokenStream::from(e.to_compile_error());
		}
	};

	// Generate code
	let output = generate_form_code(&ast);

	TokenStream::from(output)
}

/// Generates Rust code from the parsed form macro AST.
fn generate_form_code(form: &FormMacro) -> proc_macro2::TokenStream {
	// Generate field creation code
	let field_code = generate_fields_code(&form.fields);

	// Generate server-side validators
	let validator_code = generate_validators_code(&form.validators);

	// Generate client-side validators
	let client_validator_code = generate_client_validators_code(&form.client_validators);

	quote! {
		{
			let mut form = reinhardt_forms::Form::new();
			#field_code
			#validator_code
			#client_validator_code
			form
		}
	}
}

/// Generates code for field creation.
fn generate_fields_code(
	fields: &[reinhardt_forms_macros_ast::FormFieldDef],
) -> proc_macro2::TokenStream {
	let field_stmts: Vec<proc_macro2::TokenStream> = fields
		.iter()
		.map(|field| {
			let field_name = field.name.to_string();
			let field_type = &field.field_type;

			// Generate property assignments
			let property_stmts = generate_property_assignments(&field.properties);

			// Generate widget assignment if specified
			let widget_stmt = generate_widget_assignment(&field.properties);

			quote! {
				{
					let mut field = reinhardt_forms::#field_type::new(#field_name.to_string());
					#property_stmts
					#widget_stmt
					form.add_field(Box::new(field));
				}
			}
		})
		.collect();

	quote! {
		#(#field_stmts)*
	}
}

/// Generates property assignment statements for a field.
fn generate_property_assignments(properties: &[FormFieldProperty]) -> proc_macro2::TokenStream {
	let stmts: Vec<proc_macro2::TokenStream> = properties
		.iter()
		.filter_map(|prop| {
			match prop {
				FormFieldProperty::Flag { name, .. } => {
					let name_str = name.to_string();
					// Handle flag properties (e.g., `required`)
					match name_str.as_str() {
						"required" => Some(quote! { field.required = true; }),
						"strip" => Some(quote! { field.strip = true; }),
						_ => None, // Unknown flag, skip
					}
				}
				FormFieldProperty::Named { name, value, .. } => {
					let name_str = name.to_string();
					// Handle named properties (e.g., `max_length: 100`)
					match name_str.as_str() {
						"max_length" => Some(quote! { field.max_length = Some(#value); }),
						"min_length" => Some(quote! { field.min_length = Some(#value); }),
						"label" => Some(quote! { field.label = Some(#value.to_string()); }),
						"help_text" => Some(quote! { field.help_text = Some(#value.to_string()); }),
						"initial" => {
							Some(quote! { field.initial = Some(serde_json::json!(#value)); })
						}
						_ => None, // Unknown property, skip
					}
				}
				FormFieldProperty::Widget { .. } => None, // Handled separately
			}
		})
		.collect();

	quote! {
		#(#stmts)*
	}
}

/// Generates widget assignment if specified.
fn generate_widget_assignment(properties: &[FormFieldProperty]) -> proc_macro2::TokenStream {
	for prop in properties {
		if let FormFieldProperty::Widget { widget_type, .. } = prop {
			let widget_name = widget_type.to_string();
			// Map widget type name to reinhardt_forms::Widget enum variant
			let widget_expr = match widget_name.as_str() {
				"TextInput" => quote! { reinhardt_forms::Widget::TextInput },
				"PasswordInput" => quote! { reinhardt_forms::Widget::PasswordInput },
				"EmailInput" => quote! { reinhardt_forms::Widget::EmailInput },
				"NumberInput" => quote! { reinhardt_forms::Widget::NumberInput },
				"TextArea" | "Textarea" => quote! { reinhardt_forms::Widget::TextArea },
				"CheckboxInput" | "Checkbox" => quote! { reinhardt_forms::Widget::CheckboxInput },
				"DateInput" => quote! { reinhardt_forms::Widget::DateInput },
				"DateTimeInput" => quote! { reinhardt_forms::Widget::DateTimeInput },
				"FileInput" => quote! { reinhardt_forms::Widget::FileInput },
				"HiddenInput" => quote! { reinhardt_forms::Widget::HiddenInput },
				_ => {
					// For Select and RadioSelect, we can't easily get choices from the macro
					// So we use a default empty choices for now
					if widget_name == "Select" {
						quote! { reinhardt_forms::Widget::Select { choices: vec![] } }
					} else if widget_name == "RadioSelect" {
						quote! { reinhardt_forms::Widget::RadioSelect { choices: vec![] } }
					} else {
						// Unknown widget, use TextInput as default
						quote! { reinhardt_forms::Widget::TextInput }
					}
				}
			};
			return quote! { field.widget = #widget_expr; };
		}
	}
	quote! {}
}

/// Generates server-side validator code.
fn generate_validators_code(validators: &[FormValidator]) -> proc_macro2::TokenStream {
	let stmts: Vec<proc_macro2::TokenStream> = validators
		.iter()
		.flat_map(|validator| match validator {
			FormValidator::Field {
				field_name, rules, ..
			} => {
				let field_name_str = field_name.to_string();
				rules
					.iter()
					.map(|rule| {
						let expr = &rule.expr;
						let message = rule.message.value();
						quote! {
							form.add_field_clean_function(#field_name_str, |value| {
								let validator_fn = #expr;
								if validator_fn(value) {
									Ok(value.clone())
								} else {
									Err(reinhardt_forms::FormError::Validation(#message.to_string()))
								}
							});
						}
					})
					.collect::<Vec<_>>()
			}
			FormValidator::Form { rules, .. } => rules
				.iter()
				.map(|rule| {
					let expr = &rule.expr;
					let message = rule.message.value();
					quote! {
						form.add_clean_function(|data| {
							let validator_fn = #expr;
							if validator_fn(data) {
								Ok(())
							} else {
								Err(reinhardt_forms::FormError::Validation(#message.to_string()))
							}
						});
					}
				})
				.collect::<Vec<_>>(),
		})
		.collect();

	quote! {
		#(#stmts)*
	}
}

/// Generates client-side validator code.
fn generate_client_validators_code(
	validators: &[reinhardt_forms_macros_ast::ClientValidator],
) -> proc_macro2::TokenStream {
	let stmts: Vec<proc_macro2::TokenStream> = validators
		.iter()
		.flat_map(|validator| {
			let field_name = validator.field_name.to_string();
			validator.rules.iter().map(move |rule| {
				let js_expr = rule.js_expr.value();
				let message = rule.message.value();
				quote! {
					form.add_client_field_validator(#field_name, #js_expr, #message);
				}
			})
		})
		.collect();

	quote! {
		#(#stmts)*
	}
}
