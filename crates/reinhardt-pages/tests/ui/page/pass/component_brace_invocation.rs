//! UI compile-pass test for the React-style brace-form component invocation
//! (spec §3.5).
//!
//! Refs #4668 (P7) #4524.

use reinhardt_pages::component::Page;
use reinhardt_pages::page;

#[derive(bon::Builder)]
struct CardProps {
	item: String,
}

fn card(p: CardProps) -> Page {
	page!(|p: CardProps| {
		article {
			h2 {
				{ p.item.clone() }
			}
		}
	})(p)
}

fn main() {
	let _ = page!(|| {
		div {
			Card {
				item: "x".to_string(),
			}
		}
	});
}
