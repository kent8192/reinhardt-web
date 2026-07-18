use reinhardt_pages::page;
use reinhardt_pages::prelude::Signal;

fn main() {
	let selected = Signal::new(String::new());
	let dynamic_label = "Dynamic".to_owned();
	let _ = page!({
		select {
			aria_label: "Choice",
			bind: selected,
			option { { dynamic_label } }
		}
	});
}
