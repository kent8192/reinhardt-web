use reinhardt_pages::ClientFormChoices;

#[derive(Clone, Default, PartialEq, ClientFormChoices)]
enum ProviderMode {
	#[default]
	Fake,
	#[serde(deserialize_with = "deserialize_live")]
	Live,
}

fn main() {}
