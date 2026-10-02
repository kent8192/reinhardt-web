use reinhardt_pages::ClientFormChoices;

#[derive(Clone, Default, PartialEq, ClientFormChoices)]
enum ProviderMode {
	#[default]
	Fake,
	Live { endpoint: String },
}

fn main() {}
