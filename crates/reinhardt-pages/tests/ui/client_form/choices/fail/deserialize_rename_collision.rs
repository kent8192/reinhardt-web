use reinhardt_pages::ClientFormChoices;

#[derive(Clone, Default, PartialEq, ClientFormChoices)]
enum ProviderMode {
	#[default]
	#[serde(rename(serialize = "fake", deserialize = "live"))]
	Fake,
	#[serde(rename = "live")]
	Live,
}

fn main() {}
