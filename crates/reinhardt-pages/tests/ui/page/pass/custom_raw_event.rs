//! Explicit custom intrinsic events retain the raw cross-target payload.

use reinhardt_pages::page;
use reinhardt_pages::platform::Event;

fn handle_raw(event: Event) {
	let _ = event.event_type();
}

fn main() {
	let _sync = page!(|| {
		div {
			@custom("item-selected"): |event: Event| {
				let _ = event.event_type();
			},
		}
	});
	let _async = page!(|| {
		div {
			@custom("item-loaded"): async |event: Event| {
				let _ = event.event_type();
			},
		}
	});
	let _external = page!(|| {
		div {
			@custom("item-removed"): crate::handle_raw,
		}
	});
	let _zero_argument = page!(|| {
		div {
			@custom("item-focused"): || {},
		}
	});
}
