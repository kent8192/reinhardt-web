use reinhardt_pages::ClientFormChoices;

#[derive(Clone, Default, PartialEq, ClientFormChoices)]
enum ProviderMode {
	#[default]
	#[serde(alias = "live")]
	Fake,
	#[serde(rename = "live")]
	Live,
}

fn main() {}
