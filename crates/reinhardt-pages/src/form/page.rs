//! Stable, runtime-bound presentation for generated model-form mutations.

use crate::component::{IntoPage, Page, PageElement};
use crate::{FormRuntimeSource, FormServerMutation, UseFormReturn};

/// Generated controls and their enclosing form, before mutation actions are added.
#[doc(hidden)]
pub struct FormPageParts<Field> {
	/// Form metadata and hidden protocol controls, without submission handlers.
	pub container: PageElement,
	/// Selected editable fields in their declared order.
	pub fields: Vec<FormFieldPage<Field>>,
}

/// A stable field page and the metadata used by the linked error summary.
#[doc(hidden)]
pub struct FormFieldPage<Field> {
	/// Runtime field token used to read validation errors.
	pub field: Field,
	/// Unique control identifier shared by its label and summary link.
	pub control_id: String,
	/// Display label from the model metadata or a form override.
	pub label: &'static str,
	/// Bound control, auxiliary controls, help text, and field error region.
	pub page: Page,
}

/// Accessible field metadata emitted by the form macro.
#[doc(hidden)]
pub struct FormFieldDescription<Field> {
	/// Field belonging to the attached runtime.
	pub field: Field,
	/// Identifier of the bound control.
	pub control_id: String,
	/// Accessible control label.
	pub label: &'static str,
	/// Optional help text referenced by the control.
	pub help_text: Option<&'static str>,
}

/// Presentation supplied by generated model-backed forms.
///
/// This additive contract lets a server mutation render the controls of its
/// attached runtime without creating another form or submission lifecycle.
pub trait FormPageSource: FormRuntimeSource {
	/// Builds stable controls from the source retained by the supplied runtime.
	#[doc(hidden)]
	fn form_page_parts<Deps>(
		&self,
		runtime: &UseFormReturn<Self, Deps>,
	) -> FormPageParts<Self::Field>
	where
		Deps: Clone + PartialEq + 'static;
}

/// Reconciles an instance-specific identifier or reference during hydration.
///
/// Retains static metadata for page inspection while allowing a fresh client
/// instance to replace the server's ID sequence without replacing DOM nodes.
#[doc(hidden)]
pub fn instance_attribute(element: PageElement, name: &'static str, value: String) -> PageElement {
	element
		.attr(name, value.clone())
		.reactive_attr(name, move || Some(value.clone().into()))
}

/// Wraps one bound control with accessible metadata and a narrow error region.
#[doc(hidden)]
pub fn render_field<Form, Deps>(
	runtime: &UseFormReturn<Form, Deps>,
	description: FormFieldDescription<Form::Field>,
	control: PageElement,
	auxiliary: Vec<Page>,
) -> FormFieldPage<Form::Field>
where
	Form: FormRuntimeSource,
	Deps: Clone + PartialEq + 'static,
{
	let FormFieldDescription {
		field,
		control_id,
		label,
		help_text,
	} = description;
	let errors = runtime.form_state().field_errors;
	let help_id = format!("{control_id}-help");
	let error_id = format!("{control_id}-error");
	let described_by = if help_text.is_some() {
		format!("{help_id} {error_id}")
	} else {
		error_id.clone()
	};
	let control = instance_attribute(control, "id", control_id.clone());
	let control = instance_attribute(control, "aria-describedby", described_by).reactive_attr(
		"aria-invalid",
		move || {
			Some(
				if errors.get().contains_key(&field) {
					"true"
				} else {
					"false"
				}
				.into(),
			)
		},
	);
	let mut wrapper = PageElement::new("div")
		.attr("class", "reinhardt-form-field")
		.child(
			instance_attribute(PageElement::new("label"), "for", control_id.clone()).child(label),
		)
		.child(control)
		.children(auxiliary);
	if let Some(help) = help_text {
		wrapper = wrapper.child(
			instance_attribute(PageElement::new("small"), "id", help_id)
				.attr("class", "reinhardt-form-help")
				.child(help),
		);
	}
	wrapper = wrapper.child(
		instance_attribute(PageElement::new("span"), "id", error_id)
			.attr("class", "reinhardt-form-error")
			.child(Page::reactive(move || {
				errors
					.get()
					.get(&field)
					.map(|error| Page::text(error.message().to_owned()))
					.unwrap_or(Page::Empty)
			})),
	);
	FormFieldPage {
		field,
		control_id,
		label,
		page: wrapper.into_page(),
	}
}

pub(crate) fn render_mutation_page<Form, Deps, Input, Output>(
	action: &FormServerMutation<Form, Deps, Input, Output>,
) -> Page
where
	Form: FormPageSource,
	Deps: Clone + PartialEq + 'static,
	Input: 'static,
	Output: Clone + 'static,
{
	let runtime = action.form();
	let source = runtime.__reinhardt_form_source();
	let FormPageParts { container, fields } =
		crate::reactive::untracked(|| source.form_page_parts(&runtime));
	let errors = runtime.form_state().field_errors;
	let form_error = runtime.form_state().form_error;
	let ordered: Vec<_> = fields
		.iter()
		.map(|entry| (entry.field, entry.control_id.clone(), entry.label))
		.collect();
	let summary = PageElement::new("div")
		.attr("class", "reinhardt-form-error-summary")
		.attr("role", "alert")
		.child(Page::reactive(move || {
			let current = errors.get();
			let items: Vec<_> = ordered
				.iter()
				.filter_map(|(field, id, label)| {
					current.get(field).map(|error| {
						PageElement::new("li").child(
							instance_attribute(PageElement::new("a"), "href", format!("#{id}"))
								.child(format!("{label}: {}", error.message())),
						)
					})
				})
				.collect();
			if items.is_empty() {
				Page::Empty
			} else {
				PageElement::new("ul").children(items).into_page()
			}
		}));
	let global_error = PageElement::new("div")
		.attr("class", "reinhardt-form-global-error")
		.attr("role", "alert")
		.child(Page::reactive(move || {
			form_error.get().map(Page::text).unwrap_or(Page::Empty)
		}));
	let disabled_action = action.clone();
	let label_action = action.clone();
	let busy_action = action.clone();
	let status_action = action.clone();
	let submit = PageElement::new("button")
		.attr("type", "submit")
		.reactive_attr("disabled", move || {
			disabled_action.is_pending().then(|| "disabled".into())
		})
		.child(Page::reactive(move || {
			Page::text(if label_action.is_pending() {
				"Submitting..."
			} else {
				"Submit"
			})
		}));
	let reset = PageElement::new("button")
		.attr("type", "button")
		.reactive_attr("disabled", {
			let reset_action = action.clone();
			move || reset_action.is_pending().then(|| "disabled".into())
		})
		.child("Reset");
	#[cfg(wasm)]
	let reset = reset.on(
		crate::event::KnownEvent::Click,
		crate::typed_event_handler::<crate::event::ClickEvent, _>(
			move |event: crate::event::ClickEvent| {
				event.prevent_default();
				runtime.reset();
			},
		),
	);
	let container = container
		.reactive_attr("aria-busy", move || {
			Some(
				if busy_action.is_pending() {
					"true"
				} else {
					"false"
				}
				.into(),
			)
		})
		.child(summary)
		.child(global_error)
		.children(fields.into_iter().map(|entry| entry.page))
		.child(submit)
		.child(reset)
		.child(
			PageElement::new("span")
				.attr("class", "reinhardt-form-status")
				.attr("role", "status")
				.attr("aria-live", "polite")
				.child(Page::reactive(move || {
					if status_action.is_pending() {
						Page::text("Submitting...")
					} else {
						Page::Empty
					}
				})),
		);
	#[cfg(wasm)]
	let container = {
		let submit_action = action.clone();
		container.on(
			crate::event::KnownEvent::Submit,
			crate::typed_event_handler::<crate::event::SubmitEvent, _>(
				move |event: crate::event::SubmitEvent| {
					event.prevent_default();
					submit_action.dispatch();
				},
			),
		)
	};
	container.into_page()
}
