//! System check framework
//!
//! Provides a framework for running validation checks across the Reinhardt framework.
//! This is similar to Django's check framework.
//!
//! # Examples
//!
//! ```rust
//! use reinhardt_utils::utils_core::checks::{Check, CheckLevel, CheckMessage, CheckRegistry};
//!
//! struct MyCheck;
//!
//! impl Check for MyCheck {
//!     fn tags(&self) -> Vec<String> {
//!         vec!["myapp".to_string()]
//!     }
//!
//!     fn check(&self) -> Vec<CheckMessage> {
//!         vec![CheckMessage::warning("myapp.W001", "This is a warning")]
//!     }
//! }
//!
//! let mut registry = CheckRegistry::new();
//! registry.register(Box::new(MyCheck));
//!
//! let messages = registry.run_checks(&[]);
//! for message in messages {
//!     println!("[{}] {}", message.id, message.message);
//! }
//! ```

use std::sync::Mutex;

/// Check severity level
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckLevel {
	/// Debug-level information
	Debug,
	/// Informational message
	Info,
	/// Warning that should be addressed
	Warning,
	/// Error that must be fixed
	Error,
	/// Critical error that prevents operation
	Critical,
}

/// A message produced by a check
#[derive(Debug, Clone)]
pub struct CheckMessage {
	/// Severity level
	pub level: CheckLevel,
	/// Unique identifier (e.g., "static.E001")
	pub id: String,
	/// Human-readable message
	pub message: String,
	/// Optional hint for resolving the issue
	pub hint: Option<String>,
}

impl CheckMessage {
	/// Create a debug-level message
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::utils_core::checks::CheckMessage;
	///
	/// let msg = CheckMessage::debug("app.D001", "Debug information");
	/// ```
	pub fn debug(id: impl Into<String>, message: impl Into<String>) -> Self {
		Self {
			level: CheckLevel::Debug,
			id: id.into(),
			message: message.into(),
			hint: None,
		}
	}

	/// Create an info-level message
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::utils_core::checks::CheckMessage;
	///
	/// let msg = CheckMessage::info("app.I001", "Information");
	/// ```
	pub fn info(id: impl Into<String>, message: impl Into<String>) -> Self {
		Self {
			level: CheckLevel::Info,
			id: id.into(),
			message: message.into(),
			hint: None,
		}
	}

	/// Create a warning-level message
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::utils_core::checks::CheckMessage;
	///
	/// let msg = CheckMessage::warning("app.W001", "This is a warning");
	/// ```
	pub fn warning(id: impl Into<String>, message: impl Into<String>) -> Self {
		Self {
			level: CheckLevel::Warning,
			id: id.into(),
			message: message.into(),
			hint: None,
		}
	}

	/// Create an error-level message
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::utils_core::checks::CheckMessage;
	///
	/// let msg = CheckMessage::error("app.E001", "This is an error");
	/// ```
	pub fn error(id: impl Into<String>, message: impl Into<String>) -> Self {
		Self {
			level: CheckLevel::Error,
			id: id.into(),
			message: message.into(),
			hint: None,
		}
	}

	/// Create a critical-level message
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::utils_core::checks::CheckMessage;
	///
	/// let msg = CheckMessage::critical("app.C001", "Critical error");
	/// ```
	pub fn critical(id: impl Into<String>, message: impl Into<String>) -> Self {
		Self {
			level: CheckLevel::Critical,
			id: id.into(),
			message: message.into(),
			hint: None,
		}
	}

	/// Add a hint to the check message
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::utils_core::checks::CheckMessage;
	///
	/// let msg = CheckMessage::error("app.E001", "Error occurred")
	///     .with_hint("Try fixing the configuration");
	/// ```
	pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
		self.hint = Some(hint.into());
		self
	}
}

/// Trait for implementing system checks
///
/// # Examples
///
/// ```rust
/// use reinhardt_utils::utils_core::checks::{Check, CheckMessage};
///
/// struct DatabaseCheck;
///
/// impl Check for DatabaseCheck {
///     fn tags(&self) -> Vec<String> {
///         vec!["database".to_string()]
///     }
///
///     fn check(&self) -> Vec<CheckMessage> {
///         // Perform validation
///         vec![]
///     }
/// }
/// ```
pub trait Check: Send + Sync {
	/// Return the tags this check belongs to
	///
	/// Tags are used to group related checks together.
	/// Examples: "staticfiles", "models", "database", "security"
	fn tags(&self) -> Vec<String>;

	/// Perform the check and return any messages
	///
	/// Returns a vector of CheckMessage instances describing any issues found.
	fn check(&self) -> Vec<CheckMessage>;
}

/// Registry for system checks
///
/// # Examples
///
/// ```rust
/// use reinhardt_utils::utils_core::checks::{Check, CheckMessage, CheckRegistry};
///
/// struct MyCheck;
///
/// impl Check for MyCheck {
///     fn tags(&self) -> Vec<String> {
///         vec!["myapp".to_string()]
///     }
///
///     fn check(&self) -> Vec<CheckMessage> {
///         vec![]
///     }
/// }
///
/// let mut registry = CheckRegistry::new();
/// registry.register(Box::new(MyCheck));
///
/// // Run all checks
/// let messages = registry.run_checks(&[]);
///
/// // Run checks for specific tags
/// let messages = registry.run_checks(&["myapp".to_string()]);
/// ```
pub struct CheckRegistry {
	checks: Vec<Box<dyn Check>>,
}

impl CheckRegistry {
	/// Create a new check registry
	pub fn new() -> Self {
		Self { checks: Vec::new() }
	}

	/// Register a check
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::utils_core::checks::{Check, CheckMessage, CheckRegistry};
	///
	/// struct MyCheck;
	///
	/// impl Check for MyCheck {
	///     fn tags(&self) -> Vec<String> {
	///         vec!["myapp".to_string()]
	///     }
	///
	///     fn check(&self) -> Vec<CheckMessage> {
	///         vec![]
	///     }
	/// }
	///
	/// let mut registry = CheckRegistry::new();
	/// registry.register(Box::new(MyCheck));
	/// ```
	pub fn register(&mut self, check: Box<dyn Check>) {
		self.checks.push(check);
	}

	/// Run checks, optionally filtered by tags
	///
	/// If `tags` is empty, all checks are run.
	/// If `tags` contains values, only checks with matching tags are run.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::utils_core::checks::CheckRegistry;
	///
	/// let registry = CheckRegistry::new();
	///
	/// // Run all checks
	/// let messages = registry.run_checks(&[]);
	///
	/// // Run only staticfiles checks
	/// let messages = registry.run_checks(&["staticfiles".to_string()]);
	/// ```
	pub fn run_checks(&self, tags: &[String]) -> Vec<CheckMessage> {
		let mut messages = Vec::new();

		for check in &self.checks {
			// If no tags specified, run all checks
			// If tags specified, only run checks that have matching tags
			let should_run = if tags.is_empty() {
				true
			} else {
				let check_tags = check.tags();
				tags.iter().any(|tag| check_tags.contains(tag))
			};

			if should_run {
				messages.extend(check.check());
			}
		}

		messages
	}

	/// Get the global check registry
	///
	/// This returns a mutable reference to a thread-local registry.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_utils::utils_core::checks::{Check, CheckMessage, CheckRegistry};
	///
	/// struct MyCheck;
	///
	/// impl Check for MyCheck {
	///     fn tags(&self) -> Vec<String> {
	///         vec!["myapp".to_string()]
	///     }
	///
	///     fn check(&self) -> Vec<CheckMessage> {
	///         vec![]
	///     }
	/// }
	///
	/// let mut registry = CheckRegistry::global();
	/// registry.lock().unwrap().register(Box::new(MyCheck));
	/// ```
	pub fn global() -> &'static Mutex<CheckRegistry> {
		static REGISTRY: std::sync::OnceLock<Mutex<CheckRegistry>> = std::sync::OnceLock::new();
		REGISTRY.get_or_init(|| Mutex::new(CheckRegistry::new()))
	}
}

impl Default for CheckRegistry {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	struct TestCheck {
		tags: Vec<String>,
		messages: Vec<CheckMessage>,
	}

	impl Check for TestCheck {
		fn tags(&self) -> Vec<String> {
			self.tags.clone()
		}

		fn check(&self) -> Vec<CheckMessage> {
			self.messages.clone()
		}
	}

	#[rstest]
	fn test_check_message_levels() {
		let debug = CheckMessage::debug("test.D001", "Debug");
		assert!(matches!(debug.level, CheckLevel::Debug));

		let info = CheckMessage::info("test.I001", "Info");
		assert!(matches!(info.level, CheckLevel::Info));

		let warning = CheckMessage::warning("test.W001", "Warning");
		assert!(matches!(warning.level, CheckLevel::Warning));

		let error = CheckMessage::error("test.E001", "Error");
		assert!(matches!(error.level, CheckLevel::Error));

		let critical = CheckMessage::critical("test.C001", "Critical");
		assert!(matches!(critical.level, CheckLevel::Critical));
	}

	#[rstest]
	fn test_check_message_with_hint() {
		let msg = CheckMessage::error("test.E001", "Error occurred").with_hint("Try this fix");

		assert_eq!(msg.hint, Some("Try this fix".to_string()));
	}

	#[rstest]
	fn test_check_registry_new() {
		let registry = CheckRegistry::new();
		assert_eq!(registry.checks.len(), 0);
	}

	#[rstest]
	fn test_check_registry_register() {
		let mut registry = CheckRegistry::new();

		let check = TestCheck {
			tags: vec!["test".to_string()],
			messages: vec![],
		};

		registry.register(Box::new(check));
		assert_eq!(registry.checks.len(), 1);
	}

	#[rstest]
	fn test_check_registry_run_all_checks() {
		let mut registry = CheckRegistry::new();

		let check1 = TestCheck {
			tags: vec!["tag1".to_string()],
			messages: vec![CheckMessage::info("test.I001", "Info 1")],
		};

		let check2 = TestCheck {
			tags: vec!["tag2".to_string()],
			messages: vec![CheckMessage::warning("test.W001", "Warning 1")],
		};

		registry.register(Box::new(check1));
		registry.register(Box::new(check2));

		let messages = registry.run_checks(&[]);
		assert_eq!(messages.len(), 2);
	}

	#[rstest]
	fn test_check_registry_run_filtered_checks() {
		let mut registry = CheckRegistry::new();

		let check1 = TestCheck {
			tags: vec!["tag1".to_string()],
			messages: vec![CheckMessage::info("test.I001", "Info 1")],
		};

		let check2 = TestCheck {
			tags: vec!["tag2".to_string()],
			messages: vec![CheckMessage::warning("test.W001", "Warning 1")],
		};

		registry.register(Box::new(check1));
		registry.register(Box::new(check2));

		let messages = registry.run_checks(&["tag1".to_string()]);
		assert_eq!(messages.len(), 1);
		assert_eq!(messages[0].id, "test.I001");
	}

	#[rstest]
	fn test_check_registry_multiple_tags() {
		let mut registry = CheckRegistry::new();

		let check = TestCheck {
			tags: vec!["tag1".to_string(), "tag2".to_string()],
			messages: vec![CheckMessage::info("test.I001", "Info")],
		};

		registry.register(Box::new(check));

		let messages1 = registry.run_checks(&["tag1".to_string()]);
		assert_eq!(messages1.len(), 1);

		let messages2 = registry.run_checks(&["tag2".to_string()]);
		assert_eq!(messages2.len(), 1);

		let messages3 = registry.run_checks(&["tag3".to_string()]);
		assert_eq!(messages3.len(), 0);
	}
}
