//! Compile-pass (scope guarantee): a hook with a missing dep that is called
//! OUTSIDE a `page!` body is invisible to the `page!` macro and therefore not
//! checked by the deps validator — it must still compile (spec §4.5 scope
//! limitation, #4721/#4746). The runtime arity check still guarantees a deps
//! tuple is present.

use reinhardt_pages::reactive::Signal;
use reinhardt_pages::reactive::hooks::use_effect;

fn main() {
	let count = Signal::new(0_i32);
	// `count` is read but not listed in the deps tuple. Because this call is
	// not inside a `page!` body, the static validator does not see it.
	let _e = use_effect(
		{
			let count = count.clone();
			move || {
				let _ = count.get();
				None::<fn()>
			}
		},
		(),
	);
	let _ = count;
}
