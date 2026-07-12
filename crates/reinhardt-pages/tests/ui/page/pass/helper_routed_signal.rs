//! Spec §4.1 consequence: helper-routed Signal reads now "just work"
//! because the wrap is unconditional — no static-detection limitation.
//! Resolves #4515 at the root cause.

use reinhardt_pages::page;
use reinhardt_pages::reactive::Signal;

fn show(s: &Signal<i32>) -> i32 {
	s.get()
}

fn main() {
	let _ = page!(|count: Signal<i32>, show: fn(&Signal<i32>) -> i32| {
		p { { show(&count).to_string() } }
	});
}
