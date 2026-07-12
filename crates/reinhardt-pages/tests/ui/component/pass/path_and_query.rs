use reinhardt_pages::{Page, Path, Query, component, page};

#[component("/users/{id}/", "user-tab")]
fn user_tab(Path(id): Path<i64>, Query(tab): Query<String>) -> Page {
	page!(|id: i64, tab: String| {
		div { { format!("{id}:{tab}") } }
	})(id, tab)
}

fn main() {
	let _: UserTabProps = UserTabProps::builder()
		.id(7)
		.tab("profile".to_string())
		.build();
	let _: Page = page!(|| {
		UserTab {
			id: 7,
			tab: "profile".to_string(),
		}
	})();
}
