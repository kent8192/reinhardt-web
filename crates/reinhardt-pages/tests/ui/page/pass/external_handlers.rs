//! page! macro with external event handlers
//!
//! This test verifies that event handlers can be defined outside the page! macro
//! and referenced inside it using the @event syntax.

use reinhardt_pages::{Callback, page, use_shared_state};

fn main() {
	// External closure reference
	let handle_click = |_| {};
	let _external_closure = page!(|| {
		button {
			@click: handle_click,
			"Click me"
		}
	});

	// External Callback type
	let handle_submit = Callback::new(|_| {});
	let _external_callback = page!(|| {
		form {
			@submit: handle_submit,
			button {
				"Submit"
			}
		}
	});

	// Mixed: inline and external handlers
	let external_handler = |_| {};
	let _mixed = page!(|| {
		div {
			button {
				@click: external_handler,
				"External"
			}
			button {
				@click: |_| { },
				"Inline"
			}
		}
	});

	// Cloned Callback used in multiple elements
	let shared_handler = Callback::new(|_| {});
	let handler1 = shared_handler.clone();
	let handler2 = shared_handler.clone();
	let _shared = page!(|| {
		div {
			button {
				@click: handler1,
				"Button 1"
			}
			button {
				@click: handler2,
				"Button 2"
			}
		}
	});

	// Handler with thread-safe captured state
	let (counter, set_counter) = use_shared_state(0);
	let increment = {
		let counter = counter.clone();
		let set_counter = set_counter.clone();
		move |_| {
			set_counter(counter.get() + 1);
		}
	};
	let _with_state = page!(|| {
		button {
			@click: increment,
			"Increment"
		}
	});
}
