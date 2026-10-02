use reinhardt_pages::{Outlet, Page, layout, page};

mod not_a_navigation_guard {
	// Match the generated marker name while testing the missing guard contract.
	#[allow(non_camel_case_types)]
	pub struct marker;
}

#[layout(
	"/dashboard/",
	name = "dashboard",
	navigation_guard = not_a_navigation_guard,
)]
fn dashboard(outlet: Outlet) -> Page {
	page!(|outlet: Outlet| {
		main { { outlet } }
	})(outlet)
}

fn main() {}
