use reinhardt_pages::{Page, component, page};

#[component("/projects/", name = "project", loader = || async { Ok::<_, String>(1_u64) })]
fn project() -> Page {
	page!(|| { p { "project" } })()
}

fn main() {}
