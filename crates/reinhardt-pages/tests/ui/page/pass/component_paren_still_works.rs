//! Regression UI compile-pass test ensuring the legacy positional
//! `{component_fn(args)}` form continues to compile unchanged after the
//! brace-form addition (spec §3.5 — additive change, §7).
//!
//! Refs #4668 (P7) #4524.

use reinhardt_pages::component::Page;
use reinhardt_pages::page;

fn my_button(label: String, disabled: bool) -> Page {
	let _ = disabled;
	page!(|label: String| { button { {label.clone()} } })(label)
}

fn main() {
	let _ = page!(|| {
		div { {my_button("click".to_string(), false)} }
	});
}
