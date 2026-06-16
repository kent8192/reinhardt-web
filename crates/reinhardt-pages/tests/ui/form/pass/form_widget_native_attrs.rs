use reinhardt_pages::form;

fn main() {
	let _schedule_form = form! {
		name: ScheduleForm,
		action: "/schedule",
		fields: {
			billing_month: CharField {
				widget: MonthInput,
				required,
				min: "2026-01",
				max: "2026-12",
				step: 1,
			}
			sprint_week: CharField {
				widget: WeekInput,
				min: "2026-W01",
				max: "2026-W52",
				step: 1,
			}
			search: CharField {
				widget: SearchInput,
				size: 32,
			}
			avatar: FileField {
				accept: "image/png,image/jpeg",
				capture: "environment",
			}
		}
	};
}
