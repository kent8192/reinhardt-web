use reinhardt_pages::ClientFormChoices;

#[derive(Clone, PartialEq, ClientFormChoices)]
enum ProviderMode {
	Fake,
	Live,
}

fn main() {}
