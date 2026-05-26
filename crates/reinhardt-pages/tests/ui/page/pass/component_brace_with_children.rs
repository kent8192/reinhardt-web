//! UI compile-pass test for the children arity rules in spec §3.5.3
//! (this case exercises the ≥2-children branch, which lowers to
//! `Page::fragment(vec![...])`).
//!
//! Refs #4668 (P7) #4524.

use reinhardt_pages::component::Page;
use reinhardt_pages::page;

#[derive(bon::Builder)]
struct CardProps {
	item: String,
	// `Option<_>` is implicitly optional under `bon::Builder` — no
	// `#[builder(default)]` needed (bon rejects it as redundant).
	children: Option<Page>,
}

fn card(p: CardProps) -> Page {
	page!(|p: CardProps| {
		article {
			h2 { {
				p.item.clone()
			} }
			{
				p.children.unwrap_or_else(Page::empty)
			}
		}
	})(p)
}

fn main() {
	let _ = page!(|| {
		div {
			Card(item: "outer".to_string()) {
				p { "child 1" }
				p { "child 2" }
			}
		}
	});
}
