//! Untyped AST node definitions for the `form!` macro.
//!
//! These structures represent the raw parse output before semantic validation.

use proc_macro2::Span;
use syn::{Expr, ExprClosure, Ident, LitStr};

/// Top-level form macro AST.
///
/// Represents the entire `form! { ... }` invocation.
#[derive(Debug, Clone)]
pub struct FormMacro {
	/// Optional form name (e.g., `name: "login_form"`)
	pub name: Option<LitStr>,
	/// Field definitions
	pub fields: Vec<FormFieldDef>,
	/// Server-side validators
	pub validators: Vec<FormValidator>,
	/// Client-side validators (JavaScript expressions)
	pub client_validators: Vec<ClientValidator>,
	/// Span for error reporting
	pub span: Span,
}

/// A single field definition in the form macro.
///
/// Example:
/// ```ignore
/// username: CharField {
///     required,
///     max_length: 100,
///     label: "Username",
/// }
/// ```
#[derive(Debug, Clone)]
pub struct FormFieldDef {
	/// Field name identifier
	pub name: Ident,
	/// Field type identifier (e.g., CharField, EmailField)
	pub field_type: Ident,
	/// Field properties
	pub properties: Vec<FormFieldProperty>,
	/// Span for error reporting
	pub span: Span,
}

/// A property within a field definition.
#[derive(Debug, Clone)]
pub enum FormFieldProperty {
	/// Named property with a value: `max_length: 100`
	Named {
		name: Ident,
		value: Expr,
		span: Span,
	},
	/// Flag property (boolean true): `required`
	Flag { name: Ident, span: Span },
	/// Widget specification: `widget: PasswordInput`
	Widget { widget_type: Ident, span: Span },
}

impl FormFieldProperty {
	/// Returns the property name.
	pub fn name(&self) -> &Ident {
		match self {
			FormFieldProperty::Named { name, .. } => name,
			FormFieldProperty::Flag { name, .. } => name,
			FormFieldProperty::Widget { .. } => {
				// Widget is a special case, we return a synthetic ident
				panic!("Widget property has no direct name")
			}
		}
	}

	/// Returns the span for error reporting.
	pub fn span(&self) -> Span {
		match self {
			FormFieldProperty::Named { span, .. } => *span,
			FormFieldProperty::Flag { span, .. } => *span,
			FormFieldProperty::Widget { span, .. } => *span,
		}
	}
}

/// Server-side validator definition.
#[derive(Debug, Clone)]
pub enum FormValidator {
	/// Field-level validator: `username: [|v| ... => "error"]`
	Field {
		field_name: Ident,
		rules: Vec<ValidatorRule>,
		span: Span,
	},
	/// Form-level validator: `@form: [|data| ... => "error"]`
	Form {
		rules: Vec<ValidatorRule>,
		span: Span,
	},
}

/// A single validation rule with closure and error message.
#[derive(Debug, Clone)]
pub struct ValidatorRule {
	/// Validation closure expression
	pub expr: ExprClosure,
	/// Error message when validation fails
	pub message: LitStr,
	/// Span for error reporting
	pub span: Span,
}

/// Client-side validator definition (JavaScript expressions).
#[derive(Debug, Clone)]
pub struct ClientValidator {
	/// Field name to validate
	pub field_name: Ident,
	/// Validation rules
	pub rules: Vec<ClientValidatorRule>,
	/// Span for error reporting
	pub span: Span,
}

/// A single client-side validation rule.
#[derive(Debug, Clone)]
pub struct ClientValidatorRule {
	/// JavaScript expression for validation
	pub js_expr: LitStr,
	/// Error message when validation fails
	pub message: LitStr,
	/// Span for error reporting
	pub span: Span,
}

impl FormMacro {
	/// Creates a new empty FormMacro with the given span.
	pub fn new(span: Span) -> Self {
		Self {
			name: None,
			fields: Vec::new(),
			validators: Vec::new(),
			client_validators: Vec::new(),
			span,
		}
	}
}

impl FormFieldDef {
	/// Creates a new field definition.
	pub fn new(name: Ident, field_type: Ident, span: Span) -> Self {
		Self {
			name,
			field_type,
			properties: Vec::new(),
			span,
		}
	}

	/// Returns true if this field has the `required` flag.
	pub fn is_required(&self) -> bool {
		self.properties
			.iter()
			.any(|p| matches!(p, FormFieldProperty::Flag { name, .. } if name == "required"))
	}

	/// Gets a named property value by name.
	pub fn get_property(&self, prop_name: &str) -> Option<&Expr> {
		self.properties.iter().find_map(|p| {
			if let FormFieldProperty::Named { name, value, .. } = p
				&& name == prop_name
			{
				return Some(value);
			}
			None
		})
	}

	/// Gets the widget type if specified.
	pub fn get_widget(&self) -> Option<&Ident> {
		self.properties.iter().find_map(|p| {
			if let FormFieldProperty::Widget { widget_type, .. } = p {
				Some(widget_type)
			} else {
				None
			}
		})
	}
}
