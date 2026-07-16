use reinhardt_pages::{Outlet, Page, layout, loader, page};

#[loader]
async fn shell_loader() -> Result<String, String> {
	Ok(String::new())
}

#[layout("/projects/", name = "shell", loader = shell_loader)]
fn shell(outlet: Outlet) -> Page {
	page!(|outlet: Outlet| { div { { outlet } } })(outlet)
}

fn main() {}
