#![cfg(native)]

#[path = "ui/form/model_contract_support.rs"]
mod support;

use reinhardt_pages::component::{ControlValue, ControlWriteOutcome, Page, PageElement};
use reinhardt_pages::control_binding::__private::{TextBinding, into_control_binding};
use reinhardt_pages::reactive::ReactiveScope;
use reinhardt_pages::{FieldError, MutationDispatchOutcome, form, use_form};
use support::{QuestionCreateForm, QuestionCreateFormField, save_question};

#[test]
fn named_contract_binding_updates_the_attached_runtime() {
	// Arrange
	ReactiveScope::run(|| {
		let form = form! {
			name: QuestionPageForm,
			model_form: QuestionCreateForm,
			server_fn: save_question,
		};
		let runtime = use_form(&form).build();
		let binding = into_control_binding::<TextBinding, _>(
			runtime.field(QuestionCreateFormField::Title),
			(),
		);

		// Act
		assert_eq!(
			binding.write(ControlValue::Text("  Draft title  ".into())),
			Ok(ControlWriteOutcome::Committed),
		);

		// Assert
		assert_eq!(
			form.value("title"),
			Some(serde_json::json!("  Draft title  "))
		);
		assert_eq!(binding.read(), ControlValue::Text("  Draft title  ".into()));
		assert!(
			runtime
				.get_field_state(QuestionCreateFormField::Title)
				.is_dirty
		);
		assert!(
			runtime
				.get_field_state(QuestionCreateFormField::Title)
				.is_touched
		);
		runtime.reset();
		assert_eq!(binding.read(), ControlValue::Text(String::new()));
		assert!(
			!(runtime
				.get_field_state(QuestionCreateFormField::Title)
				.is_dirty)
		);
		assert!(
			!(runtime
				.get_field_state(QuestionCreateFormField::Title)
				.is_touched)
		);
	});
}

fn elements<'a>(page: &'a Page, output: &mut Vec<&'a PageElement>) {
	match page {
		Page::Element(element) => {
			output.push(element);
			for child in element.child_views() {
				elements(child, output);
			}
		}
		Page::Fragment(children) => {
			for child in children {
				elements(child, output);
			}
		}
		_ => {}
	}
}

fn attribute<'a>(element: &'a PageElement, name: &str) -> Option<&'a str> {
	element
		.attrs()
		.iter()
		.find(|(key, _)| key.as_ref() == name)
		.map(|(_, value)| value.as_ref())
}

#[test]
fn mutation_page_is_native_inert_and_preserves_field_metadata() {
	// Arrange
	ReactiveScope::run(|| {
		let form = form! {
			name: QuestionPageForm,
			model_form: QuestionCreateForm,
			server_fn: save_question,
			overrides: {
				title: {
					label: "Question",
					help_text: "Choose a concise title"
				},
			},
		};
		let calls = std::rc::Rc::new(std::cell::Cell::new(0));
		let success_calls = calls.clone();
		let runtime = use_form(&form)
			.on_submit_success(move |_| success_calls.set(success_calls.get() + 1))
			.build();
		let action = form
			.server_mutation(&runtime)
			.reset_form_on_success()
			.build();
		runtime.set_error(QuestionCreateFormField::Title, FieldError::new("Required"));

		// Act
		let page: Page = action.page();
		let html = page.render_to_string();
		let mut nodes = Vec::new();
		elements(&page, &mut nodes);

		// Assert
		let inputs: Vec<_> = nodes
			.iter()
			.copied()
			.filter(|node| attribute(node, "name") == Some("title"))
			.collect();
		assert_eq!(inputs.len(), 1);
		let input = inputs[0];
		let input_id = attribute(input, "id").expect("control ID");
		let labels: Vec<_> = nodes
			.iter()
			.copied()
			.filter(|node| attribute(node, "for") == Some(input_id))
			.collect();
		assert_eq!(labels.len(), 1);
		assert_eq!(labels[0].child_views()[0].render_to_string(), "Question");
		assert_eq!(attribute(input, "type"), Some("text"));
		assert_eq!(attribute(input, "required"), Some("required"));
		let descriptions = format!("{input_id}-help {input_id}-error");
		assert_eq!(
			attribute(input, "aria-describedby"),
			Some(descriptions.as_str())
		);
		assert_eq!(html.matches("Choose a concise title").count(), 1);
		assert_eq!(html.matches("Required").count(), 2);
		assert_eq!(
			action.dispatch(),
			MutationDispatchOutcome::UnsupportedTarget
		);
		assert_eq!(calls.get(), 0);
		assert!(!(action.is_pending()));
		assert!(action.result().is_none());
		assert_eq!(
			runtime
				.get_field_state(QuestionCreateFormField::Title)
				.error,
			Some(FieldError::new("Required"))
		);
	});
}

#[test]
fn mutation_page_disables_reset_while_submission_is_pending() {
	// Arrange
	ReactiveScope::run(|| {
		let form = form! {
			name: PendingResetPageForm,
			model_form: QuestionCreateForm,
			server_fn: save_question,
		};
		let runtime = use_form(&form).build();
		let action = form.server_mutation(&runtime).build();
		let page = action.page();
		assert!(!(page.render_to_string().contains("disabled=\"disabled\"")));

		// Act
		runtime.form_state().is_submitting.set(true);

		// Assert
		assert!(page.render_to_string().contains("disabled=\"disabled\""));
	});
}

#[test]
fn mutation_page_uses_attached_runtime_and_distinct_instance_ids() {
	ReactiveScope::run(|| {
		// Arrange
		let make_form = || {
			form! {
				name: AttachedQuestionForm,
				model_form: QuestionCreateForm,
				server_fn: save_question,
			}
		};
		let first = make_form();
		let second = make_form();
		first
			.set_value("title", serde_json::json!("first"))
			.unwrap();
		second
			.set_value("title", serde_json::json!("second"))
			.unwrap();
		let first_runtime = use_form(&first).build();
		let second_runtime = use_form(&second).build();

		// Act
		let first_page = first.server_mutation(&first_runtime).build().page();
		let attached_page = first.server_mutation(&second_runtime).build().page();
		let second_page = second.server_mutation(&second_runtime).build().page();
		let mut first_nodes = Vec::new();
		let mut second_nodes = Vec::new();
		elements(&first_page, &mut first_nodes);
		elements(&second_page, &mut second_nodes);
		let first_input = first_nodes
			.iter()
			.find(|node| attribute(node, "name") == Some("title"))
			.unwrap();
		let second_input = second_nodes
			.iter()
			.find(|node| attribute(node, "name") == Some("title"))
			.unwrap();

		// Assert
		assert_eq!(
			attached_page.render_to_string(),
			second_page.render_to_string()
		);
		assert_ne!(attribute(first_input, "id"), attribute(second_input, "id"));
		assert_eq!(
			(**first_input)
				.clone()
				.into_parts_with_control_binding()
				.6
				.unwrap()
				.read(),
			ControlValue::Text("first".into())
		);
		assert_eq!(
			(**second_input)
				.clone()
				.into_parts_with_control_binding()
				.6
				.unwrap()
				.read(),
			ControlValue::Text("second".into())
		);
		assert_eq!(first.value("title"), Some(serde_json::json!("first")));
	});
}
