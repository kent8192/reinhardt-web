use reinhardt_pages::{Page, component, page};

#[component("/projects/", name = "project", cache = true)]
fn project() -> Page {
	page!(|| {
		p { "project" }
	})()
}

fn main() {}
