use reinhardt_pages::form;

fn main() {
	let _form = form! {
		name: InvalidWeekIpAddressForm,
		action: "/invalid",
		fields: {
			sprint_week: IpAddressField {
				widget: WeekInput,
			}
		}
	};
}
