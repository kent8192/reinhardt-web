include!("../support.rs");
#[derive(Clone, reinhardt_pages::ClientForm)]
pub struct Other {
	pub text: String,
}
fn main() {
	let definition = RequestClientForm::new();
	let runtime = use_form(&definition).build();
	let mutation = definition.server_mutation(&runtime).build();
	let _view = form! {
		client_form: OtherClientForm,
		mutation: &mutation
	};
}
