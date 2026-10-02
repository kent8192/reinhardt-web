use reinhardt_pages::page;

fn main() {
	let _ = page!({
		div {
			a11y: off,
			bind: (),
		}
	});
}
