use reinhardt_pages::ClientForm;

#[derive(Clone, ClientForm)]
pub(crate) struct SettingsRequest {
	pub name: String,
	secret: String,
}

fn main() {}
