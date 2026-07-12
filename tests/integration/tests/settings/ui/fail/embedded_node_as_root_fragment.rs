use reinhardt_macros::settings;

#[settings(fragment = true)]
struct EmbeddedConfig {
	value: String,
}

#[settings(embedded: EmbeddedConfig)]
struct InvalidProjectSettings;

fn main() {
	let _settings = InvalidProjectSettings {
		embedded: EmbeddedConfig {
			value: String::new(),
		},
	};
}
