//! Compile-fail: every expression in `deps![...]` must implement `Trackable`.

use reinhardt_pages::deps;
use reinhardt_pages::reactive::hooks::use_effect;

fn main() {
	let _effect = use_effect(
		move || {},
		deps![42_i32],
	);
}
