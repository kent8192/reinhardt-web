use reinhardt_pages::{Page, Path, Query, component, page};

#[component("/users/{id}/", "user-detail")]
fn user_page(Path(id): Path<i64>, Query(tab): Query<String>) -> Page {
	page!(|id: i64, tab: String| {
		div { {
			format!("{id}:{tab}")
		} }
	})(id, tab)
}

fn main() {
	let _ = page!(|| {
		UserPage {
			id: 7,
			tab: "profile".to_string(),
		}
	})();
}
