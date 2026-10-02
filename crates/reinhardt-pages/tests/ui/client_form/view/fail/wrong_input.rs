include!("../support.rs");
fn incompatible<Deps: Clone + PartialEq + 'static>(
	mutation: &reinhardt_pages::FormServerMutation<RequestClientForm, Deps, String, String>,
) {
	let _view = form! {
		client_form: RequestClientForm,
		mutation: mutation
	};
}
fn main() {
	let _ = use_form::<RequestClientForm>;
}
