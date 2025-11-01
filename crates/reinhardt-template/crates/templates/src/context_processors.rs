//! Context processors for templates
//!
//! Context processors provide global context variables that are automatically
//! available in all templates. This is useful for:
//! - Request information (path, method, etc.)
//! - User information (authentication status, username, etc.)
//! - Site-wide settings (site name, version, etc.)
//! - Feature flags and configuration

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Context processor function type
///
/// A context processor takes no arguments and returns a HashMap of variable names
/// to their string values.
pub type ContextProcessor = Arc<dyn Fn() -> HashMap<String, String> + Send + Sync>;

/// Registry for context processors
#[derive(Clone)]
pub struct ContextProcessorRegistry {
	processors: Arc<RwLock<Vec<ContextProcessor>>>,
}

impl ContextProcessorRegistry {
	/// Create a new context processor registry
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_templates::ContextProcessorRegistry;
	///
	/// let registry = ContextProcessorRegistry::new();
	/// ```
	pub fn new() -> Self {
		Self {
			processors: Arc::new(RwLock::new(Vec::new())),
		}
	}

	/// Register a context processor
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_templates::ContextProcessorRegistry;
	/// use std::collections::HashMap;
	///
	/// let mut registry = ContextProcessorRegistry::new();
	/// registry.register(|| {
	///     let mut context = HashMap::new();
	///     context.insert("site_name".to_string(), "My Site".to_string());
	///     context
	/// });
	/// ```
	pub fn register<F>(&mut self, processor: F)
	where
		F: Fn() -> HashMap<String, String> + Send + Sync + 'static,
	{
		if let Ok(mut processors) = self.processors.write() {
			processors.push(Arc::new(processor));
		}
	}

	/// Get all context variables from registered processors
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_templates::ContextProcessorRegistry;
	/// use std::collections::HashMap;
	///
	/// let mut registry = ContextProcessorRegistry::new();
	/// registry.register(|| {
	///     let mut context = HashMap::new();
	///     context.insert("site_name".to_string(), "My Site".to_string());
	///     context
	/// });
	///
	/// let context = registry.get_context();
	/// assert_eq!(context.get("site_name"), Some(&"My Site".to_string()));
	/// ```
	pub fn get_context(&self) -> HashMap<String, String> {
		let mut context = HashMap::new();

		if let Ok(processors) = self.processors.read() {
			for processor in processors.iter() {
				let proc_context = processor();
				context.extend(proc_context);
			}
		}

		context
	}

	/// Clear all registered processors
	pub fn clear(&mut self) {
		if let Ok(mut processors) = self.processors.write() {
			processors.clear();
		}
	}
}

impl Default for ContextProcessorRegistry {
	fn default() -> Self {
		Self::new()
	}
}

/// Built-in context processor for debug information
///
/// Provides:
/// - `DEBUG`: Whether debug mode is enabled
///
/// # Examples
///
/// ```
/// use reinhardt_templates::debug_context_processor;
///
/// let context = debug_context_processor(true);
/// assert_eq!(context.get("DEBUG"), Some(&"true".to_string()));
/// ```
pub fn debug_context_processor(debug: bool) -> HashMap<String, String> {
	let mut context = HashMap::new();
	context.insert("DEBUG".to_string(), debug.to_string());
	context
}

/// Built-in context processor for static files
///
/// Provides:
/// - `STATIC_URL`: Base URL for static files
///
/// # Examples
///
/// ```
/// use reinhardt_templates::static_context_processor;
///
/// let context = static_context_processor("/static/");
/// assert_eq!(context.get("STATIC_URL"), Some(&"/static/".to_string()));
/// ```
pub fn static_context_processor(static_url: &str) -> HashMap<String, String> {
	let mut context = HashMap::new();
	context.insert("STATIC_URL".to_string(), static_url.to_string());
	context
}

/// Built-in context processor for media files
///
/// Provides:
/// - `MEDIA_URL`: Base URL for media files
///
/// # Examples
///
/// ```
/// use reinhardt_templates::media_context_processor;
///
/// let context = media_context_processor("/media/");
/// assert_eq!(context.get("MEDIA_URL"), Some(&"/media/".to_string()));
/// ```
pub fn media_context_processor(media_url: &str) -> HashMap<String, String> {
	let mut context = HashMap::new();
	context.insert("MEDIA_URL".to_string(), media_url.to_string());
	context
}

/// Built-in context processor for request information
///
/// Provides:
/// - `REQUEST_PATH`: Current request path
/// - `REQUEST_METHOD`: HTTP method (GET, POST, etc.)
///
/// # Examples
///
/// ```
/// use reinhardt_templates::request_context_processor;
///
/// let context = request_context_processor("/users/123", "GET");
/// assert_eq!(context.get("REQUEST_PATH"), Some(&"/users/123".to_string()));
/// assert_eq!(context.get("REQUEST_METHOD"), Some(&"GET".to_string()));
/// ```
pub fn request_context_processor(path: &str, method: &str) -> HashMap<String, String> {
	let mut context = HashMap::new();
	context.insert("REQUEST_PATH".to_string(), path.to_string());
	context.insert("REQUEST_METHOD".to_string(), method.to_string());
	context
}

/// Built-in context processor for user information
///
/// Provides:
/// - `USER_AUTHENTICATED`: Whether user is authenticated
/// - `USERNAME`: Username if authenticated
///
/// # Examples
///
/// ```
/// use reinhardt_templates::user_context_processor;
///
/// let context = user_context_processor(true, Some("alice"));
/// assert_eq!(context.get("USER_AUTHENTICATED"), Some(&"true".to_string()));
/// assert_eq!(context.get("USERNAME"), Some(&"alice".to_string()));
///
/// let context = user_context_processor(false, None);
/// assert_eq!(context.get("USER_AUTHENTICATED"), Some(&"false".to_string()));
/// assert_eq!(context.get("USERNAME"), Some(&"".to_string()));
/// ```
pub fn user_context_processor(
	authenticated: bool,
	username: Option<&str>,
) -> HashMap<String, String> {
	let mut context = HashMap::new();
	context.insert("USER_AUTHENTICATED".to_string(), authenticated.to_string());
	context.insert("USERNAME".to_string(), username.unwrap_or("").to_string());
	context
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_registry_new() {
		let registry = ContextProcessorRegistry::new();
		let context = registry.get_context();
		assert!(context.is_empty());
	}

	#[test]
	fn test_registry_register() {
		let mut registry = ContextProcessorRegistry::new();
		registry.register(|| {
			let mut context = HashMap::new();
			context.insert("key1".to_string(), "value1".to_string());
			context
		});

		let context = registry.get_context();
		assert_eq!(context.get("key1"), Some(&"value1".to_string()));
	}

	#[test]
	fn test_registry_multiple_processors() {
		let mut registry = ContextProcessorRegistry::new();

		registry.register(|| {
			let mut context = HashMap::new();
			context.insert("key1".to_string(), "value1".to_string());
			context
		});

		registry.register(|| {
			let mut context = HashMap::new();
			context.insert("key2".to_string(), "value2".to_string());
			context
		});

		let context = registry.get_context();
		assert_eq!(context.get("key1"), Some(&"value1".to_string()));
		assert_eq!(context.get("key2"), Some(&"value2".to_string()));
	}

	#[test]
	fn test_registry_clear() {
		let mut registry = ContextProcessorRegistry::new();
		registry.register(|| {
			let mut context = HashMap::new();
			context.insert("key1".to_string(), "value1".to_string());
			context
		});

		let context = registry.get_context();
		assert!(!context.is_empty());

		registry.clear();
		let context = registry.get_context();
		assert!(context.is_empty());
	}

	#[test]
	fn test_debug_context_processor() {
		let context = debug_context_processor(true);
		assert_eq!(context.get("DEBUG"), Some(&"true".to_string()));

		let context = debug_context_processor(false);
		assert_eq!(context.get("DEBUG"), Some(&"false".to_string()));
	}

	#[test]
	fn test_static_context_processor() {
		let context = static_context_processor("/static/");
		assert_eq!(context.get("STATIC_URL"), Some(&"/static/".to_string()));
	}

	#[test]
	fn test_media_context_processor() {
		let context = media_context_processor("/media/");
		assert_eq!(context.get("MEDIA_URL"), Some(&"/media/".to_string()));
	}

	#[test]
	fn test_request_context_processor() {
		let context = request_context_processor("/users/123", "GET");
		assert_eq!(context.get("REQUEST_PATH"), Some(&"/users/123".to_string()));
		assert_eq!(context.get("REQUEST_METHOD"), Some(&"GET".to_string()));
	}

	#[test]
	fn test_user_context_processor_authenticated() {
		let context = user_context_processor(true, Some("alice"));
		assert_eq!(context.get("USER_AUTHENTICATED"), Some(&"true".to_string()));
		assert_eq!(context.get("USERNAME"), Some(&"alice".to_string()));
	}

	#[test]
	fn test_user_context_processor_not_authenticated() {
		let context = user_context_processor(false, None);
		assert_eq!(
			context.get("USER_AUTHENTICATED"),
			Some(&"false".to_string())
		);
		assert_eq!(context.get("USERNAME"), Some(&"".to_string()));
	}
}
