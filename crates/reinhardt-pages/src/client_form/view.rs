//! Presentation descriptors borrowing a named form's application-owned runtime.

use crate::component::{ControlBinding, ControlKind, IntoPage, Page, PageElement};
use crate::reactive::Signal;
use crate::{FormRuntimeSource, FormServerMutation, RuntimeControlBindingRequest, UseFormReturn};
use reinhardt_core::types::page::EventType;

mod errors;
mod presentation;
pub use errors::{SummaryEntry, collect_summary, describedby};
pub use presentation::*;

/// One exposed DTO field and its binding to the existing runtime.
pub struct ViewField<Field> {
	/// Generated form field token.
	pub key: Field,
	/// Stable declaration ordinal.
	pub ordinal: usize,
	/// Rust identifier without a raw prefix.
	pub rust_name: &'static str,
	/// Serde serialization name.
	pub serialized_name: &'static str,
	/// Safe required hint derived from opted-in string validation.
	pub required: bool,
	/// Once-evaluated display settings.
	pub presentation: FieldDisplay,
	/// Typed control source, without a duplicate editor signal.
	pub binding: ControlBinding,
}

/// Resolves a generated field's binding using the runtime's existing adapter.
pub fn view_binding<Form, Deps>(
	runtime: &UseFormReturn<Form, Deps>,
	field: Form::Field,
	kind: ControlKind,
) -> ControlBinding
where
	Form: FormRuntimeSource,
	Deps: Clone + PartialEq + 'static,
{
	runtime
		.runtime_control_binding(
			field,
			RuntimeControlBindingRequest {
				kind,
				radio_value: None,
			},
		)
		.unwrap_or_else(|| panic!("ClientForm view field {field:?} has no {kind} binding"))
}

/// Shared retained-view descriptor behind each generated, DTO-specific wrapper.
pub struct ClientFormView<Form, Deps, Input, Output>
where
	Form: FormRuntimeSource,
	Deps: Clone + PartialEq + 'static,
	Input: 'static,
	Output: Clone + 'static,
{
	mutation: FormServerMutation<Form, Deps, Input, Output>,
	runtime: UseFormReturn<Form, Deps>,
	fields: Vec<ViewField<Form::Field>>,
	options: ViewOptions,
	id: String,
}

impl<Form, Deps, Input, Output> ClientFormView<Form, Deps, Input, Output>
where
	Form: FormRuntimeSource,
	Deps: Clone + PartialEq + 'static,
	Input: 'static,
	Output: Clone + 'static,
{
	/// Assembles metadata and a shared mutation handle without creating a runtime.
	pub fn new(
		mutation: FormServerMutation<Form, Deps, Input, Output>,
		runtime: UseFormReturn<Form, Deps>,
		fields: Vec<ViewField<Form::Field>>,
		options: ViewOptions,
		id: String,
	) -> Self {
		Self {
			mutation,
			runtime,
			fields,
			options,
			id,
		}
	}
	/// Applies one typed field customization exactly once.
	pub fn customize<Kind>(
		mut self,
		ordinal: usize,
		configure: impl FnOnce(FieldPresentation<Kind>) -> FieldPresentation<Kind>,
	) -> Self {
		let display = self.fields[ordinal].presentation.clone();
		self.fields[ordinal].presentation =
			configure(FieldPresentation::from_display(display)).into_display();
		self
	}

	/// Builds retained controls and delegates submission to the existing mutation.
	pub fn into_page(self) -> Page {
		let Self {
			mutation,
			runtime,
			fields,
			options,
			id,
		} = self;
		let state = runtime.form_state();
		let ordered_fields: Vec<_> = fields
			.iter()
			.map(|field| (field.key, format!("{id}-field-{}", field.ordinal)))
			.collect();
		let summary_label = options.summary_label.clone();
		let summary = PageElement::new("div")
			.attr("id", format!("{id}-summary"))
			.attr("class", options.summary_class.clone())
			.attr("aria-live", "polite")
			.attr("aria-atomic", "true")
			.child(Page::reactive(move || {
				let entries = collect_summary(
					&ordered_fields,
					&tracked_or_default(state.field_errors),
					tracked_or_default(state.form_error).as_deref(),
					tracked_or_default(state.submit_error).as_deref(),
				);
				if entries.is_empty() {
					return Page::empty();
				}
				Page::fragment([
					PageElement::new("p")
						.child(Page::text(summary_label.clone()))
						.into_page(),
					PageElement::new("ul")
						.children(entries.into_iter().map(summary_item))
						.into_page(),
				])
			}));
		let pending_button = mutation.clone();
		let pending_label = mutation.clone();
		let pending_form = mutation.clone();
		let button = PageElement::new("button")
			.attr("type", "submit")
			.attr("class", options.submit_class.clone())
			.reactive_attr("disabled", move || {
				pending_button.is_pending().then(|| "disabled".into())
			})
			.child(Page::reactive({
				let normal = options.submit_label.clone();
				let pending = options.pending_label.clone();
				move || {
					Page::text(if pending_label.is_pending() {
						pending.clone()
					} else {
						normal.clone()
					})
				}
			}));
		PageElement::new("form")
			.attr("id", id.clone())
			.attr("class", options.class.clone())
			.bool_attr("novalidate", true)
			.reactive_attr("aria-busy", move || {
				Some(
					if pending_form.is_pending() {
						"true"
					} else {
						"false"
					}
					.into(),
				)
			})
			.on(
				EventType::Submit,
				crate::raw_event_handler(move |event| {
					event.prevent_default();
					mutation.dispatch();
				}),
			)
			.child(summary)
			.children(
				fields
					.into_iter()
					.map(|field| render_field(field, &runtime, &options, &id)),
			)
			.child(button)
			.into_page()
	}
}

fn tracked_or_default<T: Clone + Default + 'static>(signal: Signal<T>) -> T {
	if signal.try_get_untracked().is_ok() {
		signal.get()
	} else {
		T::default()
	}
}

fn summary_item(entry: SummaryEntry) -> Page {
	let content = if let Some(id) = entry.control_id {
		PageElement::new("a")
			.attr("href", format!("#{id}"))
			.on(
				EventType::Click,
				crate::raw_event_handler(move |event| {
					event.prevent_default();
					focus_control(&id);
				}),
			)
			.child(Page::text(entry.message))
			.into_page()
	} else {
		Page::text(entry.message)
	};
	PageElement::new("li").child(content).into_page()
}

fn focus_control(id: &str) {
	#[cfg(wasm)]
	{
		use wasm_bindgen::JsCast;
		if let Some(control) = web_sys::window()
			.and_then(|window| window.document())
			.and_then(|document| document.get_element_by_id(id))
			.and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
		{
			let _ = control.focus();
		}
	}
	#[cfg(not(wasm))]
	let _ = id;
}

fn render_field<Form, Deps>(
	field: ViewField<Form::Field>,
	runtime: &UseFormReturn<Form, Deps>,
	options: &ViewOptions,
	form_id: &str,
) -> Page
where
	Form: FormRuntimeSource,
	Deps: Clone + PartialEq + 'static,
{
	let ViewField {
		key,
		ordinal,
		serialized_name,
		required,
		presentation: display,
		binding,
		..
	} = field;
	let id = format!("{form_id}-field-{ordinal}");
	let help_id = format!("{form_id}-help-{ordinal}");
	let error_id = format!("{form_id}-error-{ordinal}");
	let errors = runtime.form_state().field_errors;
	let mut control =
		match display.widget {
			WidgetKind::Textarea => PageElement::new("textarea"),
			WidgetKind::Select => {
				let mut select = PageElement::new("select");
				if display.optional_choice {
					select = select.child(
						PageElement::new("option")
							.attr("value", "none")
							.child(Page::text(display.empty_label.clone())),
					);
				}
				select.children(display.choices.iter().enumerate().map(
					|(index, (token, label))| {
						let label = if display.boolean_choice {
							if index == 0 {
								&display.false_label
							} else {
								&display.true_label
							}
						} else {
							label
						};
						PageElement::new("option")
							.attr("value", token.clone())
							.child(Page::text(label.clone()))
					},
				))
			}
			widget => {
				let input_type = match widget {
					WidgetKind::TextInput => "text",
					WidgetKind::EmailInput => "email",
					WidgetKind::UrlInput => "url",
					WidgetKind::PasswordInput => "password",
					WidgetKind::NumberInput => "number",
					WidgetKind::CheckboxInput => "checkbox",
					WidgetKind::Textarea | WidgetKind::Select => unreachable!("handled above"),
				};
				PageElement::new("input").attr("type", input_type)
			}
		}
		.attr("id", id.clone())
		.attr("name", serialized_name)
		.attr(
			"class",
			display
				.class
				.as_ref()
				.unwrap_or(&options.input_class)
				.clone(),
		)
		.attr(
			"aria-describedby",
			describedby(
				display.help_text.as_ref().map(|_| help_id.as_str()),
				&error_id,
				display.aria_describedby.as_deref(),
			),
		)
		.reactive_attr("aria-invalid", move || {
			tracked_or_default(errors)
				.contains_key(&key)
				.then(|| "true".into())
		})
		.control_binding(binding);
	if required {
		control = control
			.bool_attr("required", true)
			.attr("aria-required", "true");
	}
	if display.widget == WidgetKind::NumberInput {
		control = control.attr("step", "any");
	}
	for (name, value) in [
		("aria-label", display.aria_label.as_ref()),
		("placeholder", display.placeholder.as_ref()),
		("autocomplete", display.autocomplete.as_ref()),
	] {
		if let Some(value) = value {
			control = control.attr(name, value.clone());
		}
	}
	let mut wrapper = PageElement::new("div")
		.attr(
			"class",
			display
				.wrapper_class
				.as_ref()
				.unwrap_or(&options.field_class)
				.clone(),
		)
		.child(
			PageElement::new("label")
				.attr("for", id)
				.attr(
					"class",
					display
						.label_class
						.as_ref()
						.unwrap_or(&options.label_class)
						.clone(),
				)
				.child(Page::text(display.label)),
		)
		.child(control);
	if let Some(help) = display.help_text {
		wrapper = wrapper.child(
			PageElement::new("div")
				.attr("id", help_id)
				.attr(
					"class",
					display
						.help_class
						.as_ref()
						.unwrap_or(&options.help_class)
						.clone(),
				)
				.child(Page::text(help)),
		);
	}
	wrapper
		.child(
			PageElement::new("div")
				.attr("id", error_id)
				.attr(
					"class",
					display
						.error_class
						.as_ref()
						.unwrap_or(&options.error_class)
						.clone(),
				)
				.reactive_attr("hidden", move || {
					(!tracked_or_default(errors).contains_key(&key)).then(|| "hidden".into())
				})
				.child(Page::reactive(move || {
					tracked_or_default(errors)
						.get(&key)
						.map(|error| Page::text(error.message().to_owned()))
						.unwrap_or_else(Page::empty)
				})),
		)
		.into_page()
}
