//! Compile-pass: a `page!`-embedded hook closure that reads no Signals with an
//! empty `deps![]` dependency list is the mount-only shape and must not be flagged
//! (spec §4.5, #4721/#4746). The hook is called via a qualified path so it is
//! exempt from `page!` capture discipline.

use reinhardt_pages::reactive::hooks;
use reinhardt_pages::{deps, page};

fn main() {
	let _ = page!(|| {
		div { {
			hooks::use_effect(move || {
				// one-time mount work, no Signal reads
				None::<fn() >
			}, deps![], );
			"x"
		} }
	});
}
