use reinhardt_pages::{Loader, Outlet, Page, layout, loader, page};

#[loader]
async fn shell_loader() -> Result<u64, String> {
	Ok(1)
}

#[layout("/projects/", name = "shell", loader = shell_loader)]
fn shell(Loader(data): Loader<String>, outlet: Outlet) -> Page {
	page!(|data: String, outlet: Outlet| {
		div {
			{ data }
			{ outlet }
		}
	})(data, outlet)
}

fn main() {}
