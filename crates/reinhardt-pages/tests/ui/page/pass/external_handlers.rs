//! page! macro with external event handlers.
//!
//! This test verifies that event handlers can be defined outside the page!
//! macro and referenced inside it using the @event syntax.
//!
//! Spec §3.7 (no implicit captures): outer bindings must be declared as
//! explicit closure parameters. Event handlers travel via `Callback` typed
//! parameters instead of being captured.

use reinhardt_pages::{Callback, page, use_shared_state};

fn main() {
	// External Callback used in a single page
	let handle_click = Callback::new(|_| {});
	let _external_callback = page!(|handle_click: Callback| {
		button {
			@click: handle_click,
			"Click me"
		}
	})(handle_click);

	// External Callback used as a form handler
	let handle_submit = Callback::new(|_| {});
	let _external_submit = page!(|handle_submit: Callback| {
		form {
			@submit: handle_submit,
			button {
				"Submit"
			}
		}
	})(handle_submit);

	// Mixed: external Callback + inline closure
	let external_handler = Callback::new(|_| {});
	let _mixed = page!(|external_handler: Callback| {
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
	})(external_handler);

	// Cloned Callback used in multiple elements
	let shared_handler = Callback::new(|_| {});
	let handler1 = shared_handler.clone();
	let handler2 = shared_handler.clone();
	let _shared = page!(|handler1: Callback, handler2: Callback| {
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
	})(handler1, handler2);

	// Handler with thread-safe captured state
	let (counter, set_counter) = use_shared_state(0);
	let increment = Callback::new({
		let counter = counter.clone();
		let set_counter = set_counter.clone();
		move |_| {
			set_counter(counter.get() + 1);
		}
	});
	let _with_state = page!(|increment: Callback| {
		button {
			@click: increment,
			"Increment"
		}
	})(increment);
}
