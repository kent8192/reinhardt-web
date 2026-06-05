use reinhardt_macros::settings;

#[settings(fragment = true, section = "bad")]
struct BadSettings {
	#[serde(flatten)]
	pub nested: NestedSettings,
}

struct NestedSettings {
	pub value: String,
}

fn main() {}
