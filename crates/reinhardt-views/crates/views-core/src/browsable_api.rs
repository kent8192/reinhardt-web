//! Browsable API rendering
//!
//! This module provides HTML rendering capabilities for REST API responses,
//! allowing interactive exploration of APIs through a web browser.

pub mod forms;
pub mod highlighter;
pub mod renderer;
pub mod templates;

pub use forms::FormGenerator;
pub use highlighter::SyntaxHighlighter;
pub use renderer::BrowsableApiRenderer;
pub use templates::InteractiveDocsRenderer;

/// Color scheme for syntax highlighting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorScheme {
	/// Dark theme (default)
	Dark,
	/// Light theme
	Light,
	/// Monokai theme
	Monokai,
	/// Solarized dark theme
	SolarizedDark,
	/// Solarized light theme
	SolarizedLight,
}

impl Default for ColorScheme {
	fn default() -> Self {
		Self::Dark
	}
}

impl ColorScheme {
	/// Get the theme name for syntect
	pub fn theme_name(&self) -> &'static str {
		match self {
			Self::Dark => "base16-ocean.dark",
			Self::Light => "InspiredGitHub",
			Self::Monokai => "Monokai Extended",
			Self::SolarizedDark => "Solarized (dark)",
			Self::SolarizedLight => "Solarized (light)",
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_color_scheme_default() {
		assert_eq!(ColorScheme::default(), ColorScheme::Dark);
	}

	#[test]
	fn test_color_scheme_theme_names() {
		assert_eq!(ColorScheme::Dark.theme_name(), "base16-ocean.dark");
		assert_eq!(ColorScheme::Light.theme_name(), "InspiredGitHub");
		assert_eq!(ColorScheme::Monokai.theme_name(), "Monokai Extended");
		assert_eq!(ColorScheme::SolarizedDark.theme_name(), "Solarized (dark)");
		assert_eq!(
			ColorScheme::SolarizedLight.theme_name(),
			"Solarized (light)"
		);
	}
}
