//! Standard event handlers reject a payload for a different event name.

// reinhardt-fmt: ignore-all

use reinhardt_pages::event::InputEvent;
use reinhardt_pages::page;

fn main() {
	let _invalid = page!(|| {
		button { @click: |_event: InputEvent| {}, "Click" }
	});
}
