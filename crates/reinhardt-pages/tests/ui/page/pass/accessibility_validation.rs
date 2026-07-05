//! page! macro accessibility validation pass cases.

// reinhardt-fmt: ignore-all

use reinhardt_pages::page;

fn main() {
	let labelled_by_for_after_control = page!(|| {
		input {
			id: "search",
			r#type: "search",
			name: "search",
		}
		label {
			r#for: "search",
			"Search"
		}
	});

	let wrapped_control = page!(|| {
		label {
			"Volume"
			input {
				r#type: "range",
				name: "volume",
			}
		}
	});

	let aria_labelled_controls = page!(|| {
		h2 {
			id: "notes-heading",
			"Notes"
		}
		select {
			name: "country",
			aria_label: "Country",
			option { "Japan" }
		}
		textarea {
			name: "notes",
			aria_labelledby: "notes-heading",
			""
		}
	});

	let named_interactions = page!(|| {
		button {
			img {
				src: "/icons/save.svg",
				alt: "Save",
			}
		}
		a {
			href: "/settings",
			aria_label: "Settings",
		}
	});

	let input_button_names = page!(|| {
		input {
			r#type: "submit",
			value: "Save",
		}
		input {
			r#type: "submit",
		}
		input {
			r#type: "reset",
		}
		input {
			r#type: "image",
			src: "/icons/save.svg",
			alt: "Save",
		}
	});

	let hidden_or_noninteractive = page!(|| {
		input {
			hidden: true,
			name: "token",
		}
		button {
			hidden: true,
		}
		div {
			hidden: true,
			input {
				r#type: "text",
				name: "hidden-query",
			}
			select {
				name: "hidden-country",
			}
			button {}
		}
		a {
			id: "top",
		}
	});

	let role_tabindex_and_iframe = page!(|| {
		div {
			role: "region",
			tabindex: 0,
			"Content"
		}
		div {
			role: "comment",
			"Review note"
		}
		div {
			tabindex: -1,
			"Programmatic focus target"
		}
		iframe {
			src: "/preview",
			title: "Preview frame",
		}
	});

	let opt_out = page!(|| {
		input {
			r#type: "range",
			name: "decorative",
			a11y: off,
		}
		img {
			src: "/decorative.svg",
			a11y: off,
		}
	});

	let _ = (
		labelled_by_for_after_control,
		wrapped_control,
		aria_labelled_controls,
		named_interactions,
		input_button_names,
		hidden_or_noninteractive,
		role_tabindex_and_iframe,
		opt_out,
	);
}
