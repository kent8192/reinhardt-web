//! page! macro with elements having many attributes
//!
//! This test verifies that the page! macro can handle elements with
//! a large number of attributes (boundary value testing for attribute count).

use reinhardt_pages::page;

fn main() {
	// Element with 20+ standard attributes
	let _many_attrs = page!(|| {
		div {
			id: "main-container",
			class: "container fluid responsive",
			style: "color: red; background: blue;",
			title: "Main Container",
			lang: "en",
			dir: "ltr",
			tabindex: 0,
			role: "main",
			contenteditable: "true",
			draggable: "true",
			spellcheck: "true",
			translate: "yes",
			aria_label: "Main content area",
			aria_describedby: "desc1",
			aria_live: "polite",
			aria_atomic: "true",
			aria_relevant: "additions text",
			aria_busy: "false",
			data_testid: "main-container",
			data_component: "container",
			data_version: "1.0.0",
			data_feature: "responsive",
			data_theme: "light",
			"Content"
		}
	});

	// Input with 25+ attributes
	let _complex_input = page!(|| {
		input {
			r#type: "text",
			id: "username",
			name: "username",
			class: "form-control input-lg",
			placeholder: "Enter username",
			value: "",
			maxlength: 50,
			minlength: 3,
			pattern: "[a-zA-Z0-9]+",
			autocomplete: "username",
			tabindex: 1,
			title: "Username input field",
			aria_label: "Username",
			aria_describedby: "username-help",
			aria_required: "true",
			aria_invalid: "false",
			data_testid: "username-input",
			data_validation: "alphanumeric",
			data_min: "3",
			data_max: "50",
			data_field: "username",
			data_track: "true",
		}
	});

	// Button with 30+ attributes and events
	let _complex_button = page!(|| {
		button {
			r#type: "submit",
			id: "submit-btn",
			name: "submit",
			class: "btn btn-primary btn-lg",
			value: "submit",
			tabindex: 2,
			title: "Submit form",
			aria_label: "Submit button",
			aria_describedby: "submit-help",
			aria_pressed: "false",
			aria_expanded: "false",
			aria_haspopup: "false",
			aria_controls: "form-1",
			role: "button",
			data_testid: "submit-button",
			data_action: "submit",
			data_form: "registration",
			data_validate: "true",
			data_track: "click",
			data_category: "form",
			data_label: "submit",
			data_value: "1",
			data_loading: "false",
			data_success: "false",
			data_error: "false",
			@click: |_| { },
			@mouseenter: |_| { },
			@mouseleave: |_| { },
			@focus: |_| { },
			@blur: |_| { },
			"Submit"
		}
	});

	// Form with many attributes
	let _complex_form = page!(|| {
		form {
			id: "registration-form",
			name: "registration",
			class: "form form-horizontal",
			method: "post",
			action: "/register",
			enctype: "application/x-www-form-urlencoded",
			autocomplete: "on",
			target: "_self",
			accept_charset: "UTF-8",
			role: "form",
			aria_label: "Registration form",
			aria_describedby: "form-desc",
			data_testid: "registration-form",
			data_version: "2.0",
			data_validation: "client-server",
			data_steps: "3",
			data_current_step: "1",
			data_track: "true",
			data_analytics: "enabled",
			@submit: |_| { },
			@change: |_| { },
			div {
				"Form content"
			}
		}
	});

	// Link with many attributes
	let _complex_link = page!(|| {
		a {
			id: "main-link",
			href: "/page",
			class: "link external",
			title: "Go to page",
			target: "_blank",
			rel: "noopener noreferrer",
			hreflang: "en",
			r#type: "text/html",
			download: "file.pdf",
			ping: "/track",
			referrerpolicy: "no-referrer",
			role: "link",
			tabindex: 0,
			aria_label: "External link",
			aria_describedby: "link-desc",
			aria_current: "page",
			data_testid: "main-link",
			data_track: "click",
			data_category: "navigation",
			data_external: "true",
			data_analytics: "enabled",
			data_label: "main-link",
			@click: |_| { },
			@mouseenter: |_| { },
			@mouseleave: |_| { },
			"Click here"
		}
	});

	// Image with all recommended attributes
	let _complex_img = page!(|| {
		img {
			id: "hero-image",
			src: "/images/hero.jpg",
			alt: "Hero image showing the product",
			class: "img-fluid responsive",
			width: "800",
			height: "600",
			loading: "lazy",
			decoding: "async",
			crossorigin: "anonymous",
			referrerpolicy: "no-referrer",
			srcset: "/images/hero-400.jpg 400w, /images/hero-800.jpg 800w",
			sizes: "(max-width: 600px) 400px, 800px",
			usemap: "#map",
			title: "Product hero image",
			role: "img",
			aria_label: "Product showcase",
			aria_describedby: "img-desc",
			data_testid: "hero-image",
			data_category: "product",
			data_lazy: "true",
			data_priority: "high",
			data_track: "view",
		}
	});
}
