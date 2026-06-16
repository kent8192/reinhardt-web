use reinhardt_pages::prelude::*;

#[client_page]
pub fn prelude_page() -> Page {
	page!(|| {
		div { "Prelude" }
	})()
}

fn main() {
	let _: Page = prelude_page();
}
