//! Compile-fail: `use_effect` requires a deps tuple as the second
//! positional argument (spec §4.2). Omitting it must be a hard compile
//! error so the missing-deps mistake is caught at build time, not at
//! runtime.

use reinhardt_pages::reactive::hooks::use_effect;

fn main() {
	let _ = use_effect(|| {});
}
