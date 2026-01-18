//! ContentType permission utilities
//!
//! This module provides Django-style permission utilities for ContentTypes.
//! Permissions are formatted as "app_label.action_model" (e.g., "blog.add_article").
//!
//! # Examples
//!
//! ```
//! use reinhardt_db::contenttypes::permissions::{ContentTypePermission, PermissionAction};
//! use reinhardt_db::contenttypes::ContentType;
//!
//! let ct = ContentType::new("blog", "article");
//!
//! // Format a permission string
//! let perm = ContentTypePermission::format(&ct, PermissionAction::Add);
//! assert_eq!(perm, "blog.add_article");
//!
//! // Get default permissions for a content type
//! let perms = ContentTypePermission::default_permissions(&ct);
//! assert!(perms.contains(&"blog.view_article".to_string()));
//! assert!(perms.contains(&"blog.add_article".to_string()));
//! assert!(perms.contains(&"blog.change_article".to_string()));
//! assert!(perms.contains(&"blog.delete_article".to_string()));
//! ```

use crate::ContentType;
use std::str::FromStr;

/// Error type for parsing PermissionAction from string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsePermissionActionError {
	/// The invalid action string
	pub input: String,
}

impl std::fmt::Display for ParsePermissionActionError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Invalid permission action: '{}'", self.input)
	}
}

impl std::error::Error for ParsePermissionActionError {}

/// Permission actions for content types
///
/// Django defines four default permissions for each model:
/// - view: Read access
/// - add: Create new instances
/// - change: Modify existing instances
/// - delete: Remove instances
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PermissionAction {
	/// Permission to view/read instances
	View,
	/// Permission to add/create new instances
	Add,
	/// Permission to change/update existing instances
	Change,
	/// Permission to delete instances
	Delete,
}

impl PermissionAction {
	/// Returns the action as a lowercase string
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::contenttypes::permissions::PermissionAction;
	///
	/// assert_eq!(PermissionAction::View.as_str(), "view");
	/// assert_eq!(PermissionAction::Add.as_str(), "add");
	/// assert_eq!(PermissionAction::Change.as_str(), "change");
	/// assert_eq!(PermissionAction::Delete.as_str(), "delete");
	/// ```
	#[must_use]
	pub const fn as_str(&self) -> &'static str {
		match self {
			Self::View => "view",
			Self::Add => "add",
			Self::Change => "change",
			Self::Delete => "delete",
		}
	}

	/// Returns all standard permission actions
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::contenttypes::permissions::PermissionAction;
	///
	/// let actions = PermissionAction::all();
	/// assert_eq!(actions.len(), 4);
	/// ```
	#[must_use]
	pub const fn all() -> [Self; 4] {
		[Self::View, Self::Add, Self::Change, Self::Delete]
	}
}

impl FromStr for PermissionAction {
	type Err = ParsePermissionActionError;

	/// Parses an action string into a PermissionAction
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::contenttypes::permissions::PermissionAction;
	/// use std::str::FromStr;
	///
	/// assert_eq!(PermissionAction::from_str("view"), Ok(PermissionAction::View));
	/// assert_eq!(PermissionAction::from_str("add"), Ok(PermissionAction::Add));
	/// assert_eq!(PermissionAction::from_str("change"), Ok(PermissionAction::Change));
	/// assert_eq!(PermissionAction::from_str("delete"), Ok(PermissionAction::Delete));
	/// assert!(PermissionAction::from_str("unknown").is_err());
	///
	/// // Can also use .parse()
	/// assert_eq!("view".parse::<PermissionAction>(), Ok(PermissionAction::View));
	/// ```
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"view" => Ok(Self::View),
			"add" => Ok(Self::Add),
			"change" => Ok(Self::Change),
			"delete" => Ok(Self::Delete),
			_ => Err(ParsePermissionActionError {
				input: s.to_string(),
			}),
		}
	}
}

impl std::fmt::Display for PermissionAction {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.as_str())
	}
}

/// ContentType permission utilities
///
/// Provides methods for formatting and parsing Django-style permission strings.
pub struct ContentTypePermission;

impl ContentTypePermission {
	/// Formats a permission string in Django style: "app_label.action_model"
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::contenttypes::permissions::{ContentTypePermission, PermissionAction};
	/// use reinhardt_db::contenttypes::ContentType;
	///
	/// let ct = ContentType::new("auth", "user");
	///
	/// assert_eq!(
	///     ContentTypePermission::format(&ct, PermissionAction::View),
	///     "auth.view_user"
	/// );
	/// assert_eq!(
	///     ContentTypePermission::format(&ct, PermissionAction::Add),
	///     "auth.add_user"
	/// );
	/// ```
	#[must_use]
	pub fn format(ct: &ContentType, action: PermissionAction) -> String {
		format!("{}.{}_{}", ct.app_label, action.as_str(), ct.model)
	}

	/// Formats a permission string with a custom action name
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::contenttypes::permissions::ContentTypePermission;
	/// use reinhardt_db::contenttypes::ContentType;
	///
	/// let ct = ContentType::new("blog", "article");
	///
	/// assert_eq!(
	///     ContentTypePermission::format_custom(&ct, "publish"),
	///     "blog.publish_article"
	/// );
	/// ```
	#[must_use]
	pub fn format_custom(ct: &ContentType, action: &str) -> String {
		format!("{}.{}_{}", ct.app_label, action, ct.model)
	}

	/// Parses a permission string into its components
	///
	/// Returns `Some((app_label, action, model))` if the permission string is valid,
	/// or `None` if parsing fails.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::contenttypes::permissions::ContentTypePermission;
	///
	/// let parsed = ContentTypePermission::parse("blog.add_article");
	/// assert_eq!(parsed, Some(("blog".to_string(), "add".to_string(), "article".to_string())));
	///
	/// let parsed = ContentTypePermission::parse("auth.view_user");
	/// assert_eq!(parsed, Some(("auth".to_string(), "view".to_string(), "user".to_string())));
	///
	/// // Invalid format
	/// assert_eq!(ContentTypePermission::parse("invalid"), None);
	/// assert_eq!(ContentTypePermission::parse("app.invalid"), None);
	/// ```
	#[must_use]
	pub fn parse(permission: &str) -> Option<(String, String, String)> {
		let dot_pos = permission.find('.')?;
		let app_label = &permission[..dot_pos];
		let action_model = &permission[dot_pos + 1..];

		let underscore_pos = action_model.find('_')?;
		let action = &action_model[..underscore_pos];
		let model = &action_model[underscore_pos + 1..];

		if app_label.is_empty() || action.is_empty() || model.is_empty() {
			return None;
		}

		Some((app_label.to_string(), action.to_string(), model.to_string()))
	}

	/// Returns the four default permissions for a content type
	///
	/// Django creates these permissions automatically for every model:
	/// - `{app_label}.view_{model}`
	/// - `{app_label}.add_{model}`
	/// - `{app_label}.change_{model}`
	/// - `{app_label}.delete_{model}`
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::contenttypes::permissions::ContentTypePermission;
	/// use reinhardt_db::contenttypes::ContentType;
	///
	/// let ct = ContentType::new("blog", "article");
	/// let perms = ContentTypePermission::default_permissions(&ct);
	///
	/// assert_eq!(perms.len(), 4);
	/// assert!(perms.contains(&"blog.view_article".to_string()));
	/// assert!(perms.contains(&"blog.add_article".to_string()));
	/// assert!(perms.contains(&"blog.change_article".to_string()));
	/// assert!(perms.contains(&"blog.delete_article".to_string()));
	/// ```
	#[must_use]
	pub fn default_permissions(ct: &ContentType) -> Vec<String> {
		PermissionAction::all()
			.iter()
			.map(|action| Self::format(ct, *action))
			.collect()
	}

	/// Checks if a permission string matches a specific content type and action
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::contenttypes::permissions::{ContentTypePermission, PermissionAction};
	/// use reinhardt_db::contenttypes::ContentType;
	///
	/// let ct = ContentType::new("blog", "article");
	///
	/// assert!(ContentTypePermission::matches("blog.add_article", &ct, PermissionAction::Add));
	/// assert!(!ContentTypePermission::matches("blog.view_article", &ct, PermissionAction::Add));
	/// assert!(!ContentTypePermission::matches("auth.add_user", &ct, PermissionAction::Add));
	/// ```
	#[must_use]
	pub fn matches(permission: &str, ct: &ContentType, action: PermissionAction) -> bool {
		permission == Self::format(ct, action)
	}

	/// Extracts the ContentType from a permission string if the app_label and model are valid
	///
	/// Note: This creates a ContentType without an ID. For persisted content types,
	/// use the ContentTypeRegistry to look up the actual ContentType.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::contenttypes::permissions::ContentTypePermission;
	///
	/// let ct = ContentTypePermission::extract_content_type("blog.add_article");
	/// assert!(ct.is_some());
	/// let ct = ct.unwrap();
	/// assert_eq!(ct.app_label, "blog");
	/// assert_eq!(ct.model, "article");
	/// ```
	#[must_use]
	pub fn extract_content_type(permission: &str) -> Option<ContentType> {
		let (app_label, _, model) = Self::parse(permission)?;
		Some(ContentType::new(&app_label, &model))
	}

	/// Extracts the PermissionAction from a permission string
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::contenttypes::permissions::{ContentTypePermission, PermissionAction};
	///
	/// assert_eq!(
	///     ContentTypePermission::extract_action("blog.view_article"),
	///     Some(PermissionAction::View)
	/// );
	/// assert_eq!(
	///     ContentTypePermission::extract_action("blog.add_article"),
	///     Some(PermissionAction::Add)
	/// );
	/// assert_eq!(
	///     ContentTypePermission::extract_action("blog.custom_article"),
	///     None
	/// );
	/// ```
	#[must_use]
	pub fn extract_action(permission: &str) -> Option<PermissionAction> {
		let (_, action, _) = Self::parse(permission)?;
		action.parse().ok()
	}
}

/// Context for permission checking
///
/// Contains information about the current request/user that can be used
/// to make permission decisions.
#[derive(Debug, Clone, Default)]
pub struct PermissionContext<'a> {
	/// Username or identifier of the user
	pub username: Option<&'a str>,
	/// Whether the user is authenticated
	pub is_authenticated: bool,
	/// Whether the user is a staff member
	pub is_staff: bool,
	/// Whether the user is a superuser
	pub is_superuser: bool,
}

impl<'a> PermissionContext<'a> {
	/// Creates a new PermissionContext for an authenticated user
	#[must_use]
	pub fn authenticated(username: &'a str) -> Self {
		Self {
			username: Some(username),
			is_authenticated: true,
			is_staff: false,
			is_superuser: false,
		}
	}

	/// Creates a new PermissionContext for a staff user
	#[must_use]
	pub fn staff(username: &'a str) -> Self {
		Self {
			username: Some(username),
			is_authenticated: true,
			is_staff: true,
			is_superuser: false,
		}
	}

	/// Creates a new PermissionContext for a superuser
	#[must_use]
	pub fn superuser(username: &'a str) -> Self {
		Self {
			username: Some(username),
			is_authenticated: true,
			is_staff: true,
			is_superuser: true,
		}
	}

	/// Creates an anonymous (unauthenticated) context
	#[must_use]
	pub fn anonymous() -> Self {
		Self::default()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_permission_action_as_str() {
		assert_eq!(PermissionAction::View.as_str(), "view");
		assert_eq!(PermissionAction::Add.as_str(), "add");
		assert_eq!(PermissionAction::Change.as_str(), "change");
		assert_eq!(PermissionAction::Delete.as_str(), "delete");
	}

	#[test]
	fn test_permission_action_all() {
		let actions = PermissionAction::all();
		assert_eq!(actions.len(), 4);
		assert!(actions.contains(&PermissionAction::View));
		assert!(actions.contains(&PermissionAction::Add));
		assert!(actions.contains(&PermissionAction::Change));
		assert!(actions.contains(&PermissionAction::Delete));
	}

	#[test]
	fn test_permission_action_from_str() {
		assert_eq!(
			PermissionAction::from_str("view"),
			Ok(PermissionAction::View)
		);
		assert_eq!(
			PermissionAction::from_str("VIEW"),
			Ok(PermissionAction::View)
		);
		assert_eq!(PermissionAction::from_str("add"), Ok(PermissionAction::Add));
		assert_eq!(
			PermissionAction::from_str("change"),
			Ok(PermissionAction::Change)
		);
		assert_eq!(
			PermissionAction::from_str("delete"),
			Ok(PermissionAction::Delete)
		);
		assert!(PermissionAction::from_str("unknown").is_err());

		// Test using .parse()
		assert_eq!(
			"view".parse::<PermissionAction>(),
			Ok(PermissionAction::View)
		);
		assert!("invalid".parse::<PermissionAction>().is_err());
	}

	#[test]
	fn test_permission_action_display() {
		assert_eq!(format!("{}", PermissionAction::View), "view");
		assert_eq!(format!("{}", PermissionAction::Add), "add");
	}

	#[test]
	fn test_format_permission() {
		let ct = ContentType::new("blog", "article");

		assert_eq!(
			ContentTypePermission::format(&ct, PermissionAction::View),
			"blog.view_article"
		);
		assert_eq!(
			ContentTypePermission::format(&ct, PermissionAction::Add),
			"blog.add_article"
		);
		assert_eq!(
			ContentTypePermission::format(&ct, PermissionAction::Change),
			"blog.change_article"
		);
		assert_eq!(
			ContentTypePermission::format(&ct, PermissionAction::Delete),
			"blog.delete_article"
		);
	}

	#[test]
	fn test_format_custom_permission() {
		let ct = ContentType::new("blog", "article");

		assert_eq!(
			ContentTypePermission::format_custom(&ct, "publish"),
			"blog.publish_article"
		);
		assert_eq!(
			ContentTypePermission::format_custom(&ct, "archive"),
			"blog.archive_article"
		);
	}

	#[test]
	fn test_parse_permission() {
		let parsed = ContentTypePermission::parse("blog.add_article");
		assert_eq!(
			parsed,
			Some(("blog".to_string(), "add".to_string(), "article".to_string()))
		);

		let parsed = ContentTypePermission::parse("auth.view_user");
		assert_eq!(
			parsed,
			Some(("auth".to_string(), "view".to_string(), "user".to_string()))
		);

		// Edge cases
		assert_eq!(ContentTypePermission::parse("invalid"), None);
		assert_eq!(ContentTypePermission::parse("app.invalid"), None);
		assert_eq!(ContentTypePermission::parse(".add_model"), None);
		assert_eq!(ContentTypePermission::parse("app._model"), None);
		assert_eq!(ContentTypePermission::parse("app.action_"), None);
	}

	#[test]
	fn test_default_permissions() {
		let ct = ContentType::new("blog", "article");
		let perms = ContentTypePermission::default_permissions(&ct);

		assert_eq!(perms.len(), 4);
		assert!(perms.contains(&"blog.view_article".to_string()));
		assert!(perms.contains(&"blog.add_article".to_string()));
		assert!(perms.contains(&"blog.change_article".to_string()));
		assert!(perms.contains(&"blog.delete_article".to_string()));
	}

	#[test]
	fn test_matches_permission() {
		let ct = ContentType::new("blog", "article");

		assert!(ContentTypePermission::matches(
			"blog.add_article",
			&ct,
			PermissionAction::Add
		));
		assert!(ContentTypePermission::matches(
			"blog.view_article",
			&ct,
			PermissionAction::View
		));
		assert!(!ContentTypePermission::matches(
			"blog.view_article",
			&ct,
			PermissionAction::Add
		));
		assert!(!ContentTypePermission::matches(
			"auth.add_user",
			&ct,
			PermissionAction::Add
		));
	}

	#[test]
	fn test_extract_content_type() {
		let ct = ContentTypePermission::extract_content_type("blog.add_article");
		assert!(ct.is_some());
		let ct = ct.unwrap();
		assert_eq!(ct.app_label, "blog");
		assert_eq!(ct.model, "article");

		assert!(ContentTypePermission::extract_content_type("invalid").is_none());
	}

	#[test]
	fn test_extract_action() {
		assert_eq!(
			ContentTypePermission::extract_action("blog.view_article"),
			Some(PermissionAction::View)
		);
		assert_eq!(
			ContentTypePermission::extract_action("blog.add_article"),
			Some(PermissionAction::Add)
		);
		assert_eq!(
			ContentTypePermission::extract_action("blog.change_article"),
			Some(PermissionAction::Change)
		);
		assert_eq!(
			ContentTypePermission::extract_action("blog.delete_article"),
			Some(PermissionAction::Delete)
		);
		assert_eq!(
			ContentTypePermission::extract_action("blog.custom_article"),
			None
		);
	}

	#[test]
	fn test_permission_context_authenticated() {
		let ctx = PermissionContext::authenticated("alice");
		assert_eq!(ctx.username, Some("alice"));
		assert!(ctx.is_authenticated);
		assert!(!ctx.is_staff);
		assert!(!ctx.is_superuser);
	}

	#[test]
	fn test_permission_context_staff() {
		let ctx = PermissionContext::staff("bob");
		assert_eq!(ctx.username, Some("bob"));
		assert!(ctx.is_authenticated);
		assert!(ctx.is_staff);
		assert!(!ctx.is_superuser);
	}

	#[test]
	fn test_permission_context_superuser() {
		let ctx = PermissionContext::superuser("admin");
		assert_eq!(ctx.username, Some("admin"));
		assert!(ctx.is_authenticated);
		assert!(ctx.is_staff);
		assert!(ctx.is_superuser);
	}

	#[test]
	fn test_permission_context_anonymous() {
		let ctx = PermissionContext::anonymous();
		assert_eq!(ctx.username, None);
		assert!(!ctx.is_authenticated);
		assert!(!ctx.is_staff);
		assert!(!ctx.is_superuser);
	}

	#[test]
	fn test_permission_context_default() {
		let ctx = PermissionContext::default();
		assert_eq!(ctx.username, None);
		assert!(!ctx.is_authenticated);
	}
}
