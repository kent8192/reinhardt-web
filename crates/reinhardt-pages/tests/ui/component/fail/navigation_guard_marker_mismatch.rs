use reinhardt_pages::{Page, component, page};

mod not_a_navigation_guard {
	// Match the generated marker name while testing the missing guard contract.
	#[allow(non_camel_case_types)]
	pub struct marker;
}

#[component(
	"/account/",
	name = "account",
	navigation_guard = not_a_navigation_guard,
)]
fn account() -> Page {
	page!(|| {
		p { "account" }
	})()
}

fn main() {}
