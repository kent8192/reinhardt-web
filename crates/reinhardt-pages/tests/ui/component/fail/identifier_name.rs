#![allow(unused_imports)] // Compile-fail fixtures may stop before using every imported helper.

use reinhardt_pages::{Page, Path, component, page};

#[component("/users/{id}/", user_detail)]
fn user_page(Path(id): Path<i64>) -> Page {
	page!(|id: i64| {
		div { { id.to_string() } }
	})(id)
}

fn main() {}
