//! Welcome Page Component for Reinhardt Development Server
//!
//! This module provides the WelcomePage component that is displayed when
//! the development server starts without any configured routes.

use reinhardt_pages::component::{Component, Head, IntoPage, Page, PageElement};

/// CSS styles for the welcome page.
///
/// Defines the visual styling including gradient background, container styling,
/// typography, badges, and feature cards.
const WELCOME_PAGE_STYLES: &str = r#"
body {
    font-family: "Segoe UI", Tahoma, Geneva, Verdana, sans-serif;
    max-width: 800px;
    margin: 50px auto;
    padding: 20px;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    min-height: 100vh;
}
.container {
    background: white;
    border-radius: 10px;
    padding: 40px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.3);
}
h1 {
    color: #667eea;
    margin-bottom: 10px;
}
p {
    color: #666;
    line-height: 1.6;
}
.badge {
    background: #667eea;
    color: white;
    padding: 5px 15px;
    border-radius: 20px;
    font-size: 14px;
    display: inline-block;
    margin-bottom: 20px;
}
.features {
    margin-top: 30px;
}
.feature {
    padding: 15px;
    margin: 10px 0;
    background: #f7f7f7;
    border-left: 4px solid #667eea;
    border-radius: 5px;
}
.feature h3 {
    margin: 0 0 5px 0;
    color: #667eea;
}
"#;

/// Welcome page component displayed by the development server.
///
/// This component renders a styled welcome page with the Reinhardt version
/// and feature highlights.
pub struct WelcomePage {
	/// The version string to display in the badge.
	pub version: String,
}

impl WelcomePage {
	/// Creates a new WelcomePage with the specified version.
	pub fn new(version: impl Into<String>) -> Self {
		Self {
			version: version.into(),
		}
	}

	/// Creates a feature card element.
	fn feature_card(emoji: &str, title: &str, description: &str) -> PageElement {
		PageElement::new("div")
			.attr("class", "feature")
			.child(PageElement::new("h3").child(format!("{} {}", emoji, title)))
			.child(PageElement::new("p").child(description.to_string()))
	}
}

impl Component for WelcomePage {
	fn render(&self) -> Page {
		// Build the head section
		let head = Head::new()
			.meta_charset("UTF-8")
			.meta_viewport("width=device-width, initial-scale=1.0")
			.title("Reinhardt Framework")
			.inline_css(WELCOME_PAGE_STYLES);

		// Build the body content
		let body_content = PageElement::new("div")
			.attr("class", "container")
			// Version badge
			.child(
				PageElement::new("div")
					.attr("class", "badge")
					.child(format!("v{}", self.version)),
			)
			// Title
			.child(PageElement::new("h1").child("Welcome to Reinhardt!"))
			// Description
			.child(PageElement::new("p").child(
				"Your Django/FastAPI-inspired web framework for Rust is running successfully.",
			))
			// Features section
			.child(
				PageElement::new("div")
					.attr("class", "features")
					.child(Self::feature_card(
						"\u{1F680}",
						"High Performance",
						"Built on Tokio and Hyper for blazing-fast async performance",
					))
					.child(Self::feature_card(
						"\u{1F512}",
						"Type Safe",
						"Leverage Rust's type system for compile-time correctness",
					))
					.child(Self::feature_card(
						"\u{1F3AF}",
						"Familiar API",
						"Django and FastAPI-inspired patterns you already know",
					))
					.child(Self::feature_card(
						"\u{1F527}",
						"Modular",
						"57 crates working together - use what you need",
					)),
			);

		// Return container directly (SSR renderer adds body)
		body_content.into_page().with_head(head)
	}

	fn name() -> &'static str {
		"WelcomePage"
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_welcome_page_renders() {
		let page = WelcomePage::new("0.1.0");
		let rendered = page.render();
		let html = rendered.render_to_string();

		// Verify key elements are present
		assert!(html.contains("Welcome to Reinhardt!"));
		assert!(html.contains("v0.1.0"));
		assert!(html.contains("High Performance"));
		assert!(html.contains("Type Safe"));
		assert!(html.contains("Familiar API"));
		assert!(html.contains("Modular"));
	}

	#[rstest]
	fn test_welcome_page_has_head() {
		let page = WelcomePage::new("0.1.0");
		let rendered = page.render();

		// Verify head is attached
		let head = rendered.find_topmost_head();
		assert!(head.is_some());

		let head = head.unwrap();
		assert_eq!(
			head.title,
			Some(std::borrow::Cow::Borrowed("Reinhardt Framework"))
		);
	}

	#[rstest]
	fn test_component_name() {
		assert_eq!(WelcomePage::name(), "WelcomePage");
	}
}
