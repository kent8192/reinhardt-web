//! `IntegerField` bind listener generates code compatible with `Signal<i64>`.

use reinhardt_pages::form;

fn main() {
	let _ = form! {
		name: CounterForm,
		action: "/api/counter",

		fields: {
			count: IntegerField {
				initial: 0i64,
			}
		}

	};
}
