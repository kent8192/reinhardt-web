#![allow(unused_imports)] // Compile-fail fixtures may stop before using every imported helper.

use reinhardt_pages::{Page, Path, component, page};

#[component("/users/{id}/")]
fn user_page(Path(id): Path<i64>) -> Page {
	page!(|| {
		div { {
			id.to_string()
		} }
	})()
}

fn main() {}
