use reinhardt_pages::{FormValues, Signal};

#[derive(Clone, PartialEq, FormValues)]
struct VoteFormValues {
	question_id: i64,
	choice_id: i64,
}

fn assert_form_values_trait<T: reinhardt_pages::FormValues>() {}

fn assert_generated_fields(fields: VoteFormFields) {
	let _: Signal<i64> = fields.question_id;
	let _: Signal<i64> = fields.choice_id;
}

fn main() {
	assert_form_values_trait::<VoteFormValues>();
	let _ = VoteFormValues::field_names();
	let _ = assert_generated_fields;
}
