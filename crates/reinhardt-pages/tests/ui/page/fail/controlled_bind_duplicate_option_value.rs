use reinhardt_pages::page;
use reinhardt_pages::reactive::Signal;

fn main() {
	let selected = Signal::new(String::new());
	let condition = true;
	let _ = page!({
		select {
			a11y: off,
			bind: selected,
			if condition {
				optgroup {
					option {
						value: "first",
						value: "second",
						"Choice"
					}
				}
			}
		}
	});
}
