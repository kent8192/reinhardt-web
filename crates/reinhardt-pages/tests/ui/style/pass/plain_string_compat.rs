use reinhardt_pages::page;

fn main() {
	let _ = page!({
		div {
			class: "legacy",
			style: "color: red;",
			"Legacy"
		}
	});
}
