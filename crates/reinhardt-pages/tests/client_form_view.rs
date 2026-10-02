//! Named ClientForm metadata and retained native Page contracts.
#![cfg(not(target_arch = "wasm32"))]

#[path = "support/client_form_view_shared.rs"]
pub mod support;

use reinhardt_pages::__private::client_form::{
	PasswordInput, SummaryEntry, ViewOptions, collect_summary, describedby,
};
use reinhardt_pages::FieldError;
use reinhardt_pages::component::ControlValue;
use reinhardt_pages::component::{Page, PageElement};
use reinhardt_pages::reactive::ReactiveScope;
use reinhardt_pages::{ClientForm, client_form, use_form};
use std::collections::HashMap;
use support::{LoginRequest, LoginRequestClientForm};

#[test]
fn form_macro_evaluates_presentation_once_in_source_order() {
	ReactiveScope::run(|| {
		// Arrange
		let definition = LoginRequestClientForm::new();
		let runtime = use_form(&definition).build();
		let mutation = definition.server_mutation(&runtime).build();
		let order = std::cell::RefCell::new(Vec::new());
		let record = |key, value: &str| {
			order.borrow_mut().push(key);
			String::from(value)
		};
		// Act
		let page = reinhardt_pages::form! {
			client_form: support::LoginRequestClientForm,
			summary: { label: record(0, "Errors") },
			customize: {
				password: { widget: PasswordInput, label: record(1, "Password") },
				username: { label: record(2, "Username") },
			},
			mutation: { order.borrow_mut().push(3); &mutation },
			styling: { class: record(4, "auth-form") },
			id: record(5, "login-macro"),
			submit: { label: record(6, "Sign in"), pending_label: record(7, "Signing in...") },
		}
		.into_page();
		// Assert
		assert_eq!(*order.borrow(), (0..8).collect::<Vec<_>>());
		assert_eq!(attr(elements(&page, "form")[0], "class"), Some("auth-form"));
		runtime.set_value(definition.username_field(), String::from("changed"));
		runtime.set_error(definition.password_field(), FieldError::new("Required"));
		let _html = page.render_to_string();
		assert_eq!(*order.borrow(), (0..8).collect::<Vec<_>>());
		assert_eq!(
			elements(&page, "input")[0].bound_control().unwrap().read(),
			ControlValue::Text(String::from("changed"))
		);
	});
}

fn elements<'a>(page: &'a Page, tag: &str) -> Vec<&'a PageElement> {
	let mut result = Vec::new();
	if let Page::Element(element) = page {
		if element.tag_name() == tag {
			result.push(element);
		}
		for child in element.child_views() {
			result.extend(elements(child, tag));
		}
	}
	result
}

fn attr<'a>(element: &'a PageElement, name: &str) -> Option<&'a str> {
	element
		.attrs()
		.iter()
		.find(|(key, _)| key == name)
		.map(|(_, value)| value.as_ref())
}

#[test]
fn summary_preserves_field_order_and_deduplicates_only_equal_global_messages() {
	// Arrange
	let fields = vec![
		(0_u8, String::from("login-field-0")),
		(1_u8, String::from("login-field-1")),
	];
	let errors = HashMap::from([
		(1_u8, FieldError::new("same")),
		(0_u8, FieldError::new("same")),
	]);
	// Act / Assert
	assert_eq!(
		collect_summary(
			&fields,
			&errors,
			Some("unmatched: rejected"),
			Some("navigation failed")
		),
		vec![
			SummaryEntry {
				control_id: Some("login-field-0".into()),
				message: "same".into()
			},
			SummaryEntry {
				control_id: Some("login-field-1".into()),
				message: "same".into()
			},
			SummaryEntry {
				control_id: None,
				message: "unmatched: rejected".into()
			},
			SummaryEntry {
				control_id: None,
				message: "navigation failed".into()
			},
		]
	);
	assert_eq!(
		collect_summary::<u8>(&[], &HashMap::new(), Some("same"), Some("same")),
		vec![SummaryEntry {
			control_id: None,
			message: "same".into()
		}]
	);
	assert_eq!(
		collect_summary::<u8>(&[], &HashMap::new(), Some(""), None),
		vec![]
	);
	assert_eq!(
		describedby(Some("help"), "error", Some("external help\texternal error")),
		"help error external"
	);
}

#[test]
fn retained_controls_share_bindings_and_project_only_safe_constraints() {
	ReactiveScope::run(|| {
		// Arrange
		let definition = LoginRequestClientForm::new().with_defaults(LoginRequest {
			username: "alice".into(),
			password: "secret".into(),
		});
		let runtime = use_form(&definition).build();
		let mutation = definition.server_mutation(&runtime).build();
		let view =
			LoginRequestClientForm::__reinhardt_view(&mutation, ViewOptions::default().id("login"))
				.__reinhardt_field_username(|field| {
					field
						.help_text("A <name>")
						.aria_describedby("external login-help-0")
				})
				.__reinhardt_field_password(|field| {
					field.widget(PasswordInput).autocomplete("current-password")
				});
		// Act
		let page = view.into_page();
		// Assert
		let form = elements(&page, "form")[0];
		assert_eq!(attr(form, "novalidate"), Some("novalidate"));
		let inputs = elements(&page, "input");
		assert_eq!(inputs.len(), 2);
		assert_eq!(
			inputs
				.iter()
				.map(|input| (attr(input, "id"), attr(input, "name"), attr(input, "type")))
				.collect::<Vec<_>>(),
			vec![
				(Some("login-field-0"), Some("username"), Some("text")),
				(Some("login-field-1"), Some("password"), Some("password"))
			]
		);
		assert_eq!(
			attr(inputs[0], "aria-describedby"),
			Some("login-help-0 login-error-0 external")
		);
		for input in &inputs {
			assert_eq!(attr(input, "required"), Some("required"));
			assert_eq!(attr(input, "aria-required"), Some("true"));
			for forbidden in ["min", "max", "minlength", "maxlength", "pattern"] {
				assert_eq!(attr(input, forbidden), None);
			}
		}
		assert_eq!(
			inputs[0].bound_control().unwrap().target(),
			runtime
				.watch_field::<String>(definition.username_field())
				.id()
		);
		assert_eq!(elements(&page, "button").len(), 1);
		runtime.set_error(definition.username_field(), FieldError::new("<invalid>"));
		let invalid = inputs[0]
			.reactive_attrs()
			.iter()
			.find(|attribute| attribute.name() == "aria-invalid")
			.unwrap();
		assert_eq!(invalid.value().as_deref(), Some("true"));
		runtime.clear_errors();
		assert_eq!(invalid.value(), None);
		let html = page.render_to_string();
		assert_eq!(html.matches("secret").count(), 0);
		assert_eq!(html.matches("A &lt;name&gt;").count(), 1);
		assert_eq!(html.matches("aria-live=").count(), 1);
	});
}

#[test]
#[should_panic(expected = "ClientForm view id must be nonempty")]
fn runtime_id_expression_is_validated_before_a_view_is_assembled() {
	let _options = ViewOptions::default().id(String::from("bad id"));
}

#[test]
fn metadata_uses_the_owned_runtime_and_dto_declaration_order() {
	ReactiveScope::run(|| {
		// Arrange
		let definition = LoginRequestClientForm::new().with_defaults(LoginRequest {
			username: String::from("alice"),
			password: String::from("secret"),
		});
		let runtime = use_form(&definition).build();
		// Act
		let fields = LoginRequestClientForm::__reinhardt_view_fields(&runtime);
		// Assert
		assert_eq!(
			fields
				.iter()
				.map(|field| (
					field.ordinal,
					field.rust_name,
					field.serialized_name,
					field.required
				))
				.collect::<Vec<_>>(),
			vec![
				(0, "username", "username", true),
				(1, "password", "password", true)
			]
		);
		assert_eq!(
			fields[0].binding.read(),
			ControlValue::Text(String::from("alice"))
		);
		assert_eq!(
			fields[1].binding.read(),
			ControlValue::Text(String::from("secret"))
		);
		assert!(!runtime.form_state().is_touched.get());
		fields[0]
			.binding
			.write(ControlValue::Text(String::from("bob")))
			.unwrap();
		assert_eq!(LoginRequestClientForm::to_request(&runtime).username, "bob");
	});
}

#[client_form(name = RenamedForm)]
#[derive(Clone)]
#[serde(rename_all = "camelCase")]
struct Renamed {
	first_name: String,
	#[serde(rename(serialize = "wireType", deserialize = "otherType"))]
	r#type: String,
	#[client_form(skip)]
	internal: u32,
}

#[derive(Clone, ClientForm)]
struct Unvalidated {
	text: String,
	optional: Option<String>,
	flag: bool,
	count: u32,
}

#[test]
fn metadata_preserves_renames_raw_names_and_skipped_fields() {
	ReactiveScope::run(|| {
		// Arrange
		let definition = RenamedForm::new();
		let runtime = use_form(&definition).build();
		// Act
		let fields = RenamedForm::__reinhardt_view_fields(&runtime);
		// Assert
		assert_eq!(
			fields
				.iter()
				.map(|field| (field.rust_name, field.serialized_name, field.required))
				.collect::<Vec<_>>(),
			vec![
				("first_name", "firstName", false),
				("type", "wireType", false)
			]
		);
		let request = RenamedForm::to_request(&runtime);
		assert_eq!(
			(request.first_name, request.r#type, request.internal),
			(String::new(), String::new(), 0)
		);
	});
}

#[test]
fn derive_entry_does_not_infer_required_without_validation() {
	ReactiveScope::run(|| {
		let definition = UnvalidatedClientForm::new();
		let runtime = use_form(&definition).build();
		let fields = UnvalidatedClientForm::__reinhardt_view_fields(&runtime);
		assert_eq!(
			fields
				.iter()
				.map(|field| field.required)
				.collect::<Vec<_>>(),
			vec![false; 4]
		);
		let request = UnvalidatedClientForm::to_request(&runtime);
		assert_eq!(
			(request.text, request.optional, request.flag, request.count),
			(String::new(), None, false, 0)
		);
	});
}

#[tokio::test]
async fn request_id_scopes_are_independent_and_default_ids_are_unique_within_a_request() {
	async fn ids() -> Vec<String> {
		reinhardt_pages::reactive::hooks::id::scope_id_counter(async {
            tokio::task::yield_now().await;
            ReactiveScope::run(|| {
                let definition = LoginRequestClientForm::new();
                let runtime = use_form(&definition).build();
                let mutation = definition.server_mutation(&runtime).build();
                (0..2).map(|_| {
                    let page = reinhardt_pages::form! { client_form: LoginRequestClientForm, mutation: &mutation }.into_page();
                    attr(elements(&page, "form")[0], "id").unwrap().to_owned()
                }).collect()
            })
        }).await
	}
	let (first, second) = tokio::join!(ids(), ids());
	assert_eq!(first, vec!["client-form-0", "client-form-1"]);
	assert_eq!(first, second);
}
