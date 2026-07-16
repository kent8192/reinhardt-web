use reinhardt_pages::{Loader, Page, component, page};

#[component("/projects/", name = "project")]
fn project(Loader(data): Loader<String>) -> Page {
	page!(|data: String| {
		p { { data } }
	})(data)
}

fn main() {}
