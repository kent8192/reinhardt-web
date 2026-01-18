//! ContentType cleanup management
//!
//! This module provides utilities for cleaning up stale or orphaned content types,
//! similar to Django's `remove_stale_contenttypes` management command.
//!
//! # Example
//!
//! ```rust
//! use reinhardt_db::contenttypes::cleanup::{ContentTypeCleanupManager, CleanupResult};
//! use reinhardt_db::contenttypes::{ContentType, ContentTypeRegistry};
//!
//! let registry = ContentTypeRegistry::new();
//! registry.register(ContentType::new("blog", "article"));
//! registry.register(ContentType::new("blog", "comment"));
//!
//! let mut manager = ContentTypeCleanupManager::new();
//! manager.mark_as_active("blog", "article");
//!
//! // "blog.comment" is now considered stale since it wasn't marked as active
//! let stale = manager.find_stale_content_types(&registry);
//! assert_eq!(stale.len(), 1);
//! ```

use crate::{ContentType, ContentTypeRegistry};
use std::collections::HashSet;

/// Result of a cleanup operation
#[derive(Debug, Clone, Default)]
pub struct CleanupResult {
	/// Content types that were removed
	pub removed: Vec<ContentType>,
	/// Content types that were kept (active)
	pub kept: Vec<ContentType>,
	/// Errors encountered during cleanup
	pub errors: Vec<String>,
}

impl CleanupResult {
	/// Creates a new empty cleanup result
	#[must_use]
	pub fn new() -> Self {
		Self::default()
	}

	/// Returns true if no content types were removed
	#[must_use]
	pub fn is_empty(&self) -> bool {
		self.removed.is_empty()
	}

	/// Returns the total number of content types processed
	#[must_use]
	pub fn total_processed(&self) -> usize {
		self.removed.len() + self.kept.len()
	}

	/// Returns true if there were any errors
	#[must_use]
	pub fn has_errors(&self) -> bool {
		!self.errors.is_empty()
	}
}

/// Manages cleanup of stale content types
///
/// This manager tracks which content types are actively in use and can identify
/// stale content types that should be removed from the registry.
#[derive(Debug, Clone, Default)]
pub struct ContentTypeCleanupManager {
	/// Set of qualified names (app_label.model) that are considered active
	active_types: HashSet<String>,
	/// Callbacks to invoke when a content type is removed (reserved for future extension)
	_on_remove_callbacks: Vec<String>,
}

impl ContentTypeCleanupManager {
	/// Creates a new cleanup manager
	#[must_use]
	pub fn new() -> Self {
		Self::default()
	}

	/// Marks a content type as active (in use)
	pub fn mark_as_active(&mut self, app_label: &str, model: &str) {
		self.active_types.insert(format!("{}.{}", app_label, model));
	}

	/// Marks a content type as active using a ContentType reference
	pub fn mark_content_type_active(&mut self, content_type: &ContentType) {
		self.mark_as_active(&content_type.app_label, &content_type.model);
	}

	/// Marks multiple content types as active
	pub fn mark_all_active(&mut self, content_types: &[ContentType]) {
		for ct in content_types {
			self.mark_content_type_active(ct);
		}
	}

	/// Checks if a content type is marked as active
	#[must_use]
	pub fn is_active(&self, app_label: &str, model: &str) -> bool {
		self.active_types
			.contains(&format!("{}.{}", app_label, model))
	}

	/// Checks if a ContentType is marked as active
	#[must_use]
	pub fn is_content_type_active(&self, content_type: &ContentType) -> bool {
		self.is_active(&content_type.app_label, &content_type.model)
	}

	/// Returns the number of active content types
	#[must_use]
	pub fn active_count(&self) -> usize {
		self.active_types.len()
	}

	/// Clears all active markers
	pub fn clear_active(&mut self) {
		self.active_types.clear();
	}

	/// Finds content types in the registry that are not marked as active
	#[must_use]
	pub fn find_stale_content_types(&self, registry: &ContentTypeRegistry) -> Vec<ContentType> {
		registry
			.all()
			.into_iter()
			.filter(|ct| !self.is_content_type_active(ct))
			.collect()
	}

	/// Finds content types for a specific app that are not marked as active
	#[must_use]
	pub fn find_stale_for_app(
		&self,
		registry: &ContentTypeRegistry,
		app_label: &str,
	) -> Vec<ContentType> {
		registry
			.all()
			.into_iter()
			.filter(|ct| ct.app_label == app_label && !self.is_content_type_active(ct))
			.collect()
	}

	/// Performs a dry run of cleanup, returning what would be removed
	#[must_use]
	pub fn dry_run(&self, registry: &ContentTypeRegistry) -> CleanupResult {
		let mut result = CleanupResult::new();

		for ct in registry.all() {
			if self.is_content_type_active(&ct) {
				result.kept.push(ct);
			} else {
				result.removed.push(ct);
			}
		}

		result
	}

	/// Performs cleanup by removing stale content types from the registry
	///
	/// Note: This modifies the registry by clearing it and re-adding only active types.
	/// In a real application, you would typically want to also update the database.
	pub fn cleanup(&self, registry: &ContentTypeRegistry) -> CleanupResult {
		let mut result = CleanupResult::new();

		// Collect all content types
		let all_types: Vec<ContentType> = registry.all();

		// Separate into kept and removed
		for ct in all_types {
			if self.is_content_type_active(&ct) {
				result.kept.push(ct);
			} else {
				result.removed.push(ct);
			}
		}

		// Clear and re-register only active types
		registry.clear();
		for ct in &result.kept {
			registry.register(ct.clone());
		}

		result
	}

	/// Removes a specific content type from the registry
	pub fn remove_content_type(
		&self,
		registry: &ContentTypeRegistry,
		app_label: &str,
		model: &str,
	) -> Option<ContentType> {
		let ct = registry.get(app_label, model)?;

		// Get all types except the one to remove
		let remaining: Vec<ContentType> = registry
			.all()
			.into_iter()
			.filter(|c| !(c.app_label == app_label && c.model == model))
			.collect();

		// Clear and re-register
		registry.clear();
		for c in remaining {
			registry.register(c);
		}

		Some(ct)
	}
}

/// Handles cleanup when a model is unregistered from the system
///
/// This is typically called when an app is uninstalled or a model is removed.
pub fn on_model_unregistered(
	registry: &ContentTypeRegistry,
	app_label: &str,
	model: &str,
) -> Option<ContentType> {
	let manager = ContentTypeCleanupManager::new();
	manager.remove_content_type(registry, app_label, model)
}

/// Handles cleanup when an entire app is unregistered
///
/// This removes all content types for the specified app.
pub fn on_app_unregistered(registry: &ContentTypeRegistry, app_label: &str) -> Vec<ContentType> {
	let types_to_remove: Vec<ContentType> = registry
		.all()
		.into_iter()
		.filter(|ct| ct.app_label == app_label)
		.collect();

	// Get remaining types
	let remaining: Vec<ContentType> = registry
		.all()
		.into_iter()
		.filter(|ct| ct.app_label != app_label)
		.collect();

	// Clear and re-register remaining
	registry.clear();
	for ct in remaining {
		registry.register(ct);
	}

	types_to_remove
}

/// Statistics about the cleanup operation
#[derive(Debug, Clone, Default)]
pub struct CleanupStats {
	/// Number of content types before cleanup
	pub before_count: usize,
	/// Number of content types after cleanup
	pub after_count: usize,
	/// Number of content types removed
	pub removed_count: usize,
	/// Apps affected by the cleanup
	pub affected_apps: Vec<String>,
}

impl CleanupStats {
	/// Creates new stats from a cleanup result and registry
	#[must_use]
	pub fn from_result(result: &CleanupResult) -> Self {
		let mut affected_apps: HashSet<String> = HashSet::new();
		for ct in &result.removed {
			affected_apps.insert(ct.app_label.clone());
		}

		Self {
			before_count: result.removed.len() + result.kept.len(),
			after_count: result.kept.len(),
			removed_count: result.removed.len(),
			affected_apps: affected_apps.into_iter().collect(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_cleanup_manager_new() {
		let manager = ContentTypeCleanupManager::new();
		assert_eq!(manager.active_count(), 0);
	}

	#[test]
	fn test_mark_as_active() {
		let mut manager = ContentTypeCleanupManager::new();

		manager.mark_as_active("blog", "article");
		assert!(manager.is_active("blog", "article"));
		assert!(!manager.is_active("blog", "comment"));
		assert_eq!(manager.active_count(), 1);
	}

	#[test]
	fn test_mark_content_type_active() {
		let mut manager = ContentTypeCleanupManager::new();
		let ct = ContentType::new("auth", "user");

		manager.mark_content_type_active(&ct);
		assert!(manager.is_content_type_active(&ct));
	}

	#[test]
	fn test_mark_all_active() {
		let mut manager = ContentTypeCleanupManager::new();
		let types = vec![
			ContentType::new("blog", "article"),
			ContentType::new("blog", "comment"),
			ContentType::new("auth", "user"),
		];

		manager.mark_all_active(&types);
		assert_eq!(manager.active_count(), 3);
	}

	#[test]
	fn test_clear_active() {
		let mut manager = ContentTypeCleanupManager::new();
		manager.mark_as_active("blog", "article");
		manager.mark_as_active("auth", "user");

		assert_eq!(manager.active_count(), 2);

		manager.clear_active();
		assert_eq!(manager.active_count(), 0);
	}

	#[test]
	fn test_find_stale_content_types() {
		let registry = ContentTypeRegistry::new();
		registry.register(ContentType::new("blog", "article"));
		registry.register(ContentType::new("blog", "comment"));
		registry.register(ContentType::new("auth", "user"));

		let mut manager = ContentTypeCleanupManager::new();
		manager.mark_as_active("blog", "article");
		manager.mark_as_active("auth", "user");

		let stale = manager.find_stale_content_types(&registry);

		assert_eq!(stale.len(), 1);
		assert_eq!(stale[0].model, "comment");
	}

	#[test]
	fn test_find_stale_for_app() {
		let registry = ContentTypeRegistry::new();
		registry.register(ContentType::new("blog", "article"));
		registry.register(ContentType::new("blog", "comment"));
		registry.register(ContentType::new("blog", "tag"));
		registry.register(ContentType::new("auth", "user"));

		let mut manager = ContentTypeCleanupManager::new();
		manager.mark_as_active("blog", "article");
		manager.mark_as_active("auth", "user");

		let stale = manager.find_stale_for_app(&registry, "blog");

		assert_eq!(stale.len(), 2);
	}

	#[test]
	fn test_dry_run() {
		let registry = ContentTypeRegistry::new();
		registry.register(ContentType::new("blog", "article"));
		registry.register(ContentType::new("blog", "comment"));

		let mut manager = ContentTypeCleanupManager::new();
		manager.mark_as_active("blog", "article");

		let result = manager.dry_run(&registry);

		assert_eq!(result.kept.len(), 1);
		assert_eq!(result.removed.len(), 1);
		assert_eq!(result.kept[0].model, "article");
		assert_eq!(result.removed[0].model, "comment");

		// Verify registry unchanged after dry run
		assert!(registry.get("blog", "comment").is_some());
	}

	#[test]
	fn test_cleanup() {
		let registry = ContentTypeRegistry::new();
		registry.register(ContentType::new("blog", "article"));
		registry.register(ContentType::new("blog", "comment"));
		registry.register(ContentType::new("auth", "user"));

		let mut manager = ContentTypeCleanupManager::new();
		manager.mark_as_active("blog", "article");
		manager.mark_as_active("auth", "user");

		let result = manager.cleanup(&registry);

		assert_eq!(result.kept.len(), 2);
		assert_eq!(result.removed.len(), 1);

		// Verify registry was updated
		assert!(registry.get("blog", "article").is_some());
		assert!(registry.get("auth", "user").is_some());
		assert!(registry.get("blog", "comment").is_none());
	}

	#[test]
	fn test_remove_content_type() {
		let registry = ContentTypeRegistry::new();
		registry.register(ContentType::new("blog", "article"));
		registry.register(ContentType::new("blog", "comment"));

		let manager = ContentTypeCleanupManager::new();
		let removed = manager.remove_content_type(&registry, "blog", "comment");

		assert!(removed.is_some());
		assert_eq!(removed.unwrap().model, "comment");
		assert!(registry.get("blog", "comment").is_none());
		assert!(registry.get("blog", "article").is_some());
	}

	#[test]
	fn test_remove_content_type_not_found() {
		let registry = ContentTypeRegistry::new();
		registry.register(ContentType::new("blog", "article"));

		let manager = ContentTypeCleanupManager::new();
		let removed = manager.remove_content_type(&registry, "blog", "nonexistent");

		assert!(removed.is_none());
	}

	#[test]
	fn test_on_model_unregistered() {
		let registry = ContentTypeRegistry::new();
		registry.register(ContentType::new("blog", "article"));
		registry.register(ContentType::new("blog", "comment"));

		let removed = on_model_unregistered(&registry, "blog", "article");

		assert!(removed.is_some());
		assert!(registry.get("blog", "article").is_none());
		assert!(registry.get("blog", "comment").is_some());
	}

	#[test]
	fn test_on_app_unregistered() {
		let registry = ContentTypeRegistry::new();
		registry.register(ContentType::new("blog", "article"));
		registry.register(ContentType::new("blog", "comment"));
		registry.register(ContentType::new("auth", "user"));

		let removed = on_app_unregistered(&registry, "blog");

		assert_eq!(removed.len(), 2);
		assert!(registry.get("blog", "article").is_none());
		assert!(registry.get("blog", "comment").is_none());
		assert!(registry.get("auth", "user").is_some());
	}

	#[test]
	fn test_cleanup_result() {
		let mut result = CleanupResult::new();

		assert!(result.is_empty());
		assert_eq!(result.total_processed(), 0);
		assert!(!result.has_errors());

		result.removed.push(ContentType::new("blog", "article"));
		result.kept.push(ContentType::new("auth", "user"));

		assert!(!result.is_empty());
		assert_eq!(result.total_processed(), 2);
	}

	#[test]
	fn test_cleanup_stats() {
		let mut result = CleanupResult::new();
		result.removed.push(ContentType::new("blog", "article"));
		result.removed.push(ContentType::new("blog", "comment"));
		result.kept.push(ContentType::new("auth", "user"));

		let stats = CleanupStats::from_result(&result);

		assert_eq!(stats.before_count, 3);
		assert_eq!(stats.after_count, 1);
		assert_eq!(stats.removed_count, 2);
		assert!(stats.affected_apps.contains(&"blog".to_string()));
	}
}
