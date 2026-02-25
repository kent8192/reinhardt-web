//! page! macro with external event handlers
//!
//! This test verifies that event handlers can be defined outside the page! macro
//! and referenced inside it using the @event syntax.

use reinhardt_pages::{Callback, page, use_shared_state};

fn main() {
	// External closure reference
	let handle_click = |_| {};
	let _external_closure = __reinhardt_placeholder__!(/*0*/);

	// External Callback type
	let handle_submit = Callback::new(|_| {});
	let _external_callback = __reinhardt_placeholder__!(/*1*/);

	// Mixed: inline and external handlers
	let external_handler = |_| {};
	let _mixed = __reinhardt_placeholder__!(/*2*/);

	// Cloned Callback used in multiple elements
	let shared_handler = Callback::new(|_| {});
	let handler1 = shared_handler.clone();
	let handler2 = shared_handler.clone();
	let _shared = __reinhardt_placeholder__!(/*3*/);

	// Handler with thread-safe captured state
	let (counter, set_counter) = use_shared_state(0);
	let increment = {
		let counter = counter.clone();
		let set_counter = set_counter.clone();
		move |_| {
			set_counter(counter.get() + 1);
		}
	};
	let _with_state = __reinhardt_placeholder__!(/*4*/);
}
