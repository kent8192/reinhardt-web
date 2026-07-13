use reinhardt_pages::page;

fn main() {
	let _ = page!({
		input {
			a11y: off,
			type: String::from("text"),
			bind: ()
		}
	});
}
