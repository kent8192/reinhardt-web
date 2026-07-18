use reinhardt_pages::page;
use reinhardt_pages::reactive::Signal;

fn main() {
	let selected = Signal::new(String::new());
	let _ = page!({
		input {
			a11y: off,
			type: "radio",
			value: "first",
			value: "second",
			bind: selected
		}
	});
}
