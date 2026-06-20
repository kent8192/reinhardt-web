use reinhardt_pages::{Page, Path, component, page};

#[component("/users/{id}/", user_detail)]
fn user_page(Path(id): Path<i64>) -> Page {
	page!(|id: i64| {
		div { { id.to_string() } }
	})(id)
}

fn main() {
	let _ = UserPageProps::builder().id(7).build();
}
