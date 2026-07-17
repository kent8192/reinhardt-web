use reinhardt_core::types::page::NumberParseError;
use reinhardt_pages::page;
use reinhardt_pages::reactive::{ReactiveScope, Signal};

fn main() {
	ReactiveScope::run(|| {
		let text = Signal::new(String::new());
		let checked = Signal::new(false);
		let radio = Signal::new("draft".to_owned());
		let number = Signal::new(0_i64);
		let number_error = Signal::new(None::<NumberParseError>);
		let selected = Signal::new(String::new());
		let selected_many = Signal::new(Vec::<String>::new());

		let _ = page!({
			input {
				a11y: off,
				bind: text
			}
			textarea {
				a11y: off,
				bind: text
			}
			input {
				a11y: off,
				type: "checkbox",
				bind: checked
			}
			input {
				a11y: off,
				type: "radio",
				value: "draft",
				bind: radio
			}
			input {
				a11y: off,
				type: "number",
				bind: number(number, number_error)
			}
			select {
				a11y: off,
				bind: selected,
				option {
					value: "a",
					"A"
				}
			}
			select {
				a11y: off,
				multiple: true,
				bind: selected_many,
				option {
					value: "a",
					"A"
				}
			}
		});
	});
}
