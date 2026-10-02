use reinhardt_pages::ClientFormChoices;

pub enum WireMode {
	Live,
}

impl From<Mode> for WireMode {
	fn from(_: Mode) -> Self {
		Self::Live
	}
}

#[derive(Clone, Default, PartialEq, ClientFormChoices)]
#[serde(into = "WireMode")]
pub enum Mode {
	#[default]
	Live,
}

fn main() {}
