//! Compile-fail: the legacy unit dependency argument was removed. Use
//! `deps![]` for a mount-only explicit dependency list.

use reinhardt_pages::reactive::hooks::use_effect;

fn main() {
	let _effect = use_effect(move || {}, ());
}
