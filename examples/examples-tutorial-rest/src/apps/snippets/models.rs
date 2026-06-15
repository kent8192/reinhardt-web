use chrono::{DateTime, Utc};
use reinhardt::prelude::*;
use serde::{Deserialize, Serialize};

/// Snippet model representing a code snippet
#[model(app_label = "snippets", table_name = "snippets")]
#[derive(Serialize, Deserialize)]
pub struct Snippet {
	#[field(primary_key = true)]
	pub id: i64,

	#[field(max_length = 100)]
	pub title: String,

	#[field(max_length = 10000)]
	pub code: String,

	#[field(max_length = 50)]
	pub language: String,

	#[field(auto_now_add = true)]
	pub created_at: DateTime<Utc>,
}

impl Snippet {
	/// Get a highlighted version of the code using the runtime highlighter.
	///
	/// Returns HTML-formatted code with syntax highlighting based on the snippet's language.
	/// Falls back to plain text if the language is not recognized.
	///
	/// # Example
	///
	/// ```no_run
	/// use examples_tutorial_rest::apps::snippets::models::Snippet;
	///
	/// let snippet = Snippet::build()
	///     .title("Hello World")
	///     .code("fn main() { println!(\"Hello!\"); }")
	///     .language("rust")
	///     .finish();
	/// let html = snippet.highlighted();
	/// assert!(html.contains("<span"));
	/// ```
	pub fn highlighted(&self) -> String {
		crate::native_runtime::highlighted_code(&self.language, &self.code)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	fn snippet(
		title: impl Into<String>,
		code: impl Into<String>,
		language: impl Into<String>,
	) -> Snippet {
		Snippet::build()
			.title(title)
			.code(code)
			.language(language)
			.finish()
	}

	/// Test that Snippet can be created with valid data
	#[rstest]
	fn test_snippet_creation() {
		// Arrange / Act
		let snippet = snippet("Hello World", "fn main() { println!(\"Hello!\"); }", "rust");

		// Assert
		assert_eq!(snippet.title, "Hello World");
		assert_eq!(snippet.language, "rust");
	}

	/// Test highlighted() method produces HTML with syntax highlighting for Rust
	#[rstest]
	fn test_highlighted_rust_code() {
		let snippet = snippet(
			"Rust Example",
			"fn main() { println!(\"Hello!\"); }",
			"rust",
		);

		let html = snippet.highlighted();

		// Verify HTML output contains span tags (syntax highlighting)
		assert!(
			html.contains("<span"),
			"Expected HTML with <span> tags for syntax highlighting"
		);
		// Verify key elements are present in the output
		assert!(html.contains("fn"), "Expected 'fn' keyword in output");
		assert!(html.contains("main"), "Expected 'main' in output");
	}

	/// Test highlighted() method works for Python language
	#[rstest]
	fn test_highlighted_python_code() {
		let snippet = snippet(
			"Python Example",
			"def hello():\n    print('Hello!')",
			"python",
		);

		let html = snippet.highlighted();

		// Verify HTML output contains span tags
		assert!(
			html.contains("<span"),
			"Expected HTML with <span> tags for Python syntax highlighting"
		);
		assert!(html.contains("def"), "Expected 'def' keyword in output");
	}

	/// Test highlighted() method falls back to plain text for unknown languages
	#[rstest]
	fn test_highlighted_unknown_language() {
		let snippet = snippet(
			"Unknown Language",
			"some unknown syntax here",
			"unknown_lang_xyz",
		);

		let html = snippet.highlighted();

		// Should still produce output (plain text highlighting)
		assert!(
			!html.is_empty(),
			"Expected non-empty output for unknown language"
		);
		assert!(
			html.contains("some unknown syntax here"),
			"Expected original code to be present in output"
		);
	}

	/// Test highlighted() method handles empty code gracefully
	#[rstest]
	fn test_highlighted_empty_code() {
		let snippet = snippet("Empty Code", "", "rust");

		let html = snippet.highlighted();

		// Should not panic on empty code and should return valid (possibly empty) HTML
		// The test verifies the method completes successfully without panicking
		// NOTE: Empty code produces minimal HTML structure from the highlighter,
		// exact output varies.
		let _ = html; // Verifies method returns successfully
	}

	/// Test highlighted() method handles multiline code correctly
	#[rstest]
	fn test_highlighted_multiline_code() {
		let snippet = snippet(
			"Multiline Rust",
			r#"fn main() {
    let x = 42;
    println!("{}", x);
}"#,
			"rust",
		);

		let html = snippet.highlighted();

		// Verify HTML contains expected elements
		assert!(
			html.contains("<span"),
			"Expected span tags in multiline output"
		);
		assert!(html.contains("let"), "Expected 'let' keyword");
		assert!(html.contains("42"), "Expected number literal");
	}
}
