use reinhardt_pages::form;

fn main() {
	let _form = form! {
		name: InvalidMonthUuidForm,
		action: "/invalid",
		fields: {
			billing_month: UuidField {
				widget: MonthInput,
			}
		}
	};
}
