//! UI compile-pass test exercising the `@event: handler` brace prop form
//! (spec §3.5). The bon Builder synthesises the `.on_click(...)` setter
//! from the `on_click: Option<...>` field, which the codegen calls.
//!
//! Refs #4668 (P7) #4524.

use reinhardt_pages::callback::Callback;
use reinhardt_pages::component::{DummyEvent, Page};
use reinhardt_pages::page;

#[derive(bon::Builder)]
struct ButtonProps {
	label: String,
	// `Option<_>` is implicitly optional under `bon::Builder` — no
	// `#[builder(default)]` needed (bon rejects it as redundant).
	on_click: Option<Callback<DummyEvent, ()>>,
}

fn button(p: ButtonProps) -> Page {
	page!(|p: ButtonProps| {
		button {
			{ p.label.clone() }
		}
	})(p)
}

fn main() {
	let _ = page!(|| {
		div {
			Button {
				label: "click me".to_string(),
				@click: Callback::new(|_: DummyEvent| {}),
			}
		}
	});
}
