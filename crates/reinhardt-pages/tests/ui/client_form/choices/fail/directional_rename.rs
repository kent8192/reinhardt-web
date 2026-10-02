use reinhardt_pages::ClientFormChoices;

#[derive(Clone, Default, PartialEq, ClientFormChoices)]
enum ProviderMode {
	#[default]
	#[serde(rename(serialize = "wire_api", deserialize = "legacy_api"))]
	ApiMode,
}

fn main() {}
