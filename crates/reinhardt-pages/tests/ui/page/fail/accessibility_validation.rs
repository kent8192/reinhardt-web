//! page! macro accessibility validation failures.

// reinhardt-fmt: ignore-all

use reinhardt_pages::page;

fn main() {
	let _unlabelled_input = page!(|| {
		input {
			r#type: "text",
			name: "query",
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
