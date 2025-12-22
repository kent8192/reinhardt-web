//! Shortcut functions for common ContentType operations
//!
//! This module provides convenience functions that simplify common patterns
//! when working with content types, similar to Django's contenttypes shortcuts.
//!
//! # Example
//!
//! ```rust
//! use reinhardt_contenttypes::shortcuts::{get_content_type_for_model, format_admin_url};
//! use reinhardt_contenttypes::{ContentType, ContentTypeRegistry, GenericRelatable};
//!
//! // Get or create a content type for a model
//! let mut registry = ContentTypeRegistry::new();
//! let ct = get_content_type_for_model(&mut registry, "blog", "article");
//!
//! // Format admin URL for a model
//! let admin_url = format_admin_url(&ct, Some(42));
//! assert_eq!(admin_url, "/admin/blog/article/42/change/");
//! ```

use crate::{ContentType, ContentTypeRegistry, GenericRelatable};

/// Gets or creates a ContentType for the given app_label and model name.
///
/// This is a convenience wrapper around `ContentTypeRegistry::get_or_create`.
///
/// # Example
///
/// ```rust
/// use reinhardt_contenttypes::shortcuts::get_content_type_for_model;
/// use reinhardt_contenttypes::ContentTypeRegistry;
///
/// let mut registry = ContentTypeRegistry::new();
/// let ct = get_content_type_for_model(&mut registry, "auth", "user");
/// assert_eq!(ct.app_label, "auth");
/// assert_eq!(ct.model, "user");
/// ```
pub fn get_content_type_for_model(
	registry: &mut ContentTypeRegistry,
	app_label: &str,
	model: &str,
) -> ContentType {
	registry.get_or_create(app_label, model)
}

/// Gets the ContentType for an object that implements GenericRelatable.
///
/// # Example
///
/// ```rust
/// use reinhardt_contenttypes::shortcuts::get_content_type_for_object;
/// use reinhardt_contenttypes::{ContentType, GenericRelatable};
///
/// struct Article { id: i64 }
///
/// impl GenericRelatable for Article {
///     fn get_content_type() -> ContentType {
///         ContentType::new("blog", "article")
///     }
///     fn get_object_id(&self) -> i64 {
///         self.id
///     }
/// }
///
/// let article = Article { id: 42 };
/// let ct = get_content_type_for_object(&article);
/// assert_eq!(ct.app_label, "blog");
/// ```
pub fn get_content_type_for_object<T: GenericRelatable>(_obj: &T) -> ContentType {
	T::get_content_type()
}

/// Formats an admin URL for a given ContentType.
///
/// This generates Django-style admin URLs in the format:
/// `/admin/{app_label}/{model}/{object_id}/change/`
///
/// If no object_id is provided, returns the list view URL:
/// `/admin/{app_label}/{model}/`
///
/// # Example
///
/// ```rust
/// use reinhardt_contenttypes::shortcuts::format_admin_url;
/// use reinhardt_contenttypes::ContentType;
///
/// let ct = ContentType::new("blog", "article");
///
/// // Change view
/// assert_eq!(format_admin_url(&ct, Some(42)), "/admin/blog/article/42/change/");
///
/// // List view
/// assert_eq!(format_admin_url(&ct, None), "/admin/blog/article/");
/// ```
pub fn format_admin_url(content_type: &ContentType, object_id: Option<i64>) -> String {
	match object_id {
		Some(id) => format!(
			"/admin/{}/{}/{}/change/",
			content_type.app_label, content_type.model, id
		),
		None => format!("/admin/{}/{}/", content_type.app_label, content_type.model),
	}
}

/// Formats an admin URL for adding a new object.
///
/// Returns a URL in the format: `/admin/{app_label}/{model}/add/`
///
/// # Example
///
/// ```rust
/// use reinhardt_contenttypes::shortcuts::format_admin_add_url;
/// use reinhardt_contenttypes::ContentType;
///
/// let ct = ContentType::new("blog", "article");
/// assert_eq!(format_admin_add_url(&ct), "/admin/blog/article/add/");
/// ```
pub fn format_admin_add_url(content_type: &ContentType) -> String {
	format!(
		"/admin/{}/{}/add/",
		content_type.app_label, content_type.model
	)
}

/// Formats an admin URL for deleting an object.
///
/// Returns a URL in the format: `/admin/{app_label}/{model}/{object_id}/delete/`
///
/// # Example
///
/// ```rust
/// use reinhardt_contenttypes::shortcuts::format_admin_delete_url;
/// use reinhardt_contenttypes::ContentType;
///
/// let ct = ContentType::new("blog", "article");
/// assert_eq!(format_admin_delete_url(&ct, 42), "/admin/blog/article/42/delete/");
/// ```
pub fn format_admin_delete_url(content_type: &ContentType, object_id: i64) -> String {
	format!(
		"/admin/{}/{}/{}/delete/",
		content_type.app_label, content_type.model, object_id
	)
}

/// Formats an admin URL for viewing history of an object.
///
/// Returns a URL in the format: `/admin/{app_label}/{model}/{object_id}/history/`
///
/// # Example
///
/// ```rust
/// use reinhardt_contenttypes::shortcuts::format_admin_history_url;
/// use reinhardt_contenttypes::ContentType;
///
/// let ct = ContentType::new("blog", "article");
/// assert_eq!(format_admin_history_url(&ct, 42), "/admin/blog/article/42/history/");
/// ```
pub fn format_admin_history_url(content_type: &ContentType, object_id: i64) -> String {
	format!(
		"/admin/{}/{}/{}/history/",
		content_type.app_label, content_type.model, object_id
	)
}

/// Checks if a ContentType matches the given app_label and model.
///
/// # Example
///
/// ```rust
/// use reinhardt_contenttypes::shortcuts::matches_model;
/// use reinhardt_contenttypes::ContentType;
///
/// let ct = ContentType::new("blog", "article");
/// assert!(matches_model(&ct, "blog", "article"));
/// assert!(!matches_model(&ct, "blog", "comment"));
/// ```
pub fn matches_model(content_type: &ContentType, app_label: &str, model: &str) -> bool {
	content_type.app_label == app_label && content_type.model == model
}

/// Parses a qualified name (app_label.model) into its components.
///
/// Returns `None` if the format is invalid.
///
/// # Example
///
/// ```rust
/// use reinhardt_contenttypes::shortcuts::parse_qualified_name;
///
/// assert_eq!(parse_qualified_name("blog.article"), Some(("blog".to_string(), "article".to_string())));
/// assert_eq!(parse_qualified_name("invalid"), None);
/// ```
pub fn parse_qualified_name(qualified_name: &str) -> Option<(String, String)> {
	let parts: Vec<&str> = qualified_name.splitn(2, '.').collect();
	if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
		Some((parts[0].to_string(), parts[1].to_string()))
	} else {
		None
	}
}

/// Finds a ContentType by qualified name in the registry.
///
/// # Example
///
/// ```rust
/// use reinhardt_contenttypes::shortcuts::get_by_qualified_name;
/// use reinhardt_contenttypes::{ContentType, ContentTypeRegistry};
///
/// let registry = ContentTypeRegistry::new();
/// registry.register(ContentType::new("blog", "article"));
///
/// let ct = get_by_qualified_name(&registry, "blog.article");
/// assert!(ct.is_some());
///
/// let not_found = get_by_qualified_name(&registry, "blog.comment");
/// assert!(not_found.is_none());
/// ```
pub fn get_by_qualified_name(
	registry: &ContentTypeRegistry,
	qualified_name: &str,
) -> Option<ContentType> {
	parse_qualified_name(qualified_name)
		.and_then(|(app_label, model)| registry.get(&app_label, &model))
}

/// Groups content types by app_label.
///
/// # Example
///
/// ```rust
/// use reinhardt_contenttypes::shortcuts::group_by_app;
/// use reinhardt_contenttypes::{ContentType, ContentTypeRegistry};
///
/// let registry = ContentTypeRegistry::new();
/// registry.register(ContentType::new("blog", "article"));
/// registry.register(ContentType::new("blog", "comment"));
/// registry.register(ContentType::new("auth", "user"));
///
/// let grouped = group_by_app(&registry);
/// assert_eq!(grouped.get("blog").map(|v| v.len()), Some(2));
/// assert_eq!(grouped.get("auth").map(|v| v.len()), Some(1));
/// ```
pub fn group_by_app(
	registry: &ContentTypeRegistry,
) -> std::collections::HashMap<String, Vec<ContentType>> {
	let mut grouped: std::collections::HashMap<String, Vec<ContentType>> =
		std::collections::HashMap::new();
	for ct in registry.all() {
		grouped.entry(ct.app_label.clone()).or_default().push(ct);
	}
	grouped
}

/// Lists all app_labels that have registered content types.
///
/// # Example
///
/// ```rust
/// use reinhardt_contenttypes::shortcuts::list_app_labels;
/// use reinhardt_contenttypes::{ContentType, ContentTypeRegistry};
///
/// let registry = ContentTypeRegistry::new();
/// registry.register(ContentType::new("blog", "article"));
/// registry.register(ContentType::new("auth", "user"));
///
/// let apps = list_app_labels(&registry);
/// assert!(apps.contains(&"blog".to_string()));
/// assert!(apps.contains(&"auth".to_string()));
/// ```
pub fn list_app_labels(registry: &ContentTypeRegistry) -> Vec<String> {
	let mut apps: Vec<String> = registry
		.all()
		.into_iter()
		.map(|ct| ct.app_label)
		.collect::<std::collections::HashSet<_>>()
		.into_iter()
		.collect();
	apps.sort();
	apps
}

/// Lists all models for a given app_label.
///
/// # Example
///
/// ```rust
/// use reinhardt_contenttypes::shortcuts::list_models_for_app;
/// use reinhardt_contenttypes::{ContentType, ContentTypeRegistry};
///
/// let registry = ContentTypeRegistry::new();
/// registry.register(ContentType::new("blog", "article"));
/// registry.register(ContentType::new("blog", "comment"));
///
/// let models = list_models_for_app(&registry, "blog");
/// assert!(models.contains(&"article".to_string()));
/// assert!(models.contains(&"comment".to_string()));
/// ```
pub fn list_models_for_app(registry: &ContentTypeRegistry, app_label: &str) -> Vec<String> {
	let mut models: Vec<String> = registry
		.all()
		.into_iter()
		.filter(|ct| ct.app_label == app_label)
		.map(|ct| ct.model)
		.collect();
	models.sort();
	models
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_get_content_type_for_model() {
		let mut registry = ContentTypeRegistry::new();
		let ct = get_content_type_for_model(&mut registry, "auth", "user");

		assert_eq!(ct.app_label, "auth");
		assert_eq!(ct.model, "user");

		// Should return the same content type
		let ct2 = get_content_type_for_model(&mut registry, "auth", "user");
		assert_eq!(ct.id, ct2.id);
	}

	#[test]
	fn test_get_content_type_for_object() {
		struct TestModel {
			id: i64,
		}

		impl GenericRelatable for TestModel {
			fn get_content_type() -> ContentType {
				ContentType::new("test", "testmodel")
			}
			fn get_object_id(&self) -> i64 {
				self.id
			}
		}

		let obj = TestModel { id: 123 };
		let ct = get_content_type_for_object(&obj);

		assert_eq!(ct.app_label, "test");
		assert_eq!(ct.model, "testmodel");
	}

	#[test]
	fn test_format_admin_url() {
		let ct = ContentType::new("blog", "article");

		// With object_id
		assert_eq!(
			format_admin_url(&ct, Some(42)),
			"/admin/blog/article/42/change/"
		);

		// Without object_id (list view)
		assert_eq!(format_admin_url(&ct, None), "/admin/blog/article/");
	}

	#[test]
	fn test_format_admin_add_url() {
		let ct = ContentType::new("blog", "article");
		assert_eq!(format_admin_add_url(&ct), "/admin/blog/article/add/");
	}

	#[test]
	fn test_format_admin_delete_url() {
		let ct = ContentType::new("blog", "article");
		assert_eq!(
			format_admin_delete_url(&ct, 42),
			"/admin/blog/article/42/delete/"
		);
	}

	#[test]
	fn test_format_admin_history_url() {
		let ct = ContentType::new("blog", "article");
		assert_eq!(
			format_admin_history_url(&ct, 42),
			"/admin/blog/article/42/history/"
		);
	}

	#[test]
	fn test_matches_model() {
		let ct = ContentType::new("blog", "article");

		assert!(matches_model(&ct, "blog", "article"));
		assert!(!matches_model(&ct, "blog", "comment"));
		assert!(!matches_model(&ct, "auth", "article"));
	}

	#[test]
	fn test_parse_qualified_name() {
		assert_eq!(
			parse_qualified_name("blog.article"),
			Some(("blog".to_string(), "article".to_string()))
		);
		assert_eq!(
			parse_qualified_name("auth.user"),
			Some(("auth".to_string(), "user".to_string()))
		);

		// Invalid formats
		assert_eq!(parse_qualified_name("invalid"), None);
		assert_eq!(parse_qualified_name(""), None);
		assert_eq!(parse_qualified_name(".model"), None);
		assert_eq!(parse_qualified_name("app."), None);
		assert_eq!(parse_qualified_name("."), None);
	}

	#[test]
	fn test_get_by_qualified_name() {
		let registry = ContentTypeRegistry::new();
		registry.register(ContentType::new("blog", "article"));

		let found = get_by_qualified_name(&registry, "blog.article");
		assert!(found.is_some());
		assert_eq!(found.unwrap().model, "article");

		let not_found = get_by_qualified_name(&registry, "blog.comment");
		assert!(not_found.is_none());
	}

	#[test]
	fn test_group_by_app() {
		let registry = ContentTypeRegistry::new();
		registry.register(ContentType::new("blog", "article"));
		registry.register(ContentType::new("blog", "comment"));
		registry.register(ContentType::new("auth", "user"));
		registry.register(ContentType::new("auth", "group"));

		let grouped = group_by_app(&registry);

		assert_eq!(grouped.get("blog").map(|v| v.len()), Some(2));
		assert_eq!(grouped.get("auth").map(|v| v.len()), Some(2));
	}

	#[test]
	fn test_list_app_labels() {
		let registry = ContentTypeRegistry::new();
		registry.register(ContentType::new("blog", "article"));
		registry.register(ContentType::new("auth", "user"));
		registry.register(ContentType::new("admin", "logentry"));

		let apps = list_app_labels(&registry);

		assert_eq!(apps.len(), 3);
		assert!(apps.contains(&"admin".to_string()));
		assert!(apps.contains(&"auth".to_string()));
		assert!(apps.contains(&"blog".to_string()));
	}

	#[test]
	fn test_list_models_for_app() {
		let registry = ContentTypeRegistry::new();
		registry.register(ContentType::new("blog", "article"));
		registry.register(ContentType::new("blog", "comment"));
		registry.register(ContentType::new("blog", "tag"));

		let models = list_models_for_app(&registry, "blog");

		assert_eq!(models.len(), 3);
		assert!(models.contains(&"article".to_string()));
		assert!(models.contains(&"comment".to_string()));
		assert!(models.contains(&"tag".to_string()));

		// Non-existent app
		let empty = list_models_for_app(&registry, "nonexistent");
		assert!(empty.is_empty());
	}
}
