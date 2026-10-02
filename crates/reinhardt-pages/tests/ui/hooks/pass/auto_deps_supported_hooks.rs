//! Compile-pass: automatic dependency tracking is supported by effects,
//! layout effects, and memos. The `page!` validator must leave `deps_auto!()`
//! unchecked because the runtime discovers reads while each closure runs.

use reinhardt_pages::reactive::Signal;
use reinhardt_pages::reactive::hooks;
use reinhardt_pages::{deps_auto, page};

fn main() {
	let _ = page!(|count: Signal<i32>| {
		div { {
			let _effect = hooks::use_effect({
				let count = count.clone();
				move || {
					let _ = count.get();
				}
			}, deps_auto!());
			let _layout = hooks::use_layout_effect({
				let count = count.clone();
				move || {
					let _ = count.get();
				}
			}, deps_auto!());
			let _memo = hooks::use_memo({
				let count = count.clone();
				move || count.get()
			}, deps_auto!());
			"automatic"
		} }
	});
}
