#![deny(deprecated)]

use reinhardt_pages::event::{EventPayload, KeyPressEvent};

fn assert_payload<P: EventPayload>() {}

fn main() {
	assert_payload::<KeyPressEvent>();
}
