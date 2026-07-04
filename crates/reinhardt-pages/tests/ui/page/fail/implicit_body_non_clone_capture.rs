use reinhardt_pages::page;

struct NotClone(String);

impl NotClone {
	fn label(&self) -> &str {
		&self.0
	}
}

fn main() {
	let handle = NotClone("owned".to_string());
	let _ = page!({
		div { { handle.label() } }
	});
}
