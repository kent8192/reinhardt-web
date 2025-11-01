//! Signal documentation generator for auto-generating docs from signal definitions
//!
//! Automatically generates documentation from signal metadata, including
//! receiver information, signal flow diagrams, and usage examples.
//!
//! # Examples
//!
//! ```
//! use reinhardt_signals::{Signal, SignalName};
//! use reinhardt_signals::doc_generator::{SignalDocGenerator, SignalDocumentation};
//!
//! let signal = Signal::<String>::new(SignalName::PRE_SAVE);
//!
//! let mut generator = SignalDocGenerator::new();
//! generator.add_signal_doc(SignalDocumentation {
//!     signal_name: "pre_save".to_string(),
//!     description: "Sent before a model is saved".to_string(),
//!     receivers: vec![],
//!     example_usage: None,
//! });
//!
//! let markdown = generator.generate_markdown();
//! assert!(markdown.contains("# Signal Documentation"));
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Documentation for a single signal receiver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiverDocumentation {
	/// Receiver identifier (dispatch_uid)
	pub receiver_id: String,
	/// Description of what this receiver does
	pub description: String,
	/// Priority level (higher executes first)
	pub priority: i32,
	/// Whether this receiver is critical
	pub is_critical: bool,
	/// Expected execution time (in milliseconds)
	pub expected_duration_ms: Option<u64>,
}

/// Documentation for a signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalDocumentation {
	/// Signal name
	pub signal_name: String,
	/// Description of when and why this signal is sent
	pub description: String,
	/// List of receivers connected to this signal
	pub receivers: Vec<ReceiverDocumentation>,
	/// Example usage code
	pub example_usage: Option<String>,
}

/// Signal documentation generator
///
/// Collects and generates documentation for signals and their receivers
pub struct SignalDocGenerator {
	signals: HashMap<String, SignalDocumentation>,
}

impl SignalDocGenerator {
	/// Create a new documentation generator
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_signals::doc_generator::SignalDocGenerator;
	///
	/// let generator = SignalDocGenerator::new();
	/// ```
	pub fn new() -> Self {
		Self {
			signals: HashMap::new(),
		}
	}

	/// Add documentation for a signal
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_signals::doc_generator::{SignalDocGenerator, SignalDocumentation};
	///
	/// let mut generator = SignalDocGenerator::new();
	/// generator.add_signal_doc(SignalDocumentation {
	///     signal_name: "user_created".to_string(),
	///     description: "Sent when a new user is created".to_string(),
	///     receivers: vec![],
	///     example_usage: None,
	/// });
	/// ```
	pub fn add_signal_doc(&mut self, doc: SignalDocumentation) {
		self.signals.insert(doc.signal_name.clone(), doc);
	}

	/// Add a receiver to an existing signal's documentation
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_signals::doc_generator::{SignalDocGenerator, SignalDocumentation, ReceiverDocumentation};
	///
	/// let mut generator = SignalDocGenerator::new();
	/// generator.add_signal_doc(SignalDocumentation {
	///     signal_name: "user_created".to_string(),
	///     description: "Sent when a new user is created".to_string(),
	///     receivers: vec![],
	///     example_usage: None,
	/// });
	///
	/// generator.add_receiver_to_signal(
	///     "user_created",
	///     ReceiverDocumentation {
	///         receiver_id: "send_welcome_email".to_string(),
	///         description: "Sends a welcome email to the new user".to_string(),
	///         priority: 0,
	///         is_critical: true,
	///         expected_duration_ms: Some(100),
	///     },
	/// );
	/// ```
	pub fn add_receiver_to_signal(&mut self, signal_name: &str, receiver: ReceiverDocumentation) {
		if let Some(signal_doc) = self.signals.get_mut(signal_name) {
			signal_doc.receivers.push(receiver);
		}
	}

	/// Generate Markdown documentation
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_signals::doc_generator::{SignalDocGenerator, SignalDocumentation};
	///
	/// let mut generator = SignalDocGenerator::new();
	/// generator.add_signal_doc(SignalDocumentation {
	///     signal_name: "test_signal".to_string(),
	///     description: "A test signal".to_string(),
	///     receivers: vec![],
	///     example_usage: None,
	/// });
	///
	/// let markdown = generator.generate_markdown();
	/// assert!(markdown.contains("# Signal Documentation"));
	/// ```
	pub fn generate_markdown(&self) -> String {
		let mut output = String::from("# Signal Documentation\n\n");
		output.push_str(
			"This document provides comprehensive documentation for all signals in the system.\n\n",
		);

		if self.signals.is_empty() {
			output.push_str("*No signals documented yet.*\n");
			return output;
		}

		output.push_str("## Table of Contents\n\n");
		let mut signal_names: Vec<_> = self.signals.keys().collect();
		signal_names.sort();

		for name in &signal_names {
			output.push_str(&format!("- [{}](#{})\n", name, name.replace('_', "-")));
		}
		output.push('\n');

		// Generate documentation for each signal
		for name in signal_names {
			if let Some(doc) = self.signals.get(name) {
				output.push_str(&self.generate_signal_markdown(doc));
				output.push('\n');
			}
		}

		output
	}

	fn generate_signal_markdown(&self, doc: &SignalDocumentation) -> String {
		let mut output = String::new();

		output.push_str(&format!("## {}\n\n", doc.signal_name));
		output.push_str(&format!("{}\n\n", doc.description));

		if !doc.receivers.is_empty() {
			output.push_str("### Connected Receivers\n\n");

			let mut receivers = doc.receivers.clone();
			receivers.sort_by(|a, b| b.priority.cmp(&a.priority));

			output
				.push_str("| Receiver | Description | Priority | Critical | Expected Duration |\n");
			output
				.push_str("|----------|-------------|----------|----------|------------------|\n");

			for receiver in receivers {
				let critical = if receiver.is_critical { "✓" } else { "" };
				let duration = receiver
					.expected_duration_ms
					.map(|ms| format!("{}ms", ms))
					.unwrap_or_else(|| "-".to_string());

				output.push_str(&format!(
					"| {} | {} | {} | {} | {} |\n",
					receiver.receiver_id,
					receiver.description,
					receiver.priority,
					critical,
					duration
				));
			}
			output.push('\n');
		}

		if let Some(example) = &doc.example_usage {
			output.push_str("### Example Usage\n\n");
			output.push_str("```rust\n");
			output.push_str(example);
			output.push_str("\n```\n\n");
		}

		output.push_str("---\n\n");
		output
	}

	/// Generate HTML documentation
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_signals::doc_generator::{SignalDocGenerator, SignalDocumentation};
	///
	/// let mut generator = SignalDocGenerator::new();
	/// generator.add_signal_doc(SignalDocumentation {
	///     signal_name: "test_signal".to_string(),
	///     description: "A test signal".to_string(),
	///     receivers: vec![],
	///     example_usage: None,
	/// });
	///
	/// let html = generator.generate_html();
	/// assert!(html.contains("<h1>Signal Documentation</h1>"));
	/// ```
	pub fn generate_html(&self) -> String {
		let mut output = String::from("<!DOCTYPE html>\n<html>\n<head>\n");
		output.push_str("  <title>Signal Documentation</title>\n");
		output.push_str("  <style>\n");
		output.push_str("    body { font-family: Arial, sans-serif; margin: 20px; }\n");
		output.push_str("    h1 { color: #333; }\n");
		output.push_str(
			"    h2 { color: #666; border-bottom: 2px solid #ddd; padding-bottom: 5px; }\n",
		);
		output.push_str("    table { border-collapse: collapse; width: 100%; margin: 20px 0; }\n");
		output.push_str("    th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }\n");
		output.push_str("    th { background-color: #f2f2f2; }\n");
		output.push_str(
			"    pre { background-color: #f5f5f5; padding: 10px; border-radius: 5px; }\n",
		);
		output.push_str("    .critical { color: red; font-weight: bold; }\n");
		output.push_str("  </style>\n");
		output.push_str("</head>\n<body>\n");

		output.push_str("  <h1>Signal Documentation</h1>\n");
		output.push_str("  <p>Comprehensive documentation for all signals in the system.</p>\n");

		if self.signals.is_empty() {
			output.push_str("  <p><em>No signals documented yet.</em></p>\n");
		} else {
			let mut signal_names: Vec<_> = self.signals.keys().collect();
			signal_names.sort();

			for name in signal_names {
				if let Some(doc) = self.signals.get(name) {
					output.push_str(&self.generate_signal_html(doc));
				}
			}
		}

		output.push_str("</body>\n</html>");
		output
	}

	fn generate_signal_html(&self, doc: &SignalDocumentation) -> String {
		let mut output = String::new();

		output.push_str(&format!("  <h2>{}</h2>\n", doc.signal_name));
		output.push_str(&format!("  <p>{}</p>\n", doc.description));

		if !doc.receivers.is_empty() {
			output.push_str("  <h3>Connected Receivers</h3>\n");
			output.push_str("  <table>\n");
			output.push_str("    <tr>\n");
			output.push_str("      <th>Receiver</th>\n");
			output.push_str("      <th>Description</th>\n");
			output.push_str("      <th>Priority</th>\n");
			output.push_str("      <th>Critical</th>\n");
			output.push_str("      <th>Expected Duration</th>\n");
			output.push_str("    </tr>\n");

			let mut receivers = doc.receivers.clone();
			receivers.sort_by(|a, b| b.priority.cmp(&a.priority));

			for receiver in receivers {
				let critical = if receiver.is_critical {
					"<span class=\"critical\">✓</span>"
				} else {
					""
				};
				let duration = receiver
					.expected_duration_ms
					.map(|ms| format!("{}ms", ms))
					.unwrap_or_else(|| "-".to_string());

				output.push_str("    <tr>\n");
				output.push_str(&format!("      <td>{}</td>\n", receiver.receiver_id));
				output.push_str(&format!("      <td>{}</td>\n", receiver.description));
				output.push_str(&format!("      <td>{}</td>\n", receiver.priority));
				output.push_str(&format!("      <td>{}</td>\n", critical));
				output.push_str(&format!("      <td>{}</td>\n", duration));
				output.push_str("    </tr>\n");
			}

			output.push_str("  </table>\n");
		}

		if let Some(example) = &doc.example_usage {
			output.push_str("  <h3>Example Usage</h3>\n");
			output.push_str("  <pre><code>");
			output.push_str(&html_escape(example));
			output.push_str("</code></pre>\n");
		}

		output.push_str("  <hr>\n");
		output
	}

	/// Export documentation as JSON
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_signals::doc_generator::{SignalDocGenerator, SignalDocumentation};
	///
	/// let mut generator = SignalDocGenerator::new();
	/// generator.add_signal_doc(SignalDocumentation {
	///     signal_name: "test_signal".to_string(),
	///     description: "A test signal".to_string(),
	///     receivers: vec![],
	///     example_usage: None,
	/// });
	///
	/// let json = generator.export_json();
	/// assert!(json.is_ok());
	/// ```
	pub fn export_json(&self) -> Result<String, serde_json::Error> {
		serde_json::to_string_pretty(&self.signals)
	}

	/// Import documentation from JSON
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_signals::doc_generator::SignalDocGenerator;
	///
	/// let mut generator = SignalDocGenerator::new();
	/// let json = r#"{"test_signal": {"signal_name": "test_signal", "description": "Test", "receivers": [], "example_usage": null}}"#;
	/// generator.import_json(json).unwrap();
	///
	/// let markdown = generator.generate_markdown();
	/// assert!(markdown.contains("test_signal"));
	/// ```
	pub fn import_json(&mut self, json: &str) -> Result<(), serde_json::Error> {
		self.signals = serde_json::from_str(json)?;
		Ok(())
	}

	/// Get list of all documented signals
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_signals::doc_generator::{SignalDocGenerator, SignalDocumentation};
	///
	/// let mut generator = SignalDocGenerator::new();
	/// generator.add_signal_doc(SignalDocumentation {
	///     signal_name: "signal1".to_string(),
	///     description: "First signal".to_string(),
	///     receivers: vec![],
	///     example_usage: None,
	/// });
	///
	/// let signals = generator.list_signals();
	/// assert_eq!(signals.len(), 1);
	/// ```
	pub fn list_signals(&self) -> Vec<String> {
		let mut names: Vec<_> = self.signals.keys().cloned().collect();
		names.sort();
		names
	}

	/// Get documentation for a specific signal
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_signals::doc_generator::{SignalDocGenerator, SignalDocumentation};
	///
	/// let mut generator = SignalDocGenerator::new();
	/// generator.add_signal_doc(SignalDocumentation {
	///     signal_name: "test".to_string(),
	///     description: "Test signal".to_string(),
	///     receivers: vec![],
	///     example_usage: None,
	/// });
	///
	/// let doc = generator.get_signal_doc("test");
	/// assert!(doc.is_some());
	/// ```
	pub fn get_signal_doc(&self, signal_name: &str) -> Option<&SignalDocumentation> {
		self.signals.get(signal_name)
	}
}

impl Default for SignalDocGenerator {
	fn default() -> Self {
		Self::new()
	}
}

/// Helper function to escape HTML special characters
fn html_escape(s: &str) -> String {
	s.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('"', "&quot;")
		.replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_generate_markdown() {
		let mut generator = SignalDocGenerator::new();

		generator.add_signal_doc(SignalDocumentation {
			signal_name: "user_created".to_string(),
			description: "Sent when a new user is created".to_string(),
			receivers: vec![ReceiverDocumentation {
				receiver_id: "send_email".to_string(),
				description: "Sends welcome email".to_string(),
				priority: 10,
				is_critical: true,
				expected_duration_ms: Some(100),
			}],
			example_usage: Some("signal.send(user).await?;".to_string()),
		});

		let markdown = generator.generate_markdown();
		assert!(markdown.contains("# Signal Documentation"));
		assert!(markdown.contains("user_created"));
		assert!(markdown.contains("send_email"));
	}

	#[test]
	fn test_generate_html() {
		let mut generator = SignalDocGenerator::new();

		generator.add_signal_doc(SignalDocumentation {
			signal_name: "test_signal".to_string(),
			description: "A test signal".to_string(),
			receivers: vec![],
			example_usage: None,
		});

		let html = generator.generate_html();
		assert!(html.contains("<h1>Signal Documentation</h1>"));
		assert!(html.contains("test_signal"));
	}

	#[test]
	fn test_json_export_import() {
		let mut generator = SignalDocGenerator::new();

		generator.add_signal_doc(SignalDocumentation {
			signal_name: "test".to_string(),
			description: "Test signal".to_string(),
			receivers: vec![],
			example_usage: None,
		});

		let json = generator.export_json().unwrap();

		let mut new_generator = SignalDocGenerator::new();
		new_generator.import_json(&json).unwrap();

		assert_eq!(new_generator.list_signals().len(), 1);
		assert!(new_generator.get_signal_doc("test").is_some());
	}

	#[test]
	fn test_add_receiver_to_signal() {
		let mut generator = SignalDocGenerator::new();

		generator.add_signal_doc(SignalDocumentation {
			signal_name: "test".to_string(),
			description: "Test signal".to_string(),
			receivers: vec![],
			example_usage: None,
		});

		generator.add_receiver_to_signal(
			"test",
			ReceiverDocumentation {
				receiver_id: "receiver1".to_string(),
				description: "First receiver".to_string(),
				priority: 0,
				is_critical: false,
				expected_duration_ms: None,
			},
		);

		let doc = generator.get_signal_doc("test").unwrap();
		assert_eq!(doc.receivers.len(), 1);
	}

	#[test]
	fn test_html_escape() {
		let input = "<script>alert('XSS')</script>";
		let escaped = html_escape(input);
		assert_eq!(escaped, "&lt;script&gt;alert(&#39;XSS&#39;)&lt;/script&gt;");
	}
}
