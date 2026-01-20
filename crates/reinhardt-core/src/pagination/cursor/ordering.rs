//! Ordering strategies for cursor-based pagination

/// Trait for defining custom ordering strategies
///
/// Implementations determine how items are ordered when using cursor pagination.
/// This is particularly important for stable cursor-based pagination where the
/// ordering must remain consistent across requests.
pub trait OrderingStrategy: Send + Sync {
	/// Returns the ordering fields for this strategy
	///
	/// Fields prefixed with '-' indicate descending order.
	fn fields(&self) -> Vec<String>;

	/// Returns a description of this ordering strategy
	fn description(&self) -> String {
		format!("Ordering by: {}", self.fields().join(", "))
	}
}

/// Order by creation timestamp in descending order
///
/// This is the default ordering strategy for cursor pagination,
/// showing the most recently created items first.
///
/// # Examples
///
/// ```
/// use reinhardt_core::pagination::cursor::{CreatedAtOrdering, OrderingStrategy};
///
/// let ordering = CreatedAtOrdering::new();
/// assert_eq!(ordering.fields(), vec!["-created_at", "id"]);
/// ```
#[derive(Debug, Clone)]
pub struct CreatedAtOrdering {
	/// Field name for creation timestamp
	pub created_field: String,
	/// Field name for stable ordering (usually ID)
	pub stable_field: String,
}

impl CreatedAtOrdering {
	/// Create a new CreatedAtOrdering with default field names
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core::pagination::cursor::CreatedAtOrdering;
	///
	/// let ordering = CreatedAtOrdering::new();
	/// assert_eq!(ordering.created_field, "created_at");
	/// assert_eq!(ordering.stable_field, "id");
	/// ```
	pub fn new() -> Self {
		Self {
			created_field: "created_at".to_string(),
			stable_field: "id".to_string(),
		}
	}

	/// Set custom field names
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core::pagination::cursor::CreatedAtOrdering;
	///
	/// let ordering = CreatedAtOrdering::new()
	///     .with_fields("created", "pk");
	/// assert_eq!(ordering.created_field, "created");
	/// assert_eq!(ordering.stable_field, "pk");
	/// ```
	pub fn with_fields(mut self, created: &str, stable: &str) -> Self {
		self.created_field = created.to_string();
		self.stable_field = stable.to_string();
		self
	}
}

impl Default for CreatedAtOrdering {
	fn default() -> Self {
		Self::new()
	}
}

impl OrderingStrategy for CreatedAtOrdering {
	fn fields(&self) -> Vec<String> {
		vec![
			format!("-{}", self.created_field),
			self.stable_field.clone(),
		]
	}
}

/// Order by ID field
///
/// Simple ordering strategy that orders items by their ID field.
/// Can be configured for ascending or descending order.
///
/// # Examples
///
/// ```
/// use reinhardt_core::pagination::cursor::{IdOrdering, OrderingStrategy};
///
/// let ordering = IdOrdering::new();
/// assert_eq!(ordering.fields(), vec!["id"]);
///
/// let desc_ordering = IdOrdering::descending();
/// assert_eq!(desc_ordering.fields(), vec!["-id"]);
/// ```
#[derive(Debug, Clone)]
pub struct IdOrdering {
	/// Field name for ID
	pub id_field: String,
	/// Ascending (true) or descending (false) order
	pub ascending: bool,
}

impl IdOrdering {
	/// Create a new IdOrdering with ascending order
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core::pagination::cursor::IdOrdering;
	///
	/// let ordering = IdOrdering::new();
	/// assert_eq!(ordering.id_field, "id");
	/// assert!(ordering.ascending);
	/// ```
	pub fn new() -> Self {
		Self {
			id_field: "id".to_string(),
			ascending: true,
		}
	}

	/// Create a new IdOrdering with descending order
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core::pagination::cursor::IdOrdering;
	///
	/// let ordering = IdOrdering::descending();
	/// assert!(!ordering.ascending);
	/// ```
	pub fn descending() -> Self {
		Self {
			id_field: "id".to_string(),
			ascending: false,
		}
	}

	/// Set custom field name
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core::pagination::cursor::IdOrdering;
	///
	/// let ordering = IdOrdering::new().with_field("pk");
	/// assert_eq!(ordering.id_field, "pk");
	/// ```
	pub fn with_field(mut self, field: &str) -> Self {
		self.id_field = field.to_string();
		self
	}
}

impl Default for IdOrdering {
	fn default() -> Self {
		Self::new()
	}
}

impl OrderingStrategy for IdOrdering {
	fn fields(&self) -> Vec<String> {
		if self.ascending {
			vec![self.id_field.clone()]
		} else {
			vec![format!("-{}", self.id_field)]
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_created_at_ordering_default() {
		let ordering = CreatedAtOrdering::new();
		assert_eq!(ordering.fields(), vec!["-created_at", "id"]);
	}

	#[test]
	fn test_created_at_ordering_custom_fields() {
		let ordering = CreatedAtOrdering::new().with_fields("created", "pk");
		assert_eq!(ordering.fields(), vec!["-created", "pk"]);
	}

	#[test]
	fn test_id_ordering_ascending() {
		let ordering = IdOrdering::new();
		assert_eq!(ordering.fields(), vec!["id"]);
		assert!(ordering.ascending);
	}

	#[test]
	fn test_id_ordering_descending() {
		let ordering = IdOrdering::descending();
		assert_eq!(ordering.fields(), vec!["-id"]);
		assert!(!ordering.ascending);
	}

	#[test]
	fn test_id_ordering_custom_field() {
		let ordering = IdOrdering::new().with_field("pk");
		assert_eq!(ordering.fields(), vec!["pk"]);
	}

	#[test]
	fn test_ordering_description() {
		let ordering = CreatedAtOrdering::new();
		assert_eq!(ordering.description(), "Ordering by: -created_at, id");
	}
}
