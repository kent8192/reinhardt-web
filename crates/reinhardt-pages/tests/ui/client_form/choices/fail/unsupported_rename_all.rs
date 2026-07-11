use reinhardt_pages::ClientFormChoices;

#[derive(Clone, Default, PartialEq, ClientFormChoices)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum ProviderMode {
	#[default]
	Fake,
	Live,
}

fn main() {}
