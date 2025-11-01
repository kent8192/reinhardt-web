//! Template Rendering Strategy Selection
//!
//! This module provides strategy selection logic for choosing between
//! runtime and compile-time template rendering based on template characteristics.
//!
//! # Strategy Overview
//!
//! ## Compile-time (Tera with include_str!)
//!
//! **Pros:**
//! - Templates embedded at compile time
//! - Fast template loading (no file I/O at runtime)
//! - Compile-time template validation
//! - Reduced runtime overhead
//!
//! **Cons:**
//! - Templates must be known at compile time
//! - Requires recompilation for template changes
//! - Cannot use user-provided templates
//!
//! **Use Cases:**
//! - View templates (HTML pages)
//! - Email templates
//! - Static response pages
//! - Developer-managed templates
//!
//! ## Runtime (TemplateHTMLRenderer)
//!
//! **Pros:**
//! - Flexible, dynamic templates
//! - Templates can be loaded at runtime
//! - User-provided templates supported
//! - No recompilation needed
//!
//! **Cons:**
//! - Slower than compile-time (but optimized in Phase 2)
//! - No compile-time type checking
//! - Runtime syntax validation
//!
//! **Use Cases:**
//! - Configuration file templates
//! - User-provided templates
//! - Templates stored in database
//! - Dynamic template generation
//!
//! # Examples
//!
//! ```
//! use reinhardt_renderers::strategy::{TemplateStrategy, TemplateStrategySelector, TemplateSource};
//!
//! // Static template → Use compile-time
//! let source = TemplateSource::Static("user.html");
//! let strategy = TemplateStrategySelector::select(&source);
//! assert!(matches!(strategy, TemplateStrategy::CompileTime));
//!
//! // Dynamic template → Use runtime
//! let source = TemplateSource::Dynamic("<h1>{{ title }}</h1>".to_string());
//! let strategy = TemplateStrategySelector::select(&source);
//! assert!(matches!(strategy, TemplateStrategy::Runtime));
//! ```

/// Template rendering strategy
///
/// Determines whether to use compile-time or runtime template rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateStrategy {
	/// Compile-time rendering using Tera with include_str!
	///
	/// - **Time Complexity**: O(n) - Templates embedded at compile time
	/// - **Performance**: Faster than file-based loading
	/// - **Type Safety**: Runtime type validation
	/// - **Flexibility**: Templates must be known at compile time
	///
	/// # Use Cases
	///
	/// - View templates
	/// - Email templates
	/// - Static pages
	/// - Developer-managed templates
	CompileTime,

	/// Runtime rendering using TemplateHTMLRenderer
	///
	/// - **Time Complexity**: O(n + m) where n=template length, m=variables (Phase 2 optimized)
	/// - **Performance**: Optimized single-pass substitution
	/// - **Type Safety**: Runtime validation
	/// - **Flexibility**: Full dynamic template support
	///
	/// # Use Cases
	///
	/// - User-provided templates
	/// - Configuration templates
	/// - Database-stored templates
	/// - Dynamic template generation
	Runtime,
}

/// Template source type
///
/// Indicates where the template comes from and how it should be processed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateSource {
	/// Static template known at compile time
	///
	/// Example: `TemplateSource::Static("user.html")`
	///
	/// This typically refers to a template file in the `templates/` directory
	/// that is compiled with the application.
	Static(&'static str),

	/// Dynamic template provided at runtime
	///
	/// Example: `TemplateSource::Dynamic("<h1>{{ title }}</h1>".to_string())`
	///
	/// This is used for templates that are:
	/// - User-provided
	/// - Loaded from database
	/// - Generated dynamically
	/// - Configuration-based
	Dynamic(String),

	/// Template loaded from file path
	///
	/// Example: `TemplateSource::File("/path/to/template.html".to_string())`
	///
	/// The file extension determines the rendering strategy:
	/// - `.tera` or `.jinja` → Compile-time (if compiled)
	/// - Other extensions → Runtime
	File(String),
}

/// Template strategy selector
///
/// Analyzes template sources and selects the optimal rendering strategy.
pub struct TemplateStrategySelector;

impl TemplateStrategySelector {
	/// Selects the best rendering strategy for a given template source
	///
	/// # Selection Logic
	///
	/// 1. **Static templates** → Compile-time (Tera with include_str!)
	///    - Templates known at compile time
	///    - Embedded templates for fast loading
	///
	/// 2. **Dynamic templates** → Runtime (TemplateHTMLRenderer)
	///    - Templates provided at runtime
	///    - Maximum flexibility
	///
	/// 3. **File-based templates** → Based on extension
	///    - `.tera`, `.jinja`, `.tpl` → Compile-time (if pre-compiled)
	///    - Other extensions → Runtime
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::strategy::{TemplateStrategy, TemplateStrategySelector, TemplateSource};
	///
	/// // Static template
	/// let source = TemplateSource::Static("user.html");
	/// let strategy = TemplateStrategySelector::select(&source);
	/// assert_eq!(strategy, TemplateStrategy::CompileTime);
	///
	/// // Dynamic template
	/// let source = TemplateSource::Dynamic("<h1>{{ title }}</h1>".to_string());
	/// let strategy = TemplateStrategySelector::select(&source);
	/// assert_eq!(strategy, TemplateStrategy::Runtime);
	///
	/// // File-based with Tera extension
	/// let source = TemplateSource::File("template.tera".to_string());
	/// let strategy = TemplateStrategySelector::select(&source);
	/// assert_eq!(strategy, TemplateStrategy::CompileTime);
	///
	/// // File-based with regular extension
	/// let source = TemplateSource::File("template.html".to_string());
	/// let strategy = TemplateStrategySelector::select(&source);
	/// assert_eq!(strategy, TemplateStrategy::Runtime);
	/// ```
	pub fn select(template_source: &TemplateSource) -> TemplateStrategy {
		match template_source {
			// Static templates known at compile time → Use Tera with include_str!
			TemplateSource::Static(_) => TemplateStrategy::CompileTime,

			// Dynamic templates provided at runtime → Use TemplateHTMLRenderer
			TemplateSource::Dynamic(_) => TemplateStrategy::Runtime,

			// File-based: Check extension
			TemplateSource::File(path) => {
				if path.ends_with(".tera") || path.ends_with(".jinja") || path.ends_with(".tpl") {
					// Template-specific extensions → Compile-time (if pre-compiled)
					TemplateStrategy::CompileTime
				} else {
					// Regular extensions → Runtime for flexibility
					TemplateStrategy::Runtime
				}
			}
		}
	}

	/// Recommends a strategy based on use case description
	///
	/// This is a helper method for documentation and planning purposes.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::strategy::{TemplateStrategy, TemplateStrategySelector};
	///
	/// let strategy = TemplateStrategySelector::recommend_for_use_case("view template");
	/// assert_eq!(strategy, TemplateStrategy::CompileTime);
	///
	/// let strategy = TemplateStrategySelector::recommend_for_use_case("user template");
	/// assert_eq!(strategy, TemplateStrategy::Runtime);
	/// ```
	pub fn recommend_for_use_case(use_case: &str) -> TemplateStrategy {
		let use_case_lower = use_case.to_lowercase();

		// Compile-time use cases
		if use_case_lower.contains("view")
			|| use_case_lower.contains("email")
			|| use_case_lower.contains("static")
			|| use_case_lower.contains("page")
		{
			return TemplateStrategy::CompileTime;
		}

		// Runtime use cases
		if use_case_lower.contains("user")
			|| use_case_lower.contains("config")
			|| use_case_lower.contains("dynamic")
			|| use_case_lower.contains("database")
		{
			return TemplateStrategy::Runtime;
		}

		// Default to runtime for unknown cases (safer, more flexible)
		TemplateStrategy::Runtime
	}
}

impl TemplateSource {
	/// Checks if this source represents a static template
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::strategy::TemplateSource;
	///
	/// let source = TemplateSource::Static("user.html");
	/// assert!(source.is_static());
	///
	/// let source = TemplateSource::Dynamic("<h1>{{ title }}</h1>".to_string());
	/// assert!(!source.is_static());
	/// ```
	pub fn is_static(&self) -> bool {
		matches!(self, TemplateSource::Static(_))
	}

	/// Checks if this source represents a dynamic template
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::strategy::TemplateSource;
	///
	/// let source = TemplateSource::Dynamic("<h1>{{ title }}</h1>".to_string());
	/// assert!(source.is_dynamic());
	///
	/// let source = TemplateSource::Static("user.html");
	/// assert!(!source.is_dynamic());
	/// ```
	pub fn is_dynamic(&self) -> bool {
		matches!(self, TemplateSource::Dynamic(_))
	}

	/// Checks if this source represents a file-based template
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::strategy::TemplateSource;
	///
	/// let source = TemplateSource::File("/path/to/template.html".to_string());
	/// assert!(source.is_file());
	///
	/// let source = TemplateSource::Static("user.html");
	/// assert!(!source.is_file());
	/// ```
	pub fn is_file(&self) -> bool {
		matches!(self, TemplateSource::File(_))
	}

	/// Gets the template content or path as a string reference
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::strategy::TemplateSource;
	///
	/// let source = TemplateSource::Static("user.html");
	/// assert_eq!(source.as_str(), "user.html");
	///
	/// let source = TemplateSource::Dynamic("<h1>Title</h1>".to_string());
	/// assert_eq!(source.as_str(), "<h1>Title</h1>");
	/// ```
	pub fn as_str(&self) -> &str {
		match self {
			TemplateSource::Static(s) => s,
			TemplateSource::Dynamic(s) => s,
			TemplateSource::File(s) => s,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_strategy_selection_static() {
		let source = TemplateSource::Static("user.html");
		let strategy = TemplateStrategySelector::select(&source);
		assert_eq!(strategy, TemplateStrategy::CompileTime);
	}

	#[test]
	fn test_strategy_selection_dynamic() {
		let source = TemplateSource::Dynamic("<h1>{{ title }}</h1>".to_string());
		let strategy = TemplateStrategySelector::select(&source);
		assert_eq!(strategy, TemplateStrategy::Runtime);
	}

	#[test]
	fn test_strategy_selection_file_tera() {
		let source = TemplateSource::File("template.tera".to_string());
		let strategy = TemplateStrategySelector::select(&source);
		assert_eq!(strategy, TemplateStrategy::CompileTime);
	}

	#[test]
	fn test_strategy_selection_file_jinja() {
		let source = TemplateSource::File("template.jinja".to_string());
		let strategy = TemplateStrategySelector::select(&source);
		assert_eq!(strategy, TemplateStrategy::CompileTime);
	}

	#[test]
	fn test_strategy_selection_file_tpl() {
		let source = TemplateSource::File("template.tpl".to_string());
		let strategy = TemplateStrategySelector::select(&source);
		assert_eq!(strategy, TemplateStrategy::CompileTime);
	}

	#[test]
	fn test_strategy_selection_file_regular() {
		let source = TemplateSource::File("template.html".to_string());
		let strategy = TemplateStrategySelector::select(&source);
		assert_eq!(strategy, TemplateStrategy::Runtime);
	}

	#[test]
	fn test_recommend_for_use_case_view() {
		let strategy = TemplateStrategySelector::recommend_for_use_case("view template");
		assert_eq!(strategy, TemplateStrategy::CompileTime);
	}

	#[test]
	fn test_recommend_for_use_case_email() {
		let strategy = TemplateStrategySelector::recommend_for_use_case("email template");
		assert_eq!(strategy, TemplateStrategy::CompileTime);
	}

	#[test]
	fn test_recommend_for_use_case_user() {
		let strategy = TemplateStrategySelector::recommend_for_use_case("user template");
		assert_eq!(strategy, TemplateStrategy::Runtime);
	}

	#[test]
	fn test_recommend_for_use_case_config() {
		let strategy = TemplateStrategySelector::recommend_for_use_case("config template");
		assert_eq!(strategy, TemplateStrategy::Runtime);
	}

	#[test]
	fn test_recommend_for_use_case_unknown() {
		let strategy = TemplateStrategySelector::recommend_for_use_case("unknown");
		assert_eq!(strategy, TemplateStrategy::Runtime);
	}

	#[test]
	fn test_template_source_is_static() {
		let source = TemplateSource::Static("user.html");
		assert!(source.is_static());
		assert!(!source.is_dynamic());
		assert!(!source.is_file());
	}

	#[test]
	fn test_template_source_is_dynamic() {
		let source = TemplateSource::Dynamic("<h1>{{ title }}</h1>".to_string());
		assert!(!source.is_static());
		assert!(source.is_dynamic());
		assert!(!source.is_file());
	}

	#[test]
	fn test_template_source_is_file() {
		let source = TemplateSource::File("/path/to/template.html".to_string());
		assert!(!source.is_static());
		assert!(!source.is_dynamic());
		assert!(source.is_file());
	}

	#[test]
	fn test_template_source_as_str_static() {
		let source = TemplateSource::Static("user.html");
		assert_eq!(source.as_str(), "user.html");
	}

	#[test]
	fn test_template_source_as_str_dynamic() {
		let content = "<h1>{{ title }}</h1>".to_string();
		let source = TemplateSource::Dynamic(content.clone());
		assert_eq!(source.as_str(), content);
	}

	#[test]
	fn test_template_source_as_str_file() {
		let path = "/path/to/template.html".to_string();
		let source = TemplateSource::File(path.clone());
		assert_eq!(source.as_str(), path);
	}

	#[test]
	fn test_template_strategy_equality() {
		assert_eq!(TemplateStrategy::CompileTime, TemplateStrategy::CompileTime);
		assert_eq!(TemplateStrategy::Runtime, TemplateStrategy::Runtime);
		assert_ne!(TemplateStrategy::CompileTime, TemplateStrategy::Runtime);
	}

	#[test]
	fn test_template_source_equality() {
		let source1 = TemplateSource::Static("user.html");
		let source2 = TemplateSource::Static("user.html");
		assert_eq!(source1, source2);

		let source3 = TemplateSource::Dynamic("template".to_string());
		let source4 = TemplateSource::Dynamic("template".to_string());
		assert_eq!(source3, source4);
	}

	#[test]
	fn test_template_source_clone() {
		let source = TemplateSource::Dynamic("template".to_string());
		let cloned = source.clone();
		assert_eq!(source, cloned);
	}

	#[test]
	fn test_file_extension_edge_cases() {
		// Multiple dots in filename
		let source = TemplateSource::File("my.template.tera".to_string());
		let strategy = TemplateStrategySelector::select(&source);
		assert_eq!(strategy, TemplateStrategy::CompileTime);

		// No extension
		let source = TemplateSource::File("template".to_string());
		let strategy = TemplateStrategySelector::select(&source);
		assert_eq!(strategy, TemplateStrategy::Runtime);

		// Path with directory
		let source = TemplateSource::File("/templates/user.jinja".to_string());
		let strategy = TemplateStrategySelector::select(&source);
		assert_eq!(strategy, TemplateStrategy::CompileTime);
	}
}
