//! Compile-pass: `()` is the React-parity "mount-only" deps shape — the
//! effect runs once on mount and never re-runs (spec §4.2).

use reinhardt_pages::reactive::hooks::use_effect;

fn main() {
	let _e = use_effect(
		|| {
			// one-time mount work
		},
		(),
	);
}
