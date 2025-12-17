//! Form component for creating and editing model instances

use crate::state::{AppState, FormMode, FormState};
use dominator::{Dom, clone, events, html};
use futures_signals::signal::SignalExt;
use reinhardt_admin_types::{FieldInfo, FieldType};
use std::sync::Arc;
use wasm_bindgen::{JsCast, prelude::*};

/// Render the form for creating or editing a model instance
pub fn render(state: Arc<AppState>, model_name: String, id: Option<String>) -> Dom {
	// Get or create form state
	let form_state = if let Some(ref id_str) = id {
		state.get_edit_form_state(&model_name, id_str)
	} else {
		state.get_create_form_state(&model_name)
	};

	// Get field definitions (mock for Phase 3)
	let fields = get_mock_fields(&model_name);

	html!("div", {
		.class("form-view")
		.children(&mut [
			render_header(&form_state, &model_name),
			render_loading(&form_state),
			render_form_error(&form_state),
			render_form_fields(&form_state, &fields),
			render_form_actions(&form_state, &fields, &model_name),
		])
	})
}

/// Render the header with title and Back button
fn render_header(form_state: &Arc<FormState>, model_name: &str) -> Dom {
	let title = match form_state.mode {
		FormMode::Create => format!("Create {}", model_name),
		FormMode::Edit => format!("Edit {}", model_name),
	};

	html!("div", {
		.class("form-view-header")
		.children(&mut [
			html!("button", {
				.class("btn btn-secondary")
				.text("← Back")
				.event(|_: events::Click| {
					let window = web_sys::window().unwrap();
					let history = window.history().unwrap();
					history.back().unwrap();
				})
			}),
			html!("h1", {
				.class("form-view-title")
				.text(&title)
			}),
		])
	})
}

/// Render loading indicator
fn render_loading(form_state: &Arc<FormState>) -> Dom {
	html!("div", {
		.class("loading")
		.visible_signal(form_state.is_loading.signal())
		.text("Loading...")
	})
}

/// Render form-level error
fn render_form_error(form_state: &Arc<FormState>) -> Dom {
	html!("div", {
		.class("form-error")
		.visible_signal(form_state.form_error.signal_cloned().map(|e| e.is_some()))
		.child_signal(form_state.form_error.signal_cloned().map(clone!(form_state => move |error| {
			error.map(|msg| {
				html!("div", {
					.class("error")
					.children(&mut [
						html!("span", {
							.text(&msg)
						}),
						html!("button", {
							.class("btn btn-sm")
							.text("×")
							.event(clone!(form_state => move |_: events::Click| {
								form_state.clear_error();
							}))
						}),
					])
				})
			})
		})))
	})
}

/// Render form fields
fn render_form_fields(form_state: &Arc<FormState>, fields: &[FieldInfo]) -> Dom {
	let field_doms: Vec<Dom> = fields
		.iter()
		.map(|field| render_field(form_state, field))
		.collect();

	html!("div", {
		.class("form-fields")
		.visible_signal(form_state.is_loading.signal().map(|loading| !loading))
		.children(field_doms)
	})
}

/// Render a single field
fn render_field(form_state: &Arc<FormState>, field: &FieldInfo) -> Dom {
	let mut children = vec![
		html!("label", {
			.class("form-field-label")
			.text(&field.label)
			.apply_if(field.required, |dom| {
				dom.child(html!("span", {
					.class("required-indicator")
					.text(" *")
				}))
			})
		}),
		render_field_input(form_state, field),
		render_field_error(form_state, &field.name),
	];

	// Add help text if present
	if let Some(help) = &field.help_text {
		children.push(html!("div", {
			.class("form-field-help")
			.text(help)
		}));
	}

	html!("div", {
		.class("form-field")
		.children(children)
	})
}

/// Render field input based on field type
fn render_field_input(form_state: &Arc<FormState>, field: &FieldInfo) -> Dom {
	match &field.field_type {
		FieldType::Text => render_text_input(form_state, field),
		FieldType::TextArea => render_textarea_input(form_state, field),
		FieldType::Number => render_number_input(form_state, field),
		FieldType::Boolean => render_boolean_input(form_state, field),
		FieldType::Email => render_email_input(form_state, field),
		FieldType::Date => render_date_input(form_state, field),
		FieldType::DateTime => render_datetime_input(form_state, field),
		FieldType::Select { choices } => render_select_input(form_state, field, choices),
		FieldType::MultiSelect { choices } => render_multiselect_input(form_state, field, choices),
		FieldType::File => render_file_input(form_state, field),
		FieldType::Hidden => render_hidden_input(form_state, field),
	}
}

/// Render text input
fn render_text_input(form_state: &Arc<FormState>, field: &FieldInfo) -> Dom {
	let field_name = field.name.clone();
	let placeholder = field.placeholder.clone();
	html!("input" => web_sys::HtmlInputElement, {
		.class("form-control")
		.attr("type", "text")
		.attr("name", &field.name)
		.apply_if(field.readonly, |dom| dom.attr("readonly", ""))
		.apply_if(placeholder.is_some(), clone!(placeholder => move |dom| {
			dom.attr("placeholder", placeholder.as_ref().unwrap())
		}))
		.prop_signal("value", form_state.data.signal_ref(clone!(field_name => move |data| {
			data.get(&field_name)
				.and_then(|v| v.as_str())
				.unwrap_or("")
				.to_string()
		})))
		.event(clone!(form_state, field_name => move |event: events::Input| {
			let input: web_sys::HtmlInputElement = event.target().unwrap().dyn_into().unwrap();
			form_state.set_field(field_name.clone(), serde_json::Value::String(input.value()));
		}))
	})
}

/// Render textarea input
fn render_textarea_input(form_state: &Arc<FormState>, field: &FieldInfo) -> Dom {
	let field_name = field.name.clone();
	html!("textarea" => web_sys::HtmlTextAreaElement, {
		.class("form-control")
		.attr("name", &field.name)
		.attr("rows", "5")
		.apply_if(field.readonly, |dom| dom.attr("readonly", ""))
		.prop_signal("value", form_state.data.signal_ref(clone!(field_name => move |data| {
			data.get(&field_name)
				.and_then(|v| v.as_str())
				.unwrap_or("")
				.to_string()
		})))
		.event(clone!(form_state, field_name => move |event: events::Input| {
			let textarea: web_sys::HtmlTextAreaElement = event.target().unwrap().dyn_into().unwrap();
			form_state.set_field(field_name.clone(), serde_json::Value::String(textarea.value()));
		}))
	})
}

/// Render number input
fn render_number_input(form_state: &Arc<FormState>, field: &FieldInfo) -> Dom {
	let field_name = field.name.clone();
	html!("input" => web_sys::HtmlInputElement, {
		.class("form-control")
		.attr("type", "number")
		.attr("name", &field.name)
		.apply_if(field.readonly, |dom| dom.attr("readonly", ""))
		.prop_signal("value", form_state.data.signal_ref(clone!(field_name => move |data| {
			data.get(&field_name)
				.and_then(|v| v.as_i64())
				.map(|n| n.to_string())
				.unwrap_or_default()
		})))
		.event(clone!(form_state, field_name => move |event: events::Input| {
			let input: web_sys::HtmlInputElement = event.target().unwrap().dyn_into().unwrap();
			if let Ok(num) = input.value().parse::<i64>() {
				form_state.set_field(field_name.clone(), serde_json::Value::Number(num.into()));
			}
		}))
	})
}

/// Render boolean input (checkbox)
fn render_boolean_input(form_state: &Arc<FormState>, field: &FieldInfo) -> Dom {
	let field_name = field.name.clone();
	html!("input" => web_sys::HtmlInputElement, {
		.class("form-check-input")
		.attr("type", "checkbox")
		.attr("name", &field.name)
		.apply_if(field.readonly, |dom| dom.attr("disabled", ""))
		.prop_signal("checked", form_state.data.signal_ref(clone!(field_name => move |data| {
			data.get(&field_name)
				.and_then(|v| v.as_bool())
				.unwrap_or(false)
		})))
		.event(clone!(form_state, field_name => move |event: events::Change| {
			let input: web_sys::HtmlInputElement = event.target().unwrap().dyn_into().unwrap();
			form_state.set_field(field_name.clone(), serde_json::Value::Bool(input.checked()));
		}))
	})
}

/// Render email input
fn render_email_input(form_state: &Arc<FormState>, field: &FieldInfo) -> Dom {
	let field_name = field.name.clone();
	html!("input" => web_sys::HtmlInputElement, {
		.class("form-control")
		.attr("type", "email")
		.attr("name", &field.name)
		.apply_if(field.readonly, |dom| dom.attr("readonly", ""))
		.prop_signal("value", form_state.data.signal_ref(clone!(field_name => move |data| {
			data.get(&field_name)
				.and_then(|v| v.as_str())
				.unwrap_or("")
				.to_string()
		})))
		.event(clone!(form_state, field_name => move |event: events::Input| {
			let input: web_sys::HtmlInputElement = event.target().unwrap().dyn_into().unwrap();
			form_state.set_field(field_name.clone(), serde_json::Value::String(input.value()));
		}))
	})
}

/// Render date input
fn render_date_input(form_state: &Arc<FormState>, field: &FieldInfo) -> Dom {
	let field_name = field.name.clone();
	html!("input" => web_sys::HtmlInputElement, {
		.class("form-control")
		.attr("type", "date")
		.attr("name", &field.name)
		.apply_if(field.readonly, |dom| dom.attr("readonly", ""))
		.prop_signal("value", form_state.data.signal_ref(clone!(field_name => move |data| {
			data.get(&field_name)
				.and_then(|v| v.as_str())
				.unwrap_or("")
				.to_string()
		})))
		.event(clone!(form_state, field_name => move |event: events::Input| {
			let input: web_sys::HtmlInputElement = event.target().unwrap().dyn_into().unwrap();
			form_state.set_field(field_name.clone(), serde_json::Value::String(input.value()));
		}))
	})
}

/// Render datetime input
fn render_datetime_input(form_state: &Arc<FormState>, field: &FieldInfo) -> Dom {
	let field_name = field.name.clone();
	html!("input" => web_sys::HtmlInputElement, {
		.class("form-control")
		.attr("type", "datetime-local")
		.attr("name", &field.name)
		.apply_if(field.readonly, |dom| dom.attr("readonly", ""))
		.prop_signal("value", form_state.data.signal_ref(clone!(field_name => move |data| {
			data.get(&field_name)
				.and_then(|v| v.as_str())
				.unwrap_or("")
				.to_string()
		})))
		.event(clone!(form_state, field_name => move |event: events::Input| {
			let input: web_sys::HtmlInputElement = event.target().unwrap().dyn_into().unwrap();
			form_state.set_field(field_name.clone(), serde_json::Value::String(input.value()));
		}))
	})
}

/// Render select input
fn render_select_input(
	form_state: &Arc<FormState>,
	field: &FieldInfo,
	choices: &[(String, String)],
) -> Dom {
	let field_name = field.name.clone();
	let options: Vec<Dom> = choices
		.iter()
		.map(|(value, label)| {
			html!("option", {
				.attr("value", value)
				.text(label)
			})
		})
		.collect();

	html!("select" => web_sys::HtmlSelectElement, {
		.class("form-control")
		.attr("name", &field.name)
		.apply_if(field.readonly, |dom| dom.attr("disabled", ""))
		.prop_signal("value", form_state.data.signal_ref(clone!(field_name => move |data| {
			data.get(&field_name)
				.and_then(|v| v.as_str())
				.unwrap_or("")
				.to_string()
		})))
		.event(clone!(form_state, field_name => move |event: events::Change| {
			let select: web_sys::HtmlSelectElement = event.target().unwrap().dyn_into().unwrap();
			form_state.set_field(field_name.clone(), serde_json::Value::String(select.value()));
		}))
		.children(options)
	})
}

/// Render multiselect input
fn render_multiselect_input(
	_form_state: &Arc<FormState>,
	field: &FieldInfo,
	choices: &[(String, String)],
) -> Dom {
	let options: Vec<Dom> = choices
		.iter()
		.map(|(value, label)| {
			html!("option", {
				.attr("value", value)
				.text(label)
			})
		})
		.collect();

	html!("select" => web_sys::HtmlSelectElement, {
		.class("form-control")
		.attr("name", &field.name)
		.attr("multiple", "")
		.apply_if(field.readonly, |dom| dom.attr("disabled", ""))
		.children(options)
	})
}

/// Render file input with base64 encoding
///
/// This function handles file selection and encodes the file content
/// as base64 for storage in the form state. The file data is stored
/// as a JSON object with `filename`, `content_type`, and `data` fields.
fn render_file_input(form_state: &Arc<FormState>, field: &FieldInfo) -> Dom {
	let field_name = field.name.clone();

	html!("div", {
		.class("file-input-container")
		.children(&mut [
			html!("input" => web_sys::HtmlInputElement, {
				.class("form-control")
				.attr("type", "file")
				.attr("name", &field.name)
				.apply_if(field.readonly, |dom| dom.attr("disabled", ""))
				.event(clone!(form_state, field_name => move |event: events::Change| {
					let input: web_sys::HtmlInputElement = event.target().unwrap().dyn_into().unwrap();

					if let Some(files) = input.files()
						&& let Some(file) = files.get(0) {
							let form_state = Arc::clone(&form_state);
							let field_name = field_name.clone();
							let filename = file.name();
							let content_type = file.type_();

							// Create FileReader to read file as base64
							let reader = web_sys::FileReader::new().unwrap();

							// Set up onload callback
							let onload_callback = {
								let reader_clone = reader.clone();
								let form_state = Arc::clone(&form_state);
								let field_name = field_name.clone();
								let filename = filename.clone();
								let content_type = content_type.clone();

								Closure::wrap(Box::new(move |_event: web_sys::ProgressEvent| {
									if let Ok(result) = reader_clone.result()
										&& let Some(data_url) = result.as_string() {
											// Extract base64 data from data URL
											// Format: "data:content/type;base64,BASE64DATA"
											let base64_data = if let Some(comma_idx) = data_url.find(',') {
												&data_url[comma_idx + 1..]
											} else {
												&data_url
											};

											// Create file metadata object
											let file_obj = serde_json::json!({
												"filename": filename,
												"content_type": content_type,
												"data": base64_data,
											});

											form_state.set_field(field_name.clone(), file_obj);
										}
								}) as Box<dyn FnMut(_)>)
							};

							reader.set_onload(Some(onload_callback.as_ref().unchecked_ref()));
							onload_callback.forget(); // Prevent callback from being dropped

							// Start reading the file as data URL (base64)
							let _ = reader.read_as_data_url(&file);
						}
				}))
			}),
			// Display selected filename
			html!("div", {
				.class("file-input-status")
				.style("font-size", "12px")
				.style("color", "#666")
				.style("margin-top", "4px")
				.child_signal(form_state.data.signal_ref(clone!(field_name => move |data| {
					data.get(&field_name)
						.and_then(|v| v.get("filename"))
						.and_then(|f| f.as_str())
						.map(|filename| {
							html!("span", {
								.text(&format!("Selected: {}", filename))
							})
						})
				})))
			}),
		])
	})
}

/// Render hidden input
fn render_hidden_input(form_state: &Arc<FormState>, field: &FieldInfo) -> Dom {
	let field_name = field.name.clone();
	html!("input" => web_sys::HtmlInputElement, {
		.attr("type", "hidden")
		.attr("name", &field.name)
		.prop_signal("value", form_state.data.signal_ref(clone!(field_name => move |data| {
			data.get(&field_name)
				.and_then(|v| v.as_str())
				.unwrap_or("")
				.to_string()
		})))
	})
}

/// Render field-level error
fn render_field_error(form_state: &Arc<FormState>, field_name: &str) -> Dom {
	let field_name_clone = field_name.to_string();
	html!("div", {
		.class("form-field-error")
		.child_signal(form_state.field_errors.signal_ref(clone!(field_name_clone => move |errors| {
			errors.get(&field_name_clone).map(|err_msg| {
				html!("span", {
					.class("error-text")
					.text(err_msg)
				})
			})
		})))
	})
}

/// Render form action buttons (Save, Cancel)
fn render_form_actions(form_state: &Arc<FormState>, fields: &[FieldInfo], model_name: &str) -> Dom {
	let fields_clone = fields.to_vec();
	let model_name_clone = model_name.to_string();

	html!("div", {
		.class("form-actions")
		.visible_signal(form_state.is_loading.signal().map(|loading| !loading))
		.children(&mut [
			html!("button", {
				.class("btn btn-primary")
				.text("Save")
				.event(clone!(form_state, fields_clone, model_name_clone => move |_: events::Click| {
					form_state.submit(&fields_clone, clone!(model_name_clone => move |_response| {
						// Navigate back on success
						let window = web_sys::window().unwrap();
						let location = window.location();
						location.set_hash(&format!("#/{}", model_name_clone)).unwrap();
					}));
				}))
			}),
			html!("button", {
				.class("btn btn-secondary")
				.text("Cancel")
				.event(|_: events::Click| {
					let window = web_sys::window().unwrap();
					let history = window.history().unwrap();
					history.back().unwrap();
				})
			}),
		])
	})
}

/// Get mock field definitions for a model (Phase 3 - will be dynamic in Phase 4)
fn get_mock_fields(model_name: &str) -> Vec<FieldInfo> {
	// Mock fields for demonstration
	match model_name {
		"User" => vec![
			FieldInfo {
				name: "username".to_string(),
				label: "Username".to_string(),
				field_type: FieldType::Text,
				required: true,
				readonly: false,
				help_text: Some("Enter a unique username".to_string()),
				placeholder: Some("johndoe".to_string()),
			},
			FieldInfo {
				name: "email".to_string(),
				label: "Email Address".to_string(),
				field_type: FieldType::Email,
				required: true,
				readonly: false,
				help_text: None,
				placeholder: Some("user@example.com".to_string()),
			},
			FieldInfo {
				name: "is_active".to_string(),
				label: "Active".to_string(),
				field_type: FieldType::Boolean,
				required: false,
				readonly: false,
				help_text: None,
				placeholder: None,
			},
		],
		_ => vec![
			FieldInfo {
				name: "name".to_string(),
				label: "Name".to_string(),
				field_type: FieldType::Text,
				required: true,
				readonly: false,
				help_text: None,
				placeholder: None,
			},
			FieldInfo {
				name: "description".to_string(),
				label: "Description".to_string(),
				field_type: FieldType::TextArea,
				required: false,
				readonly: false,
				help_text: None,
				placeholder: None,
			},
		],
	}
}
