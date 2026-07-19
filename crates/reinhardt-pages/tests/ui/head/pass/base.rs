use reinhardt_pages::{Head, Page, head, page};

fn main() {
	let view: Page = page!(#head: head!(|| {
		base { href: "/app/" }
		title { "Embedded" }
	}), { main { "body" } });
	let _ = (view, Head::new());
}
