//! Form Component for Client-Side Rendering (Week 5 Day 3)
//!
//! This module provides a `FormComponent` that renders Django-style forms
//! in WASM using `FormMetadata` from `reinhardt-forms`.
//!
//! ## Architecture
//!
//! ```text
//! Server-side:              Client-side (WASM):
//! ┌──────────┐             ┌──────────────┐
//! │   Form   │             │FormComponent │
//! │          │             │              │
//! │ to_meta()│──────────▶  │  render()    │──▶ DOM
//! │          │  JSON/etc   │  validate()  │
//! │          │             │  submit()    │──▶ AJAX
//! └──────────┘             └──────────────┘
//! ```
//!
//! ## Features
//!
//! - **Automatic CSRF Protection**: CSRF token automatically injected as hidden input
//! - **Client-Side Validation**: Validates required fields, displays errors
//! - **Reactive State**: Field values managed with Signals from Phase 1
//! - **AJAX Submission**: Form submission via fetch API with CSRF token
//! - **Widget Rendering**: Renders appropriate input types based on Widget metadata
//!
//! ## Example
//!
//! ```ignore
//! use reinhardt_pages::form::FormComponent;
//! use reinhardt_forms::wasm_compat::FormMetadata;
//!
//! // FormMetadata received from server (serialized as JSON)
//! let metadata: FormMetadata = serde_json::from_str(&json_data)?;
//!
//! // Create FormComponent
//! let mut form = FormComponent::new(metadata, "/api/submit");
//!
//! // Render form to DOM
//! let form_element = form.render();
//! document.body().append_child(&form_element)?;
//!
//! // On submit button click
//! form.submit().await?;
//! ```

#[cfg(target_arch = "wasm32")]
use crate::dom::{Document, Element};
#[cfg(target_arch = "wasm32")]
use crate::reactive::Effect;
use crate::reactive::Signal;
use std::collections::HashMap;

/// Form Component for client-side rendering (Week 5 Day 3)
///
/// This component wraps `FormMetadata` from `reinhardt-forms` and provides
/// methods for rendering, validation, and submission on the client side.
///
/// ## Fields
///
/// - `metadata`: Serialized form metadata from server
/// - `values`: Current field values (Signal-based for reactivity)
/// - `errors`: Validation error messages
/// - `action`: Form submission URL (e.g., "/api/submit")
/// - `method`: HTTP method (default: "POST")
#[derive(Clone)]
pub struct FormComponent {
	/// Form metadata from server
	metadata: reinhardt_forms::wasm_compat::FormMetadata,

	/// Field values (reactive)
	values: HashMap<String, Signal<String>>,

	/// Validation errors
	errors: Signal<HashMap<String, Vec<String>>>,

	/// Form submission URL (used in WASM render() method)
	#[allow(dead_code)]
	action: String,

	/// HTTP method (GET or POST, used in WASM render() method)
	#[allow(dead_code)]
	method: String,
}

impl FormComponent {
	/// Create a new FormComponent from metadata
	///
	/// # Arguments
	///
	/// - `metadata`: Form metadata from server (via `Form::to_metadata()`)
	/// - `action`: Form submission URL (e.g., "/api/submit")
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_pages::form::FormComponent;
	///
	/// let metadata = form.to_metadata();
	/// let component = FormComponent::new(metadata, "/api/contact");
	/// ```
	pub fn new(
		metadata: reinhardt_forms::wasm_compat::FormMetadata,
		action: impl Into<String>,
	) -> Self {
		// Initialize field values from initial data
		let values: HashMap<String, Signal<String>> = metadata
			.fields
			.iter()
			.map(|field| {
				// Priority: field.initial > metadata.initial > empty string
				let initial_value = field
					.initial
					.as_ref()
					.and_then(|v| v.as_str())
					.or_else(|| metadata.initial.get(&field.name).and_then(|v| v.as_str()))
					.unwrap_or("")
					.to_string();

				(field.name.clone(), Signal::new(initial_value))
			})
			.collect();

		// Initialize errors from metadata (if form is bound)
		let initial_errors = metadata.errors.clone();

		Self {
			metadata,
			values,
			errors: Signal::new(initial_errors),
			action: action.into(),
			method: "POST".to_string(),
		}
	}

	/// Render the form to a DOM element (WASM only)
	///
	/// This method creates a `<form>` element with all fields, labels,
	/// help text, and CSRF token (if present).
	///
	/// # Returns
	///
	/// A `web_sys::Element` representing the rendered form.
	///
	/// # Examples
	///
	/// ```ignore
	/// let form_element = form_component.render();
	/// document.body().append_child(&form_element)?;
	/// ```
	#[cfg(target_arch = "wasm32")]
	pub fn render(&self) -> web_sys::Element {
		use crate::builder::html;

		// Create form element
		let form = Document::global()
			.create_element("form")
			.expect("Failed to create form element");

		form.set_attribute("action", &self.action)
			.expect("Failed to set action");
		form.set_attribute("method", &self.method)
			.expect("Failed to set method");

		// Add CSRF token (if present)
		if let Some(ref csrf_token) = self.metadata.csrf_token {
			let csrf_input = Document::global()
				.create_element("input")
				.expect("Failed to create CSRF input");
			csrf_input
				.set_attribute("type", "hidden")
				.expect("Failed to set type");
			csrf_input
				.set_attribute("name", "csrfmiddlewaretoken")
				.expect("Failed to set name");
			csrf_input
				.set_attribute("value", csrf_token)
				.expect("Failed to set value");

			form.append_child(&csrf_input)
				.expect("Failed to append CSRF input");
		}

		// Render each field
		for field_meta in &self.metadata.fields {
			let field_div = self.render_field(field_meta);
			form.append_child(&field_div)
				.expect("Failed to append field");
		}

		// Add submit button
		let submit_button = Document::global()
			.create_element("button")
			.expect("Failed to create submit button");
		submit_button
			.set_attribute("type", "submit")
			.expect("Failed to set type");
		submit_button.set_text_content(Some("Submit"));

		form.append_child(&submit_button)
			.expect("Failed to append submit button");

		// Attach submit event listener
		self.attach_submit_listener(&form);

		form
	}

	/// Render a single field (WASM only)
	#[cfg(target_arch = "wasm32")]
	fn render_field(
		&self,
		field_meta: &reinhardt_forms::wasm_compat::FieldMetadata,
	) -> web_sys::Element {
		use reinhardt_forms::Widget;

		let field_div = Document::global()
			.create_element("div")
			.expect("Failed to create field div");
		field_div
			.set_attribute("class", "form-field")
			.expect("Failed to set class");

		// Render label
		if let Some(ref label_text) = field_meta.label {
			let label = Document::global()
				.create_element("label")
				.expect("Failed to create label");
			label
				.set_attribute("for", &field_meta.name)
				.expect("Failed to set for");
			label.set_text_content(Some(label_text));

			if field_meta.required {
				let required_span = Document::global()
					.create_element("span")
					.expect("Failed to create required span");
				required_span
					.set_attribute("class", "required")
					.expect("Failed to set class");
				required_span.set_text_content(Some(" *"));
				label
					.append_child(&required_span)
					.expect("Failed to append required indicator");
			}

			field_div
				.append_child(&label)
				.expect("Failed to append label");
		}

		// Render input based on widget type
		let input = self.render_widget(&field_meta.widget, &field_meta.name);
		field_div
			.append_child(&input)
			.expect("Failed to append input");

		// Render help text
		if let Some(ref help_text) = field_meta.help_text {
			let help_span = Document::global()
				.create_element("span")
				.expect("Failed to create help span");
			help_span
				.set_attribute("class", "help-text")
				.expect("Failed to set class");
			help_span.set_text_content(Some(help_text));
			field_div
				.append_child(&help_span)
				.expect("Failed to append help text");
		}

		// Render errors (reactive)
		let error_div = Document::global()
			.create_element("div")
			.expect("Failed to create error div");
		error_div
			.set_attribute("class", "field-errors")
			.expect("Failed to set class");

		let field_name = field_meta.name.clone();
		let errors_signal = self.errors.clone();

		// Effect: Update error display when errors change
		let error_div_clone = error_div.clone();
		Effect::new(move || {
			let errors = errors_signal.get();
			if let Some(field_errors) = errors.get(&field_name) {
				let error_text = field_errors.join(", ");
				error_div_clone.set_text_content(Some(&error_text));
			} else {
				error_div_clone.set_text_content(None);
			}
		});

		field_div
			.append_child(&error_div)
			.expect("Failed to append error div");

		field_div
	}

	/// Render widget based on type (WASM only)
	#[cfg(target_arch = "wasm32")]
	fn render_widget(&self, widget: &reinhardt_forms::Widget, name: &str) -> web_sys::Element {
		use reinhardt_forms::Widget;

		let input = Document::global()
			.create_element("input")
			.expect("Failed to create input");
		input
			.set_attribute("name", name)
			.expect("Failed to set name");

		// Set input type based on widget
		let input_type = match widget {
			Widget::TextInput => "text",
			Widget::PasswordInput => "password",
			Widget::EmailInput => "email",
			Widget::NumberInput => "number",
			Widget::DateInput => "date",
			Widget::CheckboxInput => "checkbox",
			Widget::HiddenInput => "hidden",
			Widget::FileInput => "file",
			_ => "text", // Default fallback
		};

		input
			.set_attribute("type", input_type)
			.expect("Failed to set type");

		// Bind value to Signal
		if let Some(value_signal) = self.values.get(name) {
			let value = value_signal.get();
			input
				.set_attribute("value", &value)
				.expect("Failed to set value");

			// Attach input event listener
			let value_signal_clone = value_signal.clone();
			let input_clone = input.clone();

			use wasm_bindgen::JsCast;
			use wasm_bindgen::prelude::*;

			let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
				let target = input_clone
					.clone()
					.dyn_into::<web_sys::HtmlInputElement>()
					.expect("Failed to cast to HtmlInputElement");
				let new_value = target.value();
				value_signal_clone.set(new_value);
			}) as Box<dyn FnMut(_)>);

			input
				.add_event_listener_with_callback("input", closure.as_ref().unchecked_ref())
				.expect("Failed to add event listener");

			closure.forget(); // Keep closure alive
		}

		input
	}

	/// Attach submit event listener (WASM only)
	#[cfg(target_arch = "wasm32")]
	fn attach_submit_listener(&self, form: &web_sys::Element) {
		use wasm_bindgen::JsCast;
		use wasm_bindgen::prelude::*;

		let form_component = self.clone();

		let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
			event.prevent_default();

			// Validate
			if !form_component.validate() {
				web_sys::console::log_1(&"Validation failed".into());
				return;
			}

			// Submit (spawn async task)
			wasm_bindgen_futures::spawn_local(async move {
				match form_component.submit().await {
					Ok(_) => {
						web_sys::console::log_1(&"Form submitted successfully".into());
					}
					Err(err) => {
						web_sys::console::error_1(&format!("Submit error: {:?}", err).into());
					}
				}
			});
		}) as Box<dyn FnMut(_)>);

		form.add_event_listener_with_callback("submit", closure.as_ref().unchecked_ref())
			.expect("Failed to add submit listener");

		closure.forget(); // Keep closure alive
	}

	/// Validate form fields (Week 5 Day 3)
	///
	/// Performs client-side validation:
	/// - Checks required fields are not empty
	/// - Updates error state via Signal
	///
	/// # Returns
	///
	/// `true` if validation passed, `false` otherwise.
	///
	/// # Examples
	///
	/// ```ignore
	/// if form_component.validate() {
	///     form_component.submit().await?;
	/// }
	/// ```
	pub fn validate(&self) -> bool {
		let mut errors = HashMap::new();

		for field_meta in &self.metadata.fields {
			if field_meta.required
				&& let Some(value_signal) = self.values.get(&field_meta.name)
			{
				let value = value_signal.get();
				if value.trim().is_empty() {
					errors
						.entry(field_meta.name.clone())
						.or_insert_with(Vec::new)
						.push("This field is required.".to_string());
				}
			}
		}

		let is_valid = errors.is_empty();
		self.errors.set(errors);

		is_valid
	}

	/// Submit form data via AJAX (Week 5 Day 3)
	///
	/// Sends form data to the server with CSRF token automatically included.
	///
	/// # Returns
	///
	/// `Ok(())` on success, `Err(String)` on failure.
	///
	/// # Examples
	///
	/// ```ignore
	/// form_component.submit().await?;
	/// ```
	#[cfg(target_arch = "wasm32")]
	pub async fn submit(&self) -> Result<(), String> {
		use gloo_net::http::Request;
		use serde_json::json;

		// Collect form data
		let mut data = serde_json::Map::new();

		for (name, value_signal) in &self.values {
			let value = value_signal.get();
			data.insert(name.clone(), json!(value));
		}

		// Add CSRF token
		if let Some(ref csrf_token) = self.metadata.csrf_token {
			data.insert("csrfmiddlewaretoken".to_string(), json!(csrf_token));
		}

		// Send POST request
		let response = Request::post(&self.action)
			.header("Content-Type", "application/json")
			.json(&data)
			.map_err(|e| format!("Failed to create request: {:?}", e))?
			.send()
			.await
			.map_err(|e| format!("Failed to send request: {:?}", e))?;

		if response.ok() {
			Ok(())
		} else {
			Err(format!("Submit failed with status: {}", response.status()))
		}
	}

	/// Get current field value
	///
	/// # Arguments
	///
	/// - `name`: Field name
	///
	/// # Returns
	///
	/// Current value as String, or empty string if field not found.
	pub fn get_value(&self, name: &str) -> String {
		self.values
			.get(name)
			.map(|signal| signal.get())
			.unwrap_or_default()
	}

	/// Set field value
	///
	/// # Arguments
	///
	/// - `name`: Field name
	/// - `value`: New value
	pub fn set_value(&self, name: &str, value: impl Into<String>) {
		if let Some(signal) = self.values.get(name) {
			signal.set(value.into());
		}
	}

	/// Get form metadata
	pub fn metadata(&self) -> &reinhardt_forms::wasm_compat::FormMetadata {
		&self.metadata
	}

	/// Get current errors
	pub fn errors(&self) -> HashMap<String, Vec<String>> {
		self.errors.get()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_forms::Widget;
	use reinhardt_forms::wasm_compat::{FieldMetadata, FormMetadata};
	use std::collections::HashMap;

	#[test]
	fn test_form_component_creation() {
		let metadata = FormMetadata {
			fields: vec![FieldMetadata {
				name: "username".to_string(),
				label: Some("Username".to_string()),
				required: true,
				help_text: None,
				widget: Widget::TextInput,
				initial: None,
			}],
			initial: HashMap::new(),
			csrf_token: Some("test-token".to_string()),
			prefix: String::new(),
			is_bound: false,
			errors: HashMap::new(),
		};

		let component = FormComponent::new(metadata, "/api/submit");

		assert_eq!(component.action, "/api/submit");
		assert_eq!(component.method, "POST");
		assert_eq!(component.values.len(), 1);
		assert!(component.values.contains_key("username"));
	}

	#[test]
	fn test_form_component_validation_required_field() {
		let metadata = FormMetadata {
			fields: vec![FieldMetadata {
				name: "email".to_string(),
				label: Some("Email".to_string()),
				required: true,
				help_text: None,
				widget: Widget::EmailInput,
				initial: None,
			}],
			initial: HashMap::new(),
			csrf_token: None,
			prefix: String::new(),
			is_bound: false,
			errors: HashMap::new(),
		};

		let component = FormComponent::new(metadata, "/api/submit");

		// Empty value should fail validation
		assert!(!component.validate());

		let errors = component.errors();
		assert!(errors.contains_key("email"));
		assert_eq!(errors.get("email").unwrap()[0], "This field is required.");

		// Set value and validate again
		component.set_value("email", "test@example.com");
		assert!(component.validate());

		let errors = component.errors();
		assert!(errors.is_empty());
	}

	#[test]
	fn test_form_component_get_set_value() {
		let metadata = FormMetadata {
			fields: vec![FieldMetadata {
				name: "name".to_string(),
				label: Some("Name".to_string()),
				required: false,
				help_text: None,
				widget: Widget::TextInput,
				initial: None,
			}],
			initial: HashMap::new(),
			csrf_token: None,
			prefix: String::new(),
			is_bound: false,
			errors: HashMap::new(),
		};

		let component = FormComponent::new(metadata, "/api/submit");

		assert_eq!(component.get_value("name"), "");

		component.set_value("name", "John Doe");
		assert_eq!(component.get_value("name"), "John Doe");
	}

	#[test]
	fn test_form_component_with_initial_values() {
		let mut initial = HashMap::new();
		initial.insert("username".to_string(), serde_json::json!("john_doe"));

		let metadata = FormMetadata {
			fields: vec![FieldMetadata {
				name: "username".to_string(),
				label: Some("Username".to_string()),
				required: false,
				help_text: None,
				widget: Widget::TextInput,
				initial: None,
			}],
			initial,
			csrf_token: None,
			prefix: String::new(),
			is_bound: false,
			errors: HashMap::new(),
		};

		let component = FormComponent::new(metadata, "/api/submit");

		assert_eq!(component.get_value("username"), "john_doe");
	}
}
