#![allow(unused_imports)]

use reinhardt_pages::{Page, component, page};

#[component("/users/{id}/", "user-detail")]
fn user_page(id: i64) -> Page {
	page!(|| {
		div { {
			id.to_string()
		} }
	})()
}

fn main() {}
