//! Compile-fail: callbacks require an explicit `deps![...]` list; automatic
//! dependency tracking is intentionally unavailable for callback hooks.

use reinhardt_pages::reactive::Signal;
use reinhardt_pages::reactive::hooks::use_callback;
use reinhardt_pages::deps_auto;

fn main() {
	let count = Signal::new(0_i32);
	let _callback = use_callback(
		move |_: ()| {
			let _ = count.get();
		},
		deps_auto!(),
	);
}
