use reinhardt_pages::page;
use reinhardt_pages::prelude::Signal;

fn main() {
	let selected = Signal::new(String::new());
	let dynamic_tabindex = 0;
	let _ = page!({
		select {
			aria_label: "Choice",
			bind: selected,
			option {
				value: "one",
				span {
					strong {
						tabindex: dynamic_tabindex,
						"One"
					}
				}
			}
		}
	});
}
