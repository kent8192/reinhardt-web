// Rejected macro items leave their supporting imports unused.
#![allow(unused_imports)]

use reinhardt_pages::{Page, component, page};

#[component("/projects/", name = "project", cache = true)]
fn project() -> Page {
	page!(|| {
		p { "project" }
	})()
}

fn main() {}
