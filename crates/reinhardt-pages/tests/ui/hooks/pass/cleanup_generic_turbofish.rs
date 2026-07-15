//! Compile-pass: explicit cleanup-type turbofish calls keep the pre-existing
//! `use_effect::<_, C>` and `use_layout_effect::<_, C>` shape.

use reinhardt_pages::deps;
use reinhardt_pages::reactive::hooks::{use_effect, use_layout_effect};

fn main() {
	let _effect = use_effect::<_, fn()>(|| None, deps![]);
	let _layout_effect = use_layout_effect::<_, fn()>(|| None, deps![]);
}
