//! `DateField` bind listener generates code compatible with `Signal<Option<NaiveDate>>`.

use reinhardt_pages::form;

fn main() {
	let _ = form! {
		name: EventForm,
		action: "/api/event",

		fields: {
			start_date: DateField,
		}

	};
}
