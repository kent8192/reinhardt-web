//! `DateField` bind listener generates code compatible with `Signal<Option<NaiveDate>>`.

use reinhardt_pages::form;

fn main() {
	reinhardt_core::reactive::ReactiveScope::run(|| {
		let _ = form! {
			name: EventForm,
			action: "/api/event",
			fields: {
				start_date: DateField,
			}
		};
	});
}
