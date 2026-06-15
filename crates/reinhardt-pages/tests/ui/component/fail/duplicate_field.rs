#![allow(unused_imports)] // Compile-fail fixtures may stop before using every imported helper.

use reinhardt_pages::{Page, Path, component, page};

#[component("/users/{id}/posts/{post_id}/", "user-post")]
fn user_post(Path(id): Path<i64>, Path(id): Path<i64>) -> Page {
	page!(|| {
		div { {
			id.to_string()
		} }
	})()
}

fn main() {}
