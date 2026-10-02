use reinhardt_pages::ClientFormChoices;

#[derive(Clone, Default, PartialEq, ClientFormChoices)]
enum ProviderMode {
	#[default]
	#[serde(skip)]
	Archived,
	Live,
}

fn main() {}
