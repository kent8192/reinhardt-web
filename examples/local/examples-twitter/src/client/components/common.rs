//! Common UI components
//!
//! Provides reusable UI components for the Twitter clone application:
//! - `ButtonVariant` - Button style variants
//! - `button` - Styled button with click handler
//! - `loading_spinner` - Loading indicator
//! - `error_alert` - Error message display
//! - `success_alert` - Success message display
//! - `text_input` - Form text input
//! - `textarea` - Form textarea with character count
//! - `avatar` - User avatar image
//!
//! ## Design Note
//!
//! These components use the `page!` macro for JSX-like syntax.
//! Interactive components with event handlers are hydrated on the client side.

use reinhardt_pages::Signal;
use reinhardt_pages::component::{ElementView, IntoView, View};
use reinhardt_pages::page;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

/// Button variant styles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
	/// Primary action button (blue)
	Primary,
	/// Secondary action button (gray)
	Secondary,
	/// Success action button (green)
	Success,
	/// Danger action button (red)
	Danger,
	/// Warning action button (yellow)
	Warning,
	/// Link style button (no background)
	Link,
	/// Outline primary button
	OutlinePrimary,
}

impl ButtonVariant {
	/// Get Bootstrap CSS class for this variant
	pub fn class(&self) -> &'static str {
		match self {
			ButtonVariant::Primary => "btn btn-primary",
			ButtonVariant::Secondary => "btn btn-secondary",
			ButtonVariant::Success => "btn btn-success",
			ButtonVariant::Danger => "btn btn-danger",
			ButtonVariant::Warning => "btn btn-warning",
			ButtonVariant::Link => "btn btn-link",
			ButtonVariant::OutlinePrimary => "btn btn-outline-primary",
		}
	}
}

/// Button component
///
/// Displays a styled button with various variants.
/// When clicked, sets the provided Signal to true.
///
/// # Arguments
///
/// * `text` - Button label text
/// * `variant` - Visual style variant
/// * `disabled` - Whether the button is disabled
/// * `on_click` - Signal that will be set to true when clicked
pub fn button(text: &str, variant: ButtonVariant, disabled: bool, on_click: Signal<bool>) -> View {
	let class = if disabled {
		format!("{} disabled", variant.class())
	} else {
		variant.class().to_string()
	};
	let text = text.to_string();
	let disabled_attr = if disabled { "true" } else { "" }.to_string();

	#[cfg(target_arch = "wasm32")]
	{
		let on_click_clone = on_click.clone();
		page!(|class: String, text: String, disabled_attr: String| {
			button {
				class: class,
				r#type: "button",
				disabled: disabled_attr,
				@click: { let on_click = on_click_clone.clone(); move |_event| { on_click.set(true); } },
				{ text }
			}
		})(class, text, disabled_attr)
	}

	#[cfg(not(target_arch = "wasm32"))]
	{
		let _ = on_click; // Suppress unused warning
		page!(|class: String, text: String, disabled_attr: String| {
			button {
				class: { class },
				r#type: "button",
				disabled: { disabled_attr },
				data_reactive: "true",
				{ text }
			}
		})(class, text, disabled_attr)
	}
}

/// Loading spinner component
///
/// Displays a Bootstrap spinner animation while content is loading.
pub fn loading_spinner() -> View {
	page!(|| {
		div {
			class: "text-center py-5",
			div {
				class: "spinner-border",
				role: "status",
				span {
					class: "visually-hidden",
					"Loading..."
				}
			}
		}
	})()
}

/// Error alert component
///
/// Displays an error message in a styled alert box.
///
/// # Arguments
///
/// * `message` - Error message to display
/// * `dismissible` - Whether the alert can be dismissed
pub fn error_alert(message: &str, dismissible: bool) -> View {
	let message = message.to_string();
	if dismissible {
		page!(|message: String| {
			div {
				class: "alert alert-danger alert-dismissible fade show",
				role: "alert",
				{ message }
				button {
					r#type: "button",
					class: "btn-close",
					data_bs_dismiss: "alert",
					aria_label: "Close",
				}
			}
		})(message)
	} else {
		page!(|message: String| {
			div {
				class: "alert alert-danger",
				role: "alert",
				{ message }
			}
		})(message)
	}
}

/// Success alert component
///
/// Displays a success message in a styled alert box.
///
/// # Arguments
///
/// * `message` - Success message to display
pub fn success_alert(message: &str) -> View {
	let message = message.to_string();
	page!(|message: String| {
		div {
			class: "alert alert-success",
			role: "alert",
			{ message }
		}
	})(message)
}

/// Text input component
///
/// Displays a labeled text input field.
///
/// # Arguments
///
/// * `id` - Input element ID
/// * `label` - Label text
/// * `placeholder` - Placeholder text
/// * `input_type` - HTML input type (e.g., "text", "email", "password")
/// * `value` - Reactive value signal
/// * `required` - Whether the field is required
pub fn text_input(
	id: &str,
	label: &str,
	placeholder: &str,
	input_type: &str,
	value: Signal<String>,
	required: bool,
) -> View {
	let current_value = value.get();
	let id_owned = id.to_string();
	let placeholder_owned = placeholder.to_string();
	let input_type_owned = input_type.to_string();
	let label_owned = label.to_string();
	let required_attr = if required { "true" } else { "" }.to_string();

	#[cfg(target_arch = "wasm32")]
	{
		let value_clone = value.clone();
		page!(|id_owned: String, label_owned: String, input_type_owned: String, placeholder_owned: String, current_value: String, required_attr: String| {
			div {
				class: "mb-3",
				label {
					r#for: id_owned.clone(),
					class: "form-label",
					{ label_owned }
				}
				input {
					r#type: input_type_owned,
					class: "form-control",
					id: id_owned.clone(),
					name: id_owned,
					placeholder: placeholder_owned,
					value: current_value,
					required: required_attr,
					@input: { let value = value_clone.clone(); move |event : web_sys::Event| { if let Some(target) = event.target() { if let Ok(input_el) = target.dyn_into ::<web_sys::HtmlInputElement>() { value.set(input_el.value()); } } } },
				}
			}
		})(
			id_owned,
			label_owned,
			input_type_owned,
			placeholder_owned,
			current_value,
			required_attr,
		)
	}

	#[cfg(not(target_arch = "wasm32"))]
	{
		let _ = value; // Suppress unused warning
		page!(|id_owned: String, label_owned: String, input_type_owned: String, placeholder_owned: String, current_value: String, required_attr: String| {
			div {
				class: "mb-3",
				label {
					r#for: { id_owned.clone() },
					class: "form-label",
					{ label_owned }
				}
				input {
					r#type: { input_type_owned },
					class: "form-control",
					id: { id_owned.clone() },
					name: { id_owned },
					placeholder: { placeholder_owned },
					value: { current_value },
					required: { required_attr },
					data_reactive: "true",
				}
			}
		})(
			id_owned,
			label_owned,
			input_type_owned,
			placeholder_owned,
			current_value,
			required_attr,
		)
	}
}

/// Textarea component with character count
///
/// Displays a labeled textarea with optional character limit display.
///
/// # Arguments
///
/// * `id` - Textarea element ID
/// * `label` - Label text
/// * `placeholder` - Placeholder text
/// * `rows` - Number of visible rows
/// * `max_length` - Maximum character length (0 for no limit)
/// * `value` - Reactive value signal
pub fn textarea(
	id: &str,
	label: &str,
	placeholder: &str,
	rows: u32,
	max_length: usize,
	value: Signal<String>,
) -> View {
	let current_value = value.get();
	let char_count = current_value.len();
	let id_owned = id.to_string();
	let placeholder_owned = placeholder.to_string();
	let label_owned = label.to_string();
	let rows_str = rows.to_string();

	// Determine character count display class
	let count_class = if max_length > 0 {
		if char_count > max_length {
			"text-danger"
		} else if char_count > max_length * 9 / 10 {
			"text-warning"
		} else {
			"text-muted"
		}
		.to_string()
	} else {
		String::new()
	};
	let count_text = if max_length > 0 {
		format!("{}/{}", char_count, max_length)
	} else {
		String::new()
	};
	let maxlength_attr = if max_length > 0 {
		max_length.to_string()
	} else {
		String::new()
	};
	let show_count = max_length > 0;

	#[cfg(target_arch = "wasm32")]
	{
		let value_clone = value.clone();
		page!(|id_owned: String, label_owned: String, rows_str: String, placeholder_owned: String, current_value: String, maxlength_attr: String, show_count: bool, count_class: String, count_text: String| {
			div {
				class: "mb-3",
				label {
					r#for: id_owned.clone(),
					class: "form-label",
					{ label_owned }
				}
				textarea {
					class: "form-control",
					id: id_owned.clone(),
					name: id_owned,
					rows: rows_str,
					placeholder: placeholder_owned,
					maxlength: maxlength_attr,
					@input: { let value = value_clone.clone(); move |event : web_sys::Event| { if let Some(target) = event.target() { if let Ok(textarea_el) = target.dyn_into ::<web_sys::HtmlTextAreaElement>() { value.set(textarea_el.value()); } } } },
					{ current_value }
				}
				if show_count {
					small {
						class: count_class,
						{ count_text }
					}
				}
			}
		})(
			id_owned,
			label_owned,
			rows_str,
			placeholder_owned,
			current_value,
			maxlength_attr,
			show_count,
			count_class,
			count_text,
		)
	}

	#[cfg(not(target_arch = "wasm32"))]
	{
		let _ = value; // Suppress unused warning
		page!(|id_owned: String, label_owned: String, rows_str: String, placeholder_owned: String, current_value: String, maxlength_attr: String, show_count: bool, count_class: String, count_text: String| {
			div {
				class: "mb-3",
				label {
					r#for: { id_owned.clone() },
					class: "form-label",
					{ label_owned }
				}
				textarea {
					class: "form-control",
					id: { id_owned.clone() },
					name: { id_owned },
					rows: { rows_str },
					placeholder: { placeholder_owned },
					maxlength: { maxlength_attr },
					data_reactive: "true",
					{ current_value }
				}
				if show_count {
					small {
						class: { count_class },
						{ count_text }
					}
				}
			}
		})(
			id_owned,
			label_owned,
			rows_str,
			placeholder_owned,
			current_value,
			maxlength_attr,
			show_count,
			count_class,
			count_text,
		)
	}
}

/// Avatar component
///
/// Displays a user avatar image with fallback.
///
/// # Arguments
///
/// * `url` - Avatar image URL (None for default avatar)
/// * `alt` - Alt text for the image
/// * `size` - Size in pixels
pub fn avatar(url: Option<&str>, alt: &str, size: u32) -> View {
	let src = url
		.map(|s| s.to_string())
		.unwrap_or_else(|| "https://via.placeholder.com/150?text=User".to_string());
	let alt_owned = alt.to_string();
	let size_str = format!("{}px", size);

	// Use ElementView instead of page! macro for dynamic src attribute
	ElementView::new("img")
		.attr("src", src)
		.attr("alt", alt_owned)
		.attr("class", "rounded-circle")
		.attr("width", size_str.clone())
		.attr("height", size_str)
		.attr("style", "object-fit: cover;")
		.into_view()
}

/// Empty placeholder component
///
/// Displays an empty div (useful for conditional rendering).
pub fn empty() -> View {
	page!(|| { div {} })()
}
