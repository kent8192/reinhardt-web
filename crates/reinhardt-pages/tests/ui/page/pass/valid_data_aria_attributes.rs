//! page! macro with valid data-* and aria-* attributes
//!
//! This test verifies that properly formatted data-* and aria-* attributes
//! are accepted by the validator.

use reinhardt_pages::page;

fn main() {
	// Valid data-* attributes (lowercase, hyphen-separated)
	let _valid_data = page!(|| {
		div {
			data_testid: "component-1",
			data_component: "button",
			data_version: "1.0",
			data_feature_flag: "enabled",
			data_is_active: "true",
			data_user_id: "12345",
			data_created_at: "2024-01-01",
			data_custom_attribute: "value",
			data_multi_word_name: "test",
			data_a: "single letter suffix",
			data_test123: "alphanumeric",
			data_snake_case_name: "snake-case",
			"Content"
		}
	});

	// Valid aria-* attributes (lowercase, hyphen-separated)
	let _valid_aria = page!(|| {
		div {
			aria_label: "Main content",
			aria_labelledby: "label-id",
			aria_describedby: "desc-id",
			aria_hidden: "false",
			aria_expanded: "true",
			aria_controls: "menu-1",
			aria_haspopup: "true",
			aria_pressed: "false",
			aria_selected: "false",
			aria_checked: "true",
			aria_disabled: "false",
			aria_readonly: "false",
			aria_required: "true",
			aria_invalid: "false",
			aria_live: "polite",
			aria_atomic: "true",
			aria_relevant: "additions text",
			aria_busy: "false",
			aria_current: "page",
			aria_valuenow: "50",
			aria_valuemin: "0",
			aria_valuemax: "100",
			aria_valuetext: "50 percent",
			"Accessible content"
		}
	});

	// Mixed data-* and aria-* attributes
	let _mixed_attrs = page!(|| {
		button {
			aria_label: "Close dialog",
			aria_pressed: "false",
			data_testid: "close-button",
			data_dialog_id: "modal-1",
			data_action: "close",
			@click: |_| { },
			"Close"
		}
	});

	// Complex interactive component with accessibility
	let _accessible_component = page!(|| {
		div {
			role: "tablist",
			aria_label: "Content tabs",
			data_component: "tabs",
			data_version: "2.0",
			button {
				role: "tab",
				aria_selected: "true",
				aria_controls: "panel-1",
				data_tab: "tab-1",
				data_index: "0",
				@click: |_| { },
				"Tab 1"
			}
			button {
				role: "tab",
				aria_selected: "false",
				aria_controls: "panel-2",
				data_tab: "tab-2",
				data_index: "1",
				@click: |_| { },
				"Tab 2"
			}
			div {
				role: "tabpanel",
				aria_labelledby: "tab-1",
				data_panel: "panel-1",
				"Panel 1 content"
			}
		}
	});

	// Form with validation and accessibility
	let _accessible_form = page!(|| {
		form {
			aria_label: "Login form",
			data_form_type: "login",
			data_validation: "client",
			div {
				label {
					r#for: "email",
					"Email"
				}
				input {
					r#type: "email",
					id: "email",
					name: "email",
					aria_required: "true",
					aria_invalid: "false",
					aria_describedby: "email-error",
					data_field: "email",
					data_validation_type: "email",
					data_required: "true",
				}
				span {
					id: "email-error",
					aria_live: "polite",
					data_error_for: "email",
					""
				}
			}
		}
	});

	// Navigation with ARIA landmarks
	let _accessible_nav = page!(|| {
		nav {
			aria_label: "Main navigation",
			data_component: "navbar",
			ul {
				role: "menubar",
				aria_orientation: "horizontal",
				li {
					role: "none",
					a {
						href: "/home",
						role: "menuitem",
						aria_current: "page",
						data_nav_item: "home",
						"Home"
					}
				}
				li {
					role: "none",
					a {
						href: "/about",
						role: "menuitem",
						data_nav_item: "about",
						"About"
					}
				}
			}
		}
	});

	// Modal dialog with full accessibility
	let _accessible_modal = page!(|| {
		div {
			role: "dialog",
			aria_modal: "true",
			aria_labelledby: "dialog-title",
			aria_describedby: "dialog-desc",
			data_component: "modal",
			data_modal_id: "confirm-dialog",
			data_open: "true",
			div {
				h2 {
					id: "dialog-title",
					"Confirm Action"
				}
				p {
					id: "dialog-desc",
					"Are you sure you want to proceed?"
				}
				div {
					button {
						aria_label: "Confirm",
						data_action: "confirm",
						@click: |_| { },
						"Confirm"
					}
					button {
						aria_label: "Cancel",
						data_action: "cancel",
						@click: |_| { },
						"Cancel"
					}
				}
			}
		}
	});

	// Progress indicator with ARIA
	let _accessible_progress = page!(|| {
		div {
			role: "progressbar",
			aria_valuenow: "75",
			aria_valuemin: "0",
			aria_valuemax: "100",
			aria_valuetext: "75 percent complete",
			aria_label: "Upload progress",
			data_component: "progress",
			data_percentage: "75",
			data_status: "uploading",
			div {
				style: "width: 75%",
				"75%"
			}
		}
	});
}
