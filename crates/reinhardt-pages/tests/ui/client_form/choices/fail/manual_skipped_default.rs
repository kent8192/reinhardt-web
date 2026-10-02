use reinhardt_pages::ClientFormChoices;

#[derive(Clone, PartialEq, ClientFormChoices)]
enum ProviderMode {
	#[serde(skip)]
	Archived,
	Live,
}

impl Default for ProviderMode {
	fn default() -> Self {
		Self::Archived
	}
}

fn main() {}
