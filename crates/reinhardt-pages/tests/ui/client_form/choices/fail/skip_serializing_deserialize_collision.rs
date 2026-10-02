use reinhardt_pages::ClientFormChoices;

#[derive(Clone, Default, PartialEq, ClientFormChoices)]
enum ProviderMode {
	#[default]
	Fake,
	#[serde(skip_serializing, rename(deserialize = "live"))]
	Archived,
	#[serde(rename = "live")]
	Live,
}

fn main() {}
