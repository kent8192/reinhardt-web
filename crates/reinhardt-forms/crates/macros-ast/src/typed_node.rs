//! Typed AST node definitions for the `form!` macro.
//!
//! These structures represent validated AST after semantic analysis.
//! They contain resolved types and checked references.

use proc_macro2::{Span, TokenStream};
use syn::{ExprClosure, Ident, LitStr};

/// Validated form macro AST.
#[derive(Debug, Clone)]
pub struct TypedFormMacro {
	/// Optional form name
	pub name: Option<String>,
	/// Validated field definitions
	pub fields: Vec<TypedFormFieldDef>,
	/// Validated server-side validators
	pub validators: Vec<TypedFormValidator>,
	/// Validated client-side validators
	pub client_validators: Vec<TypedClientValidator>,
	/// Span for error reporting
	pub span: Span,
}

/// Known form field types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormFieldKind {
	// Text fields
	Char,
	Email,
	Url,
	Slug,
	Regex,

	// Numeric fields
	Integer,
	Float,
	Decimal,

	// Boolean
	Boolean,

	// Date/Time fields
	Date,
	DateTime,
	Time,
	Duration,

	// Choice fields
	Choice,
	MultipleChoice,
	ModelChoice,
	ModelMultipleChoice,

	// File fields
	File,
	Image,

	// Other fields
	Json,
	Uuid,
	Color,
	Password,
	ComboField,

	// Custom/unknown field type
	Custom,
}

impl FormFieldKind {
	/// Parses a field type from its identifier name.
	pub fn from_ident(ident: &Ident) -> Self {
		match ident.to_string().as_str() {
			"CharField" => Self::Char,
			"EmailField" => Self::Email,
			"URLField" | "UrlField" => Self::Url,
			"SlugField" => Self::Slug,
			"RegexField" => Self::Regex,
			"IntegerField" => Self::Integer,
			"FloatField" => Self::Float,
			"DecimalField" => Self::Decimal,
			"BooleanField" => Self::Boolean,
			"DateField" => Self::Date,
			"DateTimeField" => Self::DateTime,
			"TimeField" => Self::Time,
			"DurationField" => Self::Duration,
			"ChoiceField" => Self::Choice,
			"MultipleChoiceField" => Self::MultipleChoice,
			"ModelChoiceField" => Self::ModelChoice,
			"ModelMultipleChoiceField" => Self::ModelMultipleChoice,
			"FileField" => Self::File,
			"ImageField" => Self::Image,
			"JSONField" | "JsonField" => Self::Json,
			"UUIDField" | "UuidField" => Self::Uuid,
			"ColorField" => Self::Color,
			"PasswordField" => Self::Password,
			"ComboField" => Self::ComboField,
			_ => Self::Custom,
		}
	}

	/// Returns the Rust type name for this field kind.
	pub fn rust_type_name(&self) -> &'static str {
		match self {
			Self::Char => "CharField",
			Self::Email => "EmailField",
			Self::Url => "URLField",
			Self::Slug => "SlugField",
			Self::Regex => "RegexField",
			Self::Integer => "IntegerField",
			Self::Float => "FloatField",
			Self::Decimal => "DecimalField",
			Self::Boolean => "BooleanField",
			Self::Date => "DateField",
			Self::DateTime => "DateTimeField",
			Self::Time => "TimeField",
			Self::Duration => "DurationField",
			Self::Choice => "ChoiceField",
			Self::MultipleChoice => "MultipleChoiceField",
			Self::ModelChoice => "ModelChoiceField",
			Self::ModelMultipleChoice => "ModelMultipleChoiceField",
			Self::File => "FileField",
			Self::Image => "ImageField",
			Self::Json => "JSONField",
			Self::Uuid => "UUIDField",
			Self::Color => "ColorField",
			Self::Password => "PasswordField",
			Self::ComboField => "ComboField",
			Self::Custom => "CustomField",
		}
	}
}

/// Known widget types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetKind {
	TextInput,
	TextArea,
	PasswordInput,
	NumberInput,
	EmailInput,
	UrlInput,
	DateInput,
	TimeInput,
	DateTimeInput,
	Checkbox,
	Select,
	RadioSelect,
	CheckboxSelectMultiple,
	FileInput,
	HiddenInput,
	Custom,
}

impl WidgetKind {
	/// Parses a widget type from its identifier name.
	pub fn from_ident(ident: &Ident) -> Self {
		match ident.to_string().as_str() {
			"TextInput" => Self::TextInput,
			"TextArea" | "Textarea" => Self::TextArea,
			"PasswordInput" => Self::PasswordInput,
			"NumberInput" => Self::NumberInput,
			"EmailInput" => Self::EmailInput,
			"UrlInput" | "URLInput" => Self::UrlInput,
			"DateInput" => Self::DateInput,
			"TimeInput" => Self::TimeInput,
			"DateTimeInput" => Self::DateTimeInput,
			"Checkbox" | "CheckboxInput" => Self::Checkbox,
			"Select" => Self::Select,
			"RadioSelect" => Self::RadioSelect,
			"CheckboxSelectMultiple" => Self::CheckboxSelectMultiple,
			"FileInput" => Self::FileInput,
			"HiddenInput" => Self::HiddenInput,
			_ => Self::Custom,
		}
	}
}

/// Validated field definition.
#[derive(Debug, Clone)]
pub struct TypedFormFieldDef {
	/// Field name as string
	pub name: String,
	/// Original field name identifier (for code generation)
	pub name_ident: Ident,
	/// Resolved field type
	pub field_type: FormFieldKind,
	/// Original field type identifier (for code generation)
	pub field_type_ident: Ident,
	/// Whether the field is required
	pub required: bool,
	/// Label text
	pub label: Option<String>,
	/// Help text
	pub help_text: Option<String>,
	/// Maximum length constraint
	pub max_length: Option<usize>,
	/// Minimum length constraint
	pub min_length: Option<usize>,
	/// Initial value expression
	pub initial: Option<TokenStream>,
	/// Widget type
	pub widget: Option<WidgetKind>,
	/// Widget type identifier (for custom widgets)
	pub widget_ident: Option<Ident>,
	/// Inline validators defined within the field
	pub inline_validators: Vec<TypedValidatorRule>,
	/// Span for error reporting
	pub span: Span,
}

/// Validated server-side validator.
#[derive(Debug, Clone)]
pub enum TypedFormValidator {
	/// Field-level validator
	Field {
		field_name: String,
		rules: Vec<TypedValidatorRule>,
		span: Span,
	},
	/// Form-level validator
	Form {
		rules: Vec<TypedValidatorRule>,
		span: Span,
	},
}

/// Validated validation rule.
#[derive(Debug, Clone)]
pub struct TypedValidatorRule {
	/// Validation closure
	pub expr: ExprClosure,
	/// Error message
	pub message: String,
	/// Span for error reporting
	pub span: Span,
}

/// Validated client-side validator.
#[derive(Debug, Clone)]
pub struct TypedClientValidator {
	/// Field name
	pub field_name: String,
	/// Validation rules
	pub rules: Vec<TypedClientValidatorRule>,
	/// Span for error reporting
	pub span: Span,
}

/// Validated client-side validation rule.
#[derive(Debug, Clone)]
pub struct TypedClientValidatorRule {
	/// JavaScript expression
	pub js_expr: String,
	/// Error message
	pub message: String,
	/// Original LitStr for code generation
	pub js_expr_lit: LitStr,
	/// Span for error reporting
	pub span: Span,
}

impl TypedFormMacro {
	/// Creates a new empty TypedFormMacro with the given span.
	pub fn new(span: Span) -> Self {
		Self {
			name: None,
			fields: Vec::new(),
			validators: Vec::new(),
			client_validators: Vec::new(),
			span,
		}
	}

	/// Returns the field names in this form.
	pub fn field_names(&self) -> Vec<&str> {
		self.fields.iter().map(|f| f.name.as_str()).collect()
	}

	/// Checks if a field with the given name exists.
	pub fn has_field(&self, name: &str) -> bool {
		self.fields.iter().any(|f| f.name == name)
	}
}
