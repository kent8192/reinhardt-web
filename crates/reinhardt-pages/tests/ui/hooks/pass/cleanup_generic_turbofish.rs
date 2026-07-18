//! Compile-pass: explicit cleanup-type turbofish calls specify the two public
//! generic parameters and an explicit empty dependency list.

use reinhardt_pages::deps;
use reinhardt_pages::reactive::hooks::{use_effect, use_layout_effect};

fn main() {
	reinhardt_core::reactive::ReactiveScope::run(|| {
		let _effect = use_effect::<_, fn()>(|| None, deps![]);
		let _layout_effect = use_layout_effect::<_, fn()>(|| None, deps![]);
	});
}
