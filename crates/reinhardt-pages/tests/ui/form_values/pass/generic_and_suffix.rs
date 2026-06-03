use reinhardt_pages::{FormFields, FormValues, Signal};

#[derive(Clone, PartialEq, FormValues)]
struct GenericForm<T>
where
	T: Clone + PartialEq + 'static,
{
	pub value: T,
}

fn assert_form_values_trait<T>()
where
	T: Clone + PartialEq + 'static,
	GenericForm<T>: FormValues<Fields = GenericFormFields<T>>,
	GenericFormFields<T>: FormFields<Values = GenericForm<T>>,
{
}

fn assert_generated_fields<T>(fields: GenericFormFields<T>)
where
	T: Clone + PartialEq + 'static,
{
	let _: Signal<T> = fields.value;
}

fn main() {
	assert_form_values_trait::<String>();
	let _ = assert_generated_fields::<String>;
}
