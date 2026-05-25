//! UI compile-fail test: omitting a required prop in the brace form must
//! be rejected by `bon::Builder`'s `.build()` at compile time (spec §3.5.2
//! / DP #4 Fail early).
//!
//! Refs #4668 (P7) #4524.

use reinhardt_pages::component::Page;
use reinhardt_pages::page;

#[derive(bon::Builder)]
struct CardProps {
	// `item` is required because there is no `#[builder(default)]`.
	// This compile-fail fixture intentionally keeps `item` unused —
	// the field exists only so bon::Builder can detect its omission
	// in `Card { }` and reject `.build()` at compile time.
	#[allow(dead_code)]
	item: String,
}

fn card(p: CardProps) -> Page {
	let _ = p;
	page!(|| { div {} })()
}

fn main() {
	// Missing required `item` prop — bon must reject `.build()` at compile time.
	let _ = page!(|| {
		div {
			Card {}
		}
	});
}
