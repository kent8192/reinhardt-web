use reinhardt_pages::{Page, component, page};

#[component("/projects/", name = "project", loader = first, loader = second)]
fn project() -> Page {
	page!(|| { p { "project" } })()
}

fn main() {}
