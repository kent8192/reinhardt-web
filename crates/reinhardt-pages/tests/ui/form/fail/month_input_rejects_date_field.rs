use reinhardt_pages::form;

fn main() {
	let _form = form! {
		name: InvalidMonthForm,
		action: "/invalid",
		fields: {
			billing_month: DateField {
				widget: MonthInput,
			}
		}
	};
}
