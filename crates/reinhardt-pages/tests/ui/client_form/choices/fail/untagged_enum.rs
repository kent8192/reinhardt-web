use reinhardt_pages::ClientFormChoices;

#[derive(Clone, Default, PartialEq, ClientFormChoices)]
#[serde(untagged)]
enum ProviderMode {
	#[default]
	Fake,
	LiveApi,
}

fn main() {}
