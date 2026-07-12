use reinhardt_pages::form;

fn main() {
	let _form = form! {
		name: ControlEntryForm,
		action: "/import",
		fields: {
			file: FileField {
				accept: "text/csv",
			}
			upload: SubmitButton {
				label: "Upload",
			}
			reset: ResetButton {
				label: "Reset",
				class: "secondary",
			}
			preview: Button {
				label: "Preview",
				id: "preview-button",
				disabled,
			}
			image_submit: ImageInput {
				src: "/static/submit.png",
				alt: "Submit",
				width: 48,
				height: 48,
			}
			import_progress: Progress {
				value: 7,
				max: 10,
				label: "Import progress",
			}
			capacity: Meter {
				value: 0.72,
				min: 0.0,
				max: 1.0,
				low: 0.4,
				high: 0.8,
				optimum: 0.6,
				label: "Capacity usage",
			}
			summary: Output {
				label: "Import summary",
				for: [file],
			}
		}
	};
}
