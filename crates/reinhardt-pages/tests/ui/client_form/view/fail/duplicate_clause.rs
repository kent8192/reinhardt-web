include!("../support.rs");
fn main() {
	let definition = RequestClientForm::new();
	let runtime = use_form(&definition).build();
	let mutation = definition.server_mutation(&runtime).build();
	let _view = form! {
		client_form: RequestClientForm,
		mutation: &mutation,
		id: "a",
		id: "b"
	};
}
