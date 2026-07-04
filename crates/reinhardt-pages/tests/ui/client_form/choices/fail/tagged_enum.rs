use reinhardt_pages::ClientFormChoices;

#[derive(Clone, Default, PartialEq, ClientFormChoices)]
#[serde(tag = "kind")]
enum ProviderMode {
	#[default]
	Fake,
	LiveApi,
}

fn main() {}
