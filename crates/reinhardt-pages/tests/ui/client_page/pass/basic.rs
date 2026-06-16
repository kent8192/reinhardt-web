use reinhardt_pages::{Page, client_page};

#[client_page]
pub fn home_page() -> Page {
	reinhardt_pages::page!(|| {
		div { "Home" }
	})()
}

#[client_page]
pub fn detail_page(id: i64) -> Page {
	reinhardt_pages::page!(|id: i64| {
		div { { id.to_string() } }
	})(id)
}

fn main() {
	let _: Page = home_page();
	let _: Page = detail_page(7);
}
