//! page! macro accessibility validation failures.

// reinhardt-fmt: ignore-all

use reinhardt_pages::page;
use reinhardt_pages::component::Page;

#[derive(bon::Builder)]
struct WrapperProps {
	children: Option<Page>,
}

fn wrapper(_p: WrapperProps) -> Page {
	page!(|| {
		div {}
	})()
}

fn main() {
	let _unlabelled_input = page!(|| {
		input {
			r#type: "text",
			name: "query",
		}
	});

	let _empty_wrapping_label = page!(|| {
		label {
			input {
				r#type: "text",
				name: "email",
			}
		}
	});

	let _empty_for_label = page!(|| {
		label {
			r#for: "email-empty",
		}
		input {
			id: "email-empty",
			r#type: "text",
			name: "email",
		}
	});

	let _component_child_label_does_not_label_sibling = page!(|| {
		Wrapper {
			label {
				r#for: "component-email",
				"Email"
			}
		}
		input {
			id: "component-email",
			r#type: "text",
			name: "email",
		}
	});

	let _empty_button = page!(|| {
		button {}
	});

	let _empty_link = page!(|| {
		a { href: "/missing-name" }
	});

	let _invalid_role = page!(|| {
		div {
			role: "made-up-role",
			"Content"
		}
	});

	let _positive_tabindex = page!(|| {
		div {
			tabindex: 3,
			"Content"
		}
	});

	let _iframe_missing_title = page!(|| {
		iframe { src: "/embed" }
	});

	let _invalid_opt_out = page!(|| {
		input {
			r#type: "range",
			name: "decorative",
			a11y: true,
		}
	});
}
