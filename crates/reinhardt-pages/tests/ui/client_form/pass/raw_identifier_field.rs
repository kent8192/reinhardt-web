use reinhardt_pages::ClientForm;

#[derive(Clone, Default, PartialEq, ClientForm)]
pub struct RawFieldRequest {
	pub r#type: String,
}

fn main() {
	reinhardt_core::reactive::ReactiveScope::run(|| {
		let form = RawFieldRequestClientForm::new();
		let _field = form.type_field();
		let _variant = RawFieldRequestClientFormField::Type;
		assert_eq!(
			RawFieldRequestClientForm::field_from_name("type"),
			Some(RawFieldRequestClientFormField::Type)
		);
	});
}
