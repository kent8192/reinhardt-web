//! page! macro with all common DOM event types
//!
//! This test verifies that the page! macro supports all common DOM event handlers
//! with proper closure syntax.

use reinhardt_pages::page;

fn main() {
	// Mouse events
	let _mouse_events = page!(|| {
		div {
			@click: |_| { },
			@dblclick: |_| { },
			@mousedown: |_| { },
			@mouseup: |_| { },
			@mousemove: |_| { },
			@mouseover: |_| { },
			@mouseout: |_| { },
			@mouseenter: |_| { },
			@mouseleave: |_| { },
			"Mouse events"
		}
	});

	// Keyboard events
	let _keyboard_events = page!(|| {
		input {
			r#type: "text",
			@keydown: |_| { },
			@keyup: |_| { },
			@keypress: |_| { },
		}
	});

	// Form events
	let _form_events = page!(|| {
		form {
			@submit: |_| { },
			input {
				r#type: "text",
				@change: |_| { },
				@input: |_| { },
				@focus: |_| { },
				@blur: |_| { },
			}
			button {
				r#type: "reset",
				"Reset"
			}
		}
	});

	// Touch events
	let _touch_events = page!(|| {
		div {
			@touchstart: |_| { },
			@touchend: |_| { },
			@touchmove: |_| { },
			@touchcancel: |_| { },
			"Touch area"
		}
	});

	// Drag events
	let _drag_events = page!(|| {
		div {
			@drag: |_| { },
			@dragstart: |_| { },
			@dragend: |_| { },
			@dragover: |_| { },
			@drop: |_| { },
			@dragenter: |_| { },
			@dragleave: |_| { },
			"Drag and drop"
		}
	});

	// Scroll, load, and resize events
	let _misc_events = page!(|| {
		div {
			@scroll: |_| { },
			@load: |_| { },
			@error: |_| { },
			@resize: |_| { },
			"Content"
		}
	});

	// Multiple events on same element
	let _mixed = page!(|| {
		button {
			@click: |_| { },
			@mouseenter: |e| {
						let _ = e;
					},
			"Click me"
		}
	});
}
