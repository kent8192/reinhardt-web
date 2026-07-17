//! Compile-pass: explicit cleanup-type turbofish calls retain the
//! `use_effect::<_, C>` and `use_layout_effect::<_, C>` shape with mount-only deps.

use reinhardt_pages::{
	deps,
	reactive::hooks::{use_effect, use_layout_effect},
};

fn main() {
	reinhardt_core::reactive::ReactiveScope::run(|| {
		let _effect = use_effect::<_, fn()>(|| None, deps![]);
		let _layout_effect = use_layout_effect::<_, fn()>(|| None, deps![]);
	});
}
