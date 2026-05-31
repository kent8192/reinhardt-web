//! Compile-pass (regression guard): a dependency listed in the deps tuple but
//! never read inside the closure is NOT a compile error (#4721 mirror case).
//! Stable proc-macros have no warning channel and an unused dep is harmless,
//! so the validator deliberately stays silent here. The hook is called via a
//! qualified path so it is exempt from `page!` capture discipline.

use reinhardt_pages::page;
use reinhardt_pages::reactive::Signal;
use reinhardt_pages::reactive::hooks;

fn main() {
	let _ = page!(|count: Signal<i32>| {
		div { {
			hooks::use_effect(move || {
				// `count` is in the deps tuple but never read here.
				None::<fn() >
			}, (count.clone(), ), );
			"x"
		} }
	});
}
