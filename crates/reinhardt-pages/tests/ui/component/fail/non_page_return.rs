#![allow(unused_imports)] // Compile-fail fixtures may stop before using every imported helper.

use reinhardt_pages::{Path, component};

#[component("/users/{id}/", name = "user-name")]
fn user_name(Path(id): Path<i64>) -> String {
	id.to_string()
}

fn main() {}
