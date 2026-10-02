//! Compile-fail: the legacy tuple dependency argument was removed. Use
//! `deps![signal]` for an explicit dependency list.

use reinhardt_pages::reactive::Signal;
use reinhardt_pages::reactive::hooks::use_effect;

fn main() {
	let count = Signal::new(0_i32);
	let _effect = use_effect(move || {}, (count.clone(),));
}
