use reinhardt_pages::ClientFormChoices;

#[derive(Clone, Default, PartialEq, ClientFormChoices)]
enum ProviderMode {
	#[default]
	#[serde(rename = "same")]
	Fake,
	#[serde(rename = "same")]
	Live,
}

fn main() {}
