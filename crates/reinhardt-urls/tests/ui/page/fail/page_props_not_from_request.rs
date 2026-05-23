//! Compile-fail: a Props struct without `FromRequest` cannot be used
//! as a `ClientRouter::page` handler argument. Refs #4668 / P7 part 2.

use reinhardt_core::types::page::Page;
use reinhardt_urls::routers::ClientRouter;

struct WhateverProps;

fn handler(_p: WhateverProps) -> Page {
	Page::Empty
}

fn main() {
	let _ = ClientRouter::new().page("/x/", handler);
}
