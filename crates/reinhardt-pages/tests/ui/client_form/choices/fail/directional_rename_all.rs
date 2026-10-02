use reinhardt_pages::ClientFormChoices;

#[derive(Clone, Default, PartialEq, ClientFormChoices)]
#[serde(rename_all(serialize = "kebab-case", deserialize = "snake_case"))]
enum ProviderMode {
	#[default]
	Fake,
	LiveApi,
}

fn main() {}
