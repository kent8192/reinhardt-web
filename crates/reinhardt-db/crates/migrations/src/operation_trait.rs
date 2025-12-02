//! Operation trait for migration operations
//!
//! This module provides a unified trait for all migration operations,
//! enabling Django-style migration name generation via `migration_name_fragment()`.

/// Trait for migration operations
///
/// This trait provides a unified interface for all migration operations,
/// following Django's migration system design.
///
/// # Django Compatibility
///
/// This trait is designed to be compatible with Django's `Operation` class:
/// - `migration_name_fragment()` → Django's `migration_name_fragment` property
/// - `describe()` → Human-readable description for CLI output
///
/// # Example
///
/// ```rust,ignore
/// use reinhardt_migrations::operation_trait::MigrationOperation;
///
/// struct AddField {
///     model_name: String,
///     field_name: String,
/// }
///
/// impl MigrationOperation for AddField {
///     fn migration_name_fragment(&self) -> Option<String> {
///         Some(format!("{}_{}",
///             self.model_name.to_lowercase(),
///             self.field_name.to_lowercase()
///         ))
///     }
///
///     fn describe(&self) -> String {
///         format!("Add field {} to {}", self.field_name, self.model_name)
///     }
/// }
/// ```
pub trait MigrationOperation {
	/// Generate a fragment for the migration name
	///
	/// Returns `Some(String)` if this operation can contribute a meaningful
	/// name fragment, or `None` if it should trigger fallback to auto-generated
	/// timestamp-based naming (e.g., `auto_20251202_1430`).
	///
	/// # Django Compatibility
	///
	/// This follows Django's naming rules:
	/// - `CreateModel { name: "User" }` → `Some("user")`
	/// - `AddField { model: "User", field: "email" }` → `Some("user_email")`
	/// - `RemoveField { model: "User", field: "age" }` → `Some("remove_user_age")`
	/// - `RunSQL { ... }` → `None` (triggers auto-naming)
	///
	/// # Naming Conventions
	///
	/// - Use lowercase for model and field names
	/// - Use underscores to separate words
	/// - Use descriptive prefixes (e.g., `remove_`, `alter_`, `rename_`)
	/// - Keep fragments concise (they will be combined with `_`)
	///
	/// # Returns
	///
	/// - `Some(fragment)` - This operation provides a name fragment
	/// - `None` - This operation cannot provide a meaningful name (e.g., RunSQL, RunCode)
	fn migration_name_fragment(&self) -> Option<String>;

	/// Human-readable description of this operation
	///
	/// Used for CLI output when displaying migration contents.
	///
	/// # Example Output
	///
	/// ```text
	/// - Add field email to User
	/// - Remove field age from Profile
	/// - Create model Post
	/// ```
	fn describe(&self) -> String;

	/// Normalize operation for comparison
	///
	/// Returns a normalized version of this operation where order-independent
	/// elements (like column lists, constraint lists) are sorted for consistent
	/// comparison.
	///
	/// This is used for semantic equality checking to detect duplicate migrations
	/// even when the order of elements differs.
	fn normalize(&self) -> Self
	where
		Self: Sized + Clone,
	{
		self.clone()
	}

	/// Check if two operations are semantically equal
	///
	/// Two operations are semantically equal if they produce the same database
	/// schema changes, even if their internal representation differs in ordering.
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// let op1 = CreateIndex {
	///     table: "users",
	///     columns: vec!["email", "name"],
	/// };
	/// let op2 = CreateIndex {
	///     table: "users",
	///     columns: vec!["name", "email"],  // Different order
	/// };
	///
	/// assert!(op1.semantically_equal(&op2));
	/// ```
	fn semantically_equal(&self, other: &Self) -> bool
	where
		Self: Sized + Clone + PartialEq,
	{
		self.normalize() == other.normalize()
	}
}
