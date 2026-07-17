//! Standard intrinsic events accept typed sync, async, external, and Callback handlers.

use reinhardt_pages::event::{ClickEvent, InputEvent, KeyDownEvent, SubmitEvent};
use reinhardt_pages::{Callback, page};

fn handle_submit(event: SubmitEvent) {
	event.prevent_default();
}

fn main() {
	reinhardt_core::reactive::ReactiveScope::run(|| {
		let _inferred = page!(|| {
			button {
				@click: |event| {
					let _: ClickEvent = event;
				},
				"Click"
			},
		});
		let _explicit = page!(|| {
			input {
				aria_label: "Input",
				@input: |event: InputEvent| {
					let _ = event.value();
				},
			},
		});
		let _async = page!(|| {
			input {
				aria_label: "Keyboard",
				@keydown: async |event| {
					let _: KeyDownEvent = event;
				},
			},
		});
		let _external = page!(|| {
			form {
				@submit: crate::handle_submit,
			}
		});
		let _zero_argument = page!(|| {
			button {
				@click: || {},
				"Click"
			}
		});
		let _zero_argument_async = page!(|| {
			button {
				@click: async || {},
				"Click"
			}
		});

		let callback = Callback::new(|event: ClickEvent| event.prevent_default());
		let _callback = page!(|callback: Callback<ClickEvent, ()>| {
			button {
				@click: callback,
				"Click"
			}
		})(callback);
	});
}
