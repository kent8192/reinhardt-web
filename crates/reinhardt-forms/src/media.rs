/// Media assets for form widgets
#[derive(Debug, Clone, Default)]
pub struct Media {
	#[allow(dead_code)]
	css: Vec<String>,
	#[allow(dead_code)]
	js: Vec<String>,
}

impl Media {
	pub fn new() -> Self {
		Self::default()
	}
}

/// Trait for widgets that define their own media
pub trait MediaDefiningWidget {
	fn media(&self) -> Media;
}
