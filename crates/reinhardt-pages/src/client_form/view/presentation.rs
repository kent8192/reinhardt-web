//! Owned presentation settings and sealed field/widget capabilities.

use std::marker::PhantomData;

/// Rendering primitive selected by a typed presentation adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetKind {
	/// Single-line text.
	TextInput,
	/// Multiline text.
	Textarea,
	/// Email input.
	EmailInput,
	/// URL input.
	UrlInput,
	/// Password input, omitted from SSR values.
	PasswordInput,
	/// Typed primitive number.
	NumberInput,
	/// Boolean checkbox.
	CheckboxInput,
	/// Typed single choice.
	Select,
}

macro_rules! kinds {
    ($($kind:ident),* $(,)?) => {$(
        #[doc = concat!("Capability marker for ", stringify!($kind), " fields.")]
        pub struct $kind;
    )*};
}
kinds!(TextKind, NumberKind, BoolKind, OptionalBoolKind);
/// Capability marker for a required enum.
pub struct EnumKind<T>(PhantomData<T>);
/// Capability marker for an optional enum.
pub struct OptionalEnumKind<T>(PhantomData<T>);

mod sealed {
	pub trait Widget<Kind> {}
	pub trait Placeholder {}
	pub trait OptionalChoice {}
}

/// Closed compatibility table used by generated field adapters.
pub trait AllowedWidget<Kind>: sealed::Widget<Kind> {
	/// Corresponding rendering primitive.
	const KIND: WidgetKind;
}

macro_rules! widgets {
    ($($widget:ident => $kind:ty),* $(,)?) => {$(
        #[doc = concat!("Presentation marker for ", stringify!($widget), ".")]
        pub struct $widget;
        impl sealed::Widget<$kind> for $widget {}
        impl AllowedWidget<$kind> for $widget {
            const KIND: WidgetKind = WidgetKind::$widget;
        }
    )*};
}
widgets!(
	TextInput => TextKind, Textarea => TextKind, EmailInput => TextKind,
	UrlInput => TextKind, PasswordInput => TextKind, NumberInput => NumberKind,
	CheckboxInput => BoolKind, Select => OptionalBoolKind,
);
impl<T> sealed::Widget<EnumKind<T>> for Select {}
impl<T> AllowedWidget<EnumKind<T>> for Select {
	const KIND: WidgetKind = WidgetKind::Select;
}
impl<T> sealed::Widget<OptionalEnumKind<T>> for Select {}
impl<T> AllowedWidget<OptionalEnumKind<T>> for Select {
	const KIND: WidgetKind = WidgetKind::Select;
}
impl sealed::Placeholder for TextKind {}
impl sealed::Placeholder for NumberKind {}
impl sealed::OptionalChoice for OptionalBoolKind {}
impl<T> sealed::OptionalChoice for OptionalEnumKind<T> {}

/// Owned display metadata; optional strings distinguish inheritance from an empty override.
#[derive(Clone, Debug)]
pub struct FieldDisplay {
	/// Control primitive.
	pub widget: WidgetKind,
	/// Visible label.
	pub label: String,
	/// Stable ordinal tokens and labels.
	pub choices: Vec<(String, String)>,
	/// Whether choices include an unset option.
	pub optional_choice: bool,
	/// Whether the two options are boolean values.
	pub boolean_choice: bool,
	/// Explicit accessible name.
	pub aria_label: Option<String>,
	/// Additional described-by IDs.
	pub aria_describedby: Option<String>,
	/// Optional explanatory text.
	pub help_text: Option<String>,
	/// Editor placeholder.
	pub placeholder: Option<String>,
	/// Browser autocomplete hint.
	pub autocomplete: Option<String>,
	/// Control class override.
	pub class: Option<String>,
	/// Wrapper class override.
	pub wrapper_class: Option<String>,
	/// Label class override.
	pub label_class: Option<String>,
	/// Help class override.
	pub help_class: Option<String>,
	/// Error class override.
	pub error_class: Option<String>,
	/// Unset choice label.
	pub empty_label: String,
	/// True choice label.
	pub true_label: String,
	/// False choice label.
	pub false_label: String,
}

impl FieldDisplay {
	/// Creates declaration-derived defaults without reading field values.
	pub fn new(
		label: &str,
		widget: WidgetKind,
		choices: Vec<(String, String)>,
		optional_choice: bool,
		boolean_choice: bool,
	) -> Self {
		Self {
			widget,
			label: label.into(),
			choices,
			optional_choice,
			boolean_choice,
			aria_label: None,
			aria_describedby: None,
			help_text: None,
			placeholder: None,
			autocomplete: None,
			class: None,
			wrapper_class: None,
			label_class: None,
			help_class: None,
			error_class: None,
			empty_label: String::new(),
			true_label: String::from("True"),
			false_label: String::from("False"),
		}
	}
}

/// Typed configuration boundary used only by generated companion methods.
pub struct FieldPresentation<Kind> {
	display: FieldDisplay,
	kind: PhantomData<fn() -> Kind>,
}

macro_rules! optional_setters {
    ($($name:ident),* $(,)?) => {$(
        #[doc = concat!("Overrides ", stringify!($name), " for this field.")]
        pub fn $name(mut self, value: impl Into<String>) -> Self {
            self.display.$name = Some(value.into());
            self
        }
    )*};
}
impl<Kind> FieldPresentation<Kind> {
	/// Wraps the existing display settings for one typed customization.
	pub fn from_display(display: FieldDisplay) -> Self {
		Self {
			display,
			kind: PhantomData,
		}
	}
	/// Returns the customized owned settings.
	pub fn into_display(self) -> FieldDisplay {
		self.display
	}
	/// Selects a widget accepted by this field's capability table.
	pub fn widget<W: AllowedWidget<Kind>>(mut self, _: W) -> Self {
		self.display.widget = W::KIND;
		self
	}
	/// Overrides the visible label.
	pub fn label(mut self, value: impl Into<String>) -> Self {
		self.display.label = value.into();
		self
	}
	optional_setters!(
		aria_label,
		aria_describedby,
		help_text,
		autocomplete,
		class,
		wrapper_class,
		label_class,
		help_class,
		error_class
	);
}
impl<Kind: sealed::Placeholder> FieldPresentation<Kind> {
	optional_setters!(placeholder);
}
impl<Kind: sealed::OptionalChoice> FieldPresentation<Kind> {
	/// Overrides the unset option label.
	pub fn empty_label(mut self, value: impl Into<String>) -> Self {
		self.display.empty_label = value.into();
		self
	}
}
impl FieldPresentation<OptionalBoolKind> {
	/// Overrides the true option label.
	pub fn true_label(mut self, value: impl Into<String>) -> Self {
		self.display.true_label = value.into();
		self
	}
	/// Overrides the false option label.
	pub fn false_label(mut self, value: impl Into<String>) -> Self {
		self.display.false_label = value.into();
		self
	}
}

/// Once-evaluated form-wide settings, separate from the resolved instance ID.
#[derive(Clone, Debug)]
pub struct ViewOptions {
	/// Explicit ID, if supplied.
	pub id: Option<String>,
	/// Form class.
	pub class: String,
	/// Field wrapper class.
	pub field_class: String,
	/// Control class.
	pub input_class: String,
	/// Label class.
	pub label_class: String,
	/// Help class.
	pub help_class: String,
	/// Error class.
	pub error_class: String,
	/// Summary class.
	pub summary_class: String,
	/// Submit button label.
	pub submit_label: String,
	/// Pending submit button label.
	pub pending_label: String,
	/// Submit button class.
	pub submit_class: String,
	/// Validation summary label.
	pub summary_label: String,
}
impl Default for ViewOptions {
	fn default() -> Self {
		Self {
			id: None,
			class: "reinhardt-form".into(),
			field_class: "reinhardt-field".into(),
			input_class: "reinhardt-input".into(),
			label_class: "reinhardt-label".into(),
			help_class: "reinhardt-help".into(),
			error_class: "reinhardt-error".into(),
			summary_class: "reinhardt-validation-summary".into(),
			submit_label: "Submit".into(),
			pending_label: "Submitting...".into(),
			submit_class: "reinhardt-submit".into(),
			summary_label: "Please correct the following errors".into(),
		}
	}
}
macro_rules! option_setters {
    ($($name:ident),* $(,)?) => {$(
        #[doc = concat!("Sets the form-wide ", stringify!($name), ".")]
        pub fn $name(mut self, value: impl Into<String>) -> Self { self.$name = value.into(); self }
    )*};
}
impl ViewOptions {
	/// Sets and validates the explicit form ID before constructing bindings.
	pub fn id(mut self, value: impl Into<String>) -> Self {
		let id = value.into();
		assert!(
			!id.is_empty() && !id.chars().any(|ch| ch.is_ascii_whitespace()),
			"ClientForm view id must be nonempty and contain no ASCII whitespace"
		);
		self.id = Some(id);
		self
	}
	option_setters!(
		class,
		field_class,
		input_class,
		label_class,
		help_class,
		error_class,
		summary_class,
		submit_label,
		pending_label,
		submit_class,
		summary_label
	);
	/// Resolves one instance ID using the existing request-scoped ID hook.
	pub fn resolve_id(&self) -> String {
		if let Some(id) = &self.id {
			assert!(
				!id.is_empty() && !id.chars().any(|ch| ch.is_ascii_whitespace()),
				"ClientForm view id must be nonempty and contain no ASCII whitespace"
			);
			id.clone()
		} else {
			crate::reactive::hooks::id::use_id_with_prefix("client-form")
		}
	}
}
