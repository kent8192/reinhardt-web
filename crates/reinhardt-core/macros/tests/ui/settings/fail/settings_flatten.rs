use reinhardt_macros::settings;

#[settings(fragment = true)]
struct BadSettings {
	#[serde(flatten)]
	pub nested: NestedSettings,
}

struct NestedSettings {
	pub value: String,
}

fn main() {}
