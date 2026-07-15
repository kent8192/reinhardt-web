//! Compile-fail: retained effects are registration-style hooks and require
//! an explicit `deps![...]` list for their mounted lifetime.

use reinhardt_pages::deps_auto;
use reinhardt_pages::reactive::Signal;
use reinhardt_pages::reactive::hooks::use_retained_effect;

fn main() {
	let count = Signal::new(0_i32);
	use_retained_effect(
		move || {
			let _ = count.get();
		},
		deps_auto!(),
	);
}
