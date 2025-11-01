//! Database routing for multi-database support
//!
//! This module provides functionality to route database operations to different databases
//! based on model names and operation types (read/write).

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Router for directing database operations to specific databases
///
/// # Examples
///
/// ```
/// use reinhardt_orm::database_routing::DatabaseRouter;
///
/// let router = DatabaseRouter::new("default")
///     .add_rule("User", "primary")
///     .add_rule("Log", "analytics");
///
/// assert_eq!(router.db_for_read("User"), "primary");
/// assert_eq!(router.db_for_write("Log"), "analytics");
/// assert_eq!(router.db_for_read("Unknown"), "default");
/// ```
#[derive(Debug, Clone)]
pub struct DatabaseRouter {
	rules: Arc<RwLock<HashMap<String, DatabaseRule>>>,
	default_db: String,
}

/// Represents routing rules for a specific model
#[derive(Debug, Clone)]
struct DatabaseRule {
	/// Database alias for read operations
	read_db: Option<String>,
	/// Database alias for write operations
	write_db: Option<String>,
}

impl DatabaseRouter {
	/// Creates a new database router with the specified default database
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::database_routing::DatabaseRouter;
	///
	/// let router = DatabaseRouter::new("default");
	/// assert_eq!(router.default_db(), "default");
	/// ```
	pub fn new(default_db: impl Into<String>) -> Self {
		Self {
			rules: Arc::new(RwLock::new(HashMap::new())),
			default_db: default_db.into(),
		}
	}

	/// Adds a routing rule for a model to use the same database for both reads and writes
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::database_routing::DatabaseRouter;
	///
	/// let router = DatabaseRouter::new("default")
	///     .add_rule("User", "users_db");
	///
	/// assert_eq!(router.db_for_read("User"), "users_db");
	/// assert_eq!(router.db_for_write("User"), "users_db");
	/// ```
	pub fn add_rule(self, model_name: impl Into<String>, db_alias: impl Into<String>) -> Self {
		let db_alias = db_alias.into();
		let rule = DatabaseRule {
			read_db: Some(db_alias.clone()),
			write_db: Some(db_alias),
		};
		self.rules.write().insert(model_name.into(), rule);
		self
	}

	/// Adds a routing rule with separate databases for read and write operations
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::database_routing::DatabaseRouter;
	///
	/// let router = DatabaseRouter::new("default")
	///     .add_read_write_rule("User", "replica", "primary");
	///
	/// assert_eq!(router.db_for_read("User"), "replica");
	/// assert_eq!(router.db_for_write("User"), "primary");
	/// ```
	pub fn add_read_write_rule(
		self,
		model_name: impl Into<String>,
		read_db: impl Into<String>,
		write_db: impl Into<String>,
	) -> Self {
		let rule = DatabaseRule {
			read_db: Some(read_db.into()),
			write_db: Some(write_db.into()),
		};
		self.rules.write().insert(model_name.into(), rule);
		self
	}

	/// Adds a routing rule for read operations only
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::database_routing::DatabaseRouter;
	///
	/// let router = DatabaseRouter::new("default")
	///     .add_read_rule("Analytics", "analytics_db");
	///
	/// assert_eq!(router.db_for_read("Analytics"), "analytics_db");
	/// assert_eq!(router.db_for_write("Analytics"), "default");
	/// ```
	pub fn add_read_rule(self, model_name: impl Into<String>, read_db: impl Into<String>) -> Self {
		let model_name = model_name.into();
		let mut rules = self.rules.write();

		let rule = rules.entry(model_name).or_insert_with(|| DatabaseRule {
			read_db: None,
			write_db: None,
		});

		rule.read_db = Some(read_db.into());
		drop(rules);
		self
	}

	/// Adds a routing rule for write operations only
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::database_routing::DatabaseRouter;
	///
	/// let router = DatabaseRouter::new("default")
	///     .add_write_rule("AuditLog", "audit_db");
	///
	/// assert_eq!(router.db_for_read("AuditLog"), "default");
	/// assert_eq!(router.db_for_write("AuditLog"), "audit_db");
	/// ```
	pub fn add_write_rule(
		self,
		model_name: impl Into<String>,
		write_db: impl Into<String>,
	) -> Self {
		let model_name = model_name.into();
		let mut rules = self.rules.write();

		let rule = rules.entry(model_name).or_insert_with(|| DatabaseRule {
			read_db: None,
			write_db: None,
		});

		rule.write_db = Some(write_db.into());
		drop(rules);
		self
	}

	/// Gets the database alias for read operations on the specified model
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::database_routing::DatabaseRouter;
	///
	/// let router = DatabaseRouter::new("default")
	///     .add_rule("User", "users_db");
	///
	/// assert_eq!(router.db_for_read("User"), "users_db");
	/// assert_eq!(router.db_for_read("Unknown"), "default");
	/// ```
	pub fn db_for_read(&self, model_name: &str) -> String {
		let rules = self.rules.read();
		rules
			.get(model_name)
			.and_then(|rule| rule.read_db.clone())
			.unwrap_or_else(|| self.default_db.clone())
	}

	/// Gets the database alias for write operations on the specified model
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::database_routing::DatabaseRouter;
	///
	/// let router = DatabaseRouter::new("default")
	///     .add_rule("User", "users_db");
	///
	/// assert_eq!(router.db_for_write("User"), "users_db");
	/// assert_eq!(router.db_for_write("Unknown"), "default");
	/// ```
	pub fn db_for_write(&self, model_name: &str) -> String {
		let rules = self.rules.read();
		rules
			.get(model_name)
			.and_then(|rule| rule.write_db.clone())
			.unwrap_or_else(|| self.default_db.clone())
	}

	/// Gets the default database alias
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::database_routing::DatabaseRouter;
	///
	/// let router = DatabaseRouter::new("my_default");
	/// assert_eq!(router.default_db(), "my_default");
	/// ```
	pub fn default_db(&self) -> &str {
		&self.default_db
	}

	/// Removes routing rules for a specific model
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::database_routing::DatabaseRouter;
	///
	/// let mut router = DatabaseRouter::new("default")
	///     .add_rule("User", "users_db");
	///
	/// assert_eq!(router.db_for_read("User"), "users_db");
	///
	/// router.remove_rule("User");
	/// assert_eq!(router.db_for_read("User"), "default");
	/// ```
	pub fn remove_rule(&mut self, model_name: &str) {
		self.rules.write().remove(model_name);
	}

	/// Checks if a routing rule exists for the specified model
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::database_routing::DatabaseRouter;
	///
	/// let router = DatabaseRouter::new("default")
	///     .add_rule("User", "users_db");
	///
	/// assert!(router.has_rule("User"));
	/// assert!(!router.has_rule("Unknown"));
	/// ```
	pub fn has_rule(&self, model_name: &str) -> bool {
		self.rules.read().contains_key(model_name)
	}

	/// Clears all routing rules
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::database_routing::DatabaseRouter;
	///
	/// let mut router = DatabaseRouter::new("default")
	///     .add_rule("User", "users_db")
	///     .add_rule("Log", "logs_db");
	///
	/// assert!(router.has_rule("User"));
	/// assert!(router.has_rule("Log"));
	///
	/// router.clear_rules();
	/// assert!(!router.has_rule("User"));
	/// assert!(!router.has_rule("Log"));
	/// ```
	pub fn clear_rules(&mut self) {
		self.rules.write().clear();
	}

	/// Returns the number of routing rules
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::database_routing::DatabaseRouter;
	///
	/// let router = DatabaseRouter::new("default")
	///     .add_rule("User", "users_db")
	///     .add_rule("Log", "logs_db");
	///
	/// assert_eq!(router.rule_count(), 2);
	/// ```
	pub fn rule_count(&self) -> usize {
		self.rules.read().len()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_new_router_with_default_db() {
		let router = DatabaseRouter::new("default");
		assert_eq!(router.default_db(), "default");
		assert_eq!(router.rule_count(), 0);
	}

	#[test]
	fn test_add_rule_single_database() {
		let router = DatabaseRouter::new("default").add_rule("User", "users_db");

		assert_eq!(router.db_for_read("User"), "users_db");
		assert_eq!(router.db_for_write("User"), "users_db");
		assert_eq!(router.rule_count(), 1);
	}

	#[test]
	fn test_add_rule_multiple_models() {
		let router = DatabaseRouter::new("default")
			.add_rule("User", "users_db")
			.add_rule("Log", "logs_db")
			.add_rule("Analytics", "analytics_db");

		assert_eq!(router.db_for_read("User"), "users_db");
		assert_eq!(router.db_for_read("Log"), "logs_db");
		assert_eq!(router.db_for_read("Analytics"), "analytics_db");
		assert_eq!(router.rule_count(), 3);
	}

	#[test]
	fn test_add_rule_overwrites_existing() {
		let router = DatabaseRouter::new("default")
			.add_rule("User", "db1")
			.add_rule("User", "db2");

		assert_eq!(router.db_for_read("User"), "db2");
		assert_eq!(router.db_for_write("User"), "db2");
		assert_eq!(router.rule_count(), 1);
	}

	#[test]
	fn test_add_rule_with_string_types() {
		let router = DatabaseRouter::new(String::from("default"))
			.add_rule(String::from("User"), String::from("users_db"));

		assert_eq!(router.db_for_read("User"), "users_db");
	}

	#[test]
	fn test_add_read_write_rule_separate_databases() {
		let router =
			DatabaseRouter::new("default").add_read_write_rule("User", "replica", "primary");

		assert_eq!(router.db_for_read("User"), "replica");
		assert_eq!(router.db_for_write("User"), "primary");
		assert_eq!(router.rule_count(), 1);
	}

	#[test]
	fn test_add_read_write_rule_multiple_models() {
		let router = DatabaseRouter::new("default")
			.add_read_write_rule("User", "replica1", "primary1")
			.add_read_write_rule("Product", "replica2", "primary2");

		assert_eq!(router.db_for_read("User"), "replica1");
		assert_eq!(router.db_for_write("User"), "primary1");
		assert_eq!(router.db_for_read("Product"), "replica2");
		assert_eq!(router.db_for_write("Product"), "primary2");
	}

	#[test]
	fn test_add_read_write_rule_same_database() {
		let router =
			DatabaseRouter::new("default").add_read_write_rule("Log", "logs_db", "logs_db");

		assert_eq!(router.db_for_read("Log"), "logs_db");
		assert_eq!(router.db_for_write("Log"), "logs_db");
	}

	#[test]
	fn test_add_read_write_rule_overwrites_add_rule() {
		let router = DatabaseRouter::new("default")
			.add_rule("User", "unified_db")
			.add_read_write_rule("User", "replica", "primary");

		assert_eq!(router.db_for_read("User"), "replica");
		assert_eq!(router.db_for_write("User"), "primary");
		assert_eq!(router.rule_count(), 1);
	}

	#[test]
	fn test_add_read_write_rule_with_string_types() {
		let router = DatabaseRouter::new("default").add_read_write_rule(
			String::from("User"),
			String::from("replica"),
			String::from("primary"),
		);

		assert_eq!(router.db_for_read("User"), "replica");
		assert_eq!(router.db_for_write("User"), "primary");
	}

	#[test]
	fn test_add_read_rule_new_model() {
		let router = DatabaseRouter::new("default").add_read_rule("Analytics", "analytics_replica");

		assert_eq!(router.db_for_read("Analytics"), "analytics_replica");
		assert_eq!(router.db_for_write("Analytics"), "default");
	}

	#[test]
	fn test_add_read_rule_existing_model() {
		let router = DatabaseRouter::new("default")
			.add_write_rule("User", "primary")
			.add_read_rule("User", "replica");

		assert_eq!(router.db_for_read("User"), "replica");
		assert_eq!(router.db_for_write("User"), "primary");
	}

	#[test]
	fn test_add_read_rule_overwrites_previous_read() {
		let router = DatabaseRouter::new("default")
			.add_read_rule("Log", "replica1")
			.add_read_rule("Log", "replica2");

		assert_eq!(router.db_for_read("Log"), "replica2");
	}

	#[test]
	fn test_add_read_rule_multiple_models() {
		let router = DatabaseRouter::new("default")
			.add_read_rule("User", "user_replica")
			.add_read_rule("Product", "product_replica")
			.add_read_rule("Order", "order_replica");

		assert_eq!(router.db_for_read("User"), "user_replica");
		assert_eq!(router.db_for_read("Product"), "product_replica");
		assert_eq!(router.db_for_read("Order"), "order_replica");
	}

	#[test]
	fn test_add_read_rule_with_string_types() {
		let router = DatabaseRouter::new("default")
			.add_read_rule(String::from("User"), String::from("replica"));

		assert_eq!(router.db_for_read("User"), "replica");
	}

	#[test]
	fn test_add_write_rule_new_model() {
		let router = DatabaseRouter::new("default").add_write_rule("AuditLog", "audit_primary");

		assert_eq!(router.db_for_read("AuditLog"), "default");
		assert_eq!(router.db_for_write("AuditLog"), "audit_primary");
	}

	#[test]
	fn test_add_write_rule_existing_model() {
		let router = DatabaseRouter::new("default")
			.add_read_rule("User", "replica")
			.add_write_rule("User", "primary");

		assert_eq!(router.db_for_read("User"), "replica");
		assert_eq!(router.db_for_write("User"), "primary");
	}

	#[test]
	fn test_add_write_rule_overwrites_previous_write() {
		let router = DatabaseRouter::new("default")
			.add_write_rule("Order", "primary1")
			.add_write_rule("Order", "primary2");

		assert_eq!(router.db_for_write("Order"), "primary2");
	}

	#[test]
	fn test_add_write_rule_multiple_models() {
		let router = DatabaseRouter::new("default")
			.add_write_rule("User", "user_primary")
			.add_write_rule("Product", "product_primary")
			.add_write_rule("Order", "order_primary");

		assert_eq!(router.db_for_write("User"), "user_primary");
		assert_eq!(router.db_for_write("Product"), "product_primary");
		assert_eq!(router.db_for_write("Order"), "order_primary");
	}

	#[test]
	fn test_add_write_rule_with_string_types() {
		let router = DatabaseRouter::new("default")
			.add_write_rule(String::from("User"), String::from("primary"));

		assert_eq!(router.db_for_write("User"), "primary");
	}

	#[test]
	fn test_db_for_read_returns_default_for_unknown_model() {
		let router = DatabaseRouter::new("default").add_rule("User", "users_db");

		assert_eq!(router.db_for_read("UnknownModel"), "default");
	}

	#[test]
	fn test_db_for_read_with_multiple_rules() {
		let router = DatabaseRouter::new("default")
			.add_rule("User", "users_db")
			.add_read_write_rule("Product", "product_replica", "product_primary");

		assert_eq!(router.db_for_read("User"), "users_db");
		assert_eq!(router.db_for_read("Product"), "product_replica");
		assert_eq!(router.db_for_read("Unknown"), "default");
	}

	#[test]
	fn test_db_for_read_case_sensitive() {
		let router = DatabaseRouter::new("default").add_rule("User", "users_db");

		assert_eq!(router.db_for_read("User"), "users_db");
		assert_eq!(router.db_for_read("user"), "default");
		assert_eq!(router.db_for_read("USER"), "default");
	}

	#[test]
	fn test_db_for_read_empty_router() {
		let router = DatabaseRouter::new("default");
		assert_eq!(router.db_for_read("AnyModel"), "default");
	}

	#[test]
	fn test_db_for_read_returns_reference_with_same_lifetime() {
		let router = DatabaseRouter::new("default").add_rule("User", "users_db");

		let db1 = router.db_for_read("User");
		let db2 = router.db_for_read("User");
		assert_eq!(db1, db2);
	}

	#[test]
	fn test_db_for_write_returns_default_for_unknown_model() {
		let router = DatabaseRouter::new("default").add_rule("User", "users_db");

		assert_eq!(router.db_for_write("UnknownModel"), "default");
	}

	#[test]
	fn test_db_for_write_with_multiple_rules() {
		let router = DatabaseRouter::new("default")
			.add_rule("User", "users_db")
			.add_read_write_rule("Product", "product_replica", "product_primary");

		assert_eq!(router.db_for_write("User"), "users_db");
		assert_eq!(router.db_for_write("Product"), "product_primary");
		assert_eq!(router.db_for_write("Unknown"), "default");
	}

	#[test]
	fn test_db_for_write_case_sensitive() {
		let router = DatabaseRouter::new("default").add_rule("User", "users_db");

		assert_eq!(router.db_for_write("User"), "users_db");
		assert_eq!(router.db_for_write("user"), "default");
		assert_eq!(router.db_for_write("USER"), "default");
	}

	#[test]
	fn test_db_for_write_empty_router() {
		let router = DatabaseRouter::new("default");
		assert_eq!(router.db_for_write("AnyModel"), "default");
	}

	#[test]
	fn test_db_for_write_returns_reference_with_same_lifetime() {
		let router = DatabaseRouter::new("default").add_rule("User", "users_db");

		let db1 = router.db_for_write("User");
		let db2 = router.db_for_write("User");
		assert_eq!(db1, db2);
	}

	#[test]
	fn test_remove_rule_existing_model() {
		let mut router = DatabaseRouter::new("default")
			.add_rule("User", "users_db")
			.add_rule("Product", "products_db");

		assert!(router.has_rule("User"));
		assert_eq!(router.rule_count(), 2);

		router.remove_rule("User");

		assert!(!router.has_rule("User"));
		assert_eq!(router.db_for_read("User"), "default");
		assert_eq!(router.rule_count(), 1);
	}

	#[test]
	fn test_remove_rule_non_existing_model() {
		let mut router = DatabaseRouter::new("default").add_rule("User", "users_db");

		assert_eq!(router.rule_count(), 1);
		router.remove_rule("NonExisting");
		assert_eq!(router.rule_count(), 1);
	}

	#[test]
	fn test_remove_rule_empty_router() {
		let mut router = DatabaseRouter::new("default");

		router.remove_rule("AnyModel");
		assert_eq!(router.rule_count(), 0);
	}

	#[test]
	fn test_remove_rule_case_sensitive() {
		let mut router = DatabaseRouter::new("default").add_rule("User", "users_db");

		router.remove_rule("user");
		assert!(router.has_rule("User"));
		assert_eq!(router.rule_count(), 1);
	}

	#[test]
	fn test_remove_rule_multiple_times() {
		let mut router = DatabaseRouter::new("default").add_rule("User", "users_db");

		router.remove_rule("User");
		router.remove_rule("User");
		assert!(!router.has_rule("User"));
		assert_eq!(router.rule_count(), 0);
	}

	#[test]
	fn test_has_rule_existing_model() {
		let router = DatabaseRouter::new("default").add_rule("User", "users_db");

		assert!(router.has_rule("User"));
	}

	#[test]
	fn test_has_rule_non_existing_model() {
		let router = DatabaseRouter::new("default").add_rule("User", "users_db");

		assert!(!router.has_rule("Product"));
	}

	#[test]
	fn test_has_rule_empty_router() {
		let router = DatabaseRouter::new("default");
		assert!(!router.has_rule("AnyModel"));
	}

	#[test]
	fn test_has_rule_case_sensitive() {
		let router = DatabaseRouter::new("default").add_rule("User", "users_db");

		assert!(router.has_rule("User"));
		assert!(!router.has_rule("user"));
		assert!(!router.has_rule("USER"));
	}

	#[test]
	fn test_has_rule_multiple_models() {
		let router = DatabaseRouter::new("default")
			.add_rule("User", "users_db")
			.add_rule("Product", "products_db");

		assert!(router.has_rule("User"));
		assert!(router.has_rule("Product"));
		assert!(!router.has_rule("Order"));
	}

	#[test]
	fn test_clear_rules_with_existing_rules() {
		let mut router = DatabaseRouter::new("default")
			.add_rule("User", "users_db")
			.add_rule("Product", "products_db")
			.add_rule("Order", "orders_db");

		assert_eq!(router.rule_count(), 3);

		router.clear_rules();

		assert_eq!(router.rule_count(), 0);
		assert!(!router.has_rule("User"));
		assert!(!router.has_rule("Product"));
		assert!(!router.has_rule("Order"));
	}

	#[test]
	fn test_clear_rules_empty_router() {
		let mut router = DatabaseRouter::new("default");

		router.clear_rules();
		assert_eq!(router.rule_count(), 0);
	}

	#[test]
	fn test_clear_rules_returns_to_default() {
		let mut router = DatabaseRouter::new("default").add_rule("User", "users_db");

		router.clear_rules();

		assert_eq!(router.db_for_read("User"), "default");
		assert_eq!(router.db_for_write("User"), "default");
	}

	#[test]
	fn test_clear_rules_multiple_times() {
		let mut router = DatabaseRouter::new("default").add_rule("User", "users_db");

		router.clear_rules();
		router.clear_rules();
		assert_eq!(router.rule_count(), 0);
	}

	#[test]
	fn test_clear_rules_then_add_new() {
		let mut router = DatabaseRouter::new("default").add_rule("User", "users_db");

		router.clear_rules();

		let router = router.add_rule("Product", "products_db");

		assert_eq!(router.rule_count(), 1);
		assert!(!router.has_rule("User"));
		assert!(router.has_rule("Product"));
	}

	#[test]
	fn test_rule_count_empty_router() {
		let router = DatabaseRouter::new("default");
		assert_eq!(router.rule_count(), 0);
	}

	#[test]
	fn test_rule_count_single_rule() {
		let router = DatabaseRouter::new("default").add_rule("User", "users_db");

		assert_eq!(router.rule_count(), 1);
	}

	#[test]
	fn test_rule_count_multiple_rules() {
		let router = DatabaseRouter::new("default")
			.add_rule("User", "users_db")
			.add_rule("Product", "products_db")
			.add_rule("Order", "orders_db");

		assert_eq!(router.rule_count(), 3);
	}

	#[test]
	fn test_rule_count_after_overwrite() {
		let router = DatabaseRouter::new("default")
			.add_rule("User", "db1")
			.add_rule("User", "db2");

		assert_eq!(router.rule_count(), 1);
	}

	#[test]
	fn test_rule_count_after_remove() {
		let mut router = DatabaseRouter::new("default")
			.add_rule("User", "users_db")
			.add_rule("Product", "products_db");

		router.remove_rule("User");
		assert_eq!(router.rule_count(), 1);
	}

	#[test]
	fn test_clone_router_preserves_state() {
		let router1 = DatabaseRouter::new("default").add_rule("User", "users_db");

		let router2 = router1.clone();

		assert_eq!(router2.db_for_read("User"), "users_db");
		assert_eq!(router2.default_db(), "default");
		assert_eq!(router2.rule_count(), 1);
	}

	#[test]
	fn test_clone_router_shares_rules() {
		let router1 = DatabaseRouter::new("default").add_rule("User", "users_db");

		let mut router2 = router1.clone();

		assert!(router1.has_rule("User"));
		assert!(router2.has_rule("User"));

		router2.remove_rule("User");

		assert!(!router1.has_rule("User"));
		assert!(!router2.has_rule("User"));
	}

	#[test]
	fn test_complex_routing_scenario() {
		let router = DatabaseRouter::new("default")
			.add_rule("Session", "sessions_db")
			.add_read_write_rule("User", "user_replica", "user_primary")
			.add_read_rule("Analytics", "analytics_replica")
			.add_write_rule("AuditLog", "audit_primary");

		assert_eq!(router.db_for_read("Session"), "sessions_db");
		assert_eq!(router.db_for_write("Session"), "sessions_db");

		assert_eq!(router.db_for_read("User"), "user_replica");
		assert_eq!(router.db_for_write("User"), "user_primary");

		assert_eq!(router.db_for_read("Analytics"), "analytics_replica");
		assert_eq!(router.db_for_write("Analytics"), "default");

		assert_eq!(router.db_for_read("AuditLog"), "default");
		assert_eq!(router.db_for_write("AuditLog"), "audit_primary");

		assert_eq!(router.db_for_read("Unknown"), "default");
		assert_eq!(router.db_for_write("Unknown"), "default");
	}

	#[test]
	fn test_thread_safety_concurrent_reads() {
		use std::sync::Arc;
		use std::thread;

		let router = Arc::new(
			DatabaseRouter::new("default")
				.add_rule("User", "users_db")
				.add_rule("Product", "products_db"),
		);

		let handles: Vec<_> = (0..10)
			.map(|i| {
				let router_clone = Arc::clone(&router);
				thread::spawn(move || {
					let model = if i % 2 == 0 { "User" } else { "Product" };
					let db = router_clone.db_for_read(model);
					assert!(!db.is_empty());
				})
			})
			.collect();

		for handle in handles {
			handle.join().unwrap();
		}
	}

	#[test]
	fn test_thread_safety_concurrent_writes() {
		use std::sync::Arc;
		use std::thread;

		let router = Arc::new(DatabaseRouter::new("default"));

		let handles: Vec<_> = (0..10)
			.map(|i| {
				let _router = Arc::clone(&router);
				thread::spawn(move || {
					let model_name = format!("Model{}", i);
					let db_name = format!("db{}", i);

					let new_router = DatabaseRouter::new("default").add_rule(&model_name, &db_name);

					assert_eq!(new_router.db_for_read(&model_name), db_name);
				})
			})
			.collect();

		for handle in handles {
			handle.join().unwrap();
		}
	}
}
