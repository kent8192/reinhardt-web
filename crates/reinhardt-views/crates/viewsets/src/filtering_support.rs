//! Filtering and ordering support for ViewSets
//!
//! Provides automatic filtering and ordering integration for list actions in ViewSets.

use async_trait::async_trait;
use reinhardt_apps::Request;
use std::collections::HashMap;
use std::marker::PhantomData;

/// Filtering configuration for ViewSets
#[derive(Debug, Clone)]
pub struct FilterConfig {
	/// Fields that can be filtered by exact match
	pub filterable_fields: Vec<String>,
	/// Fields that can be searched (contains/icontains)
	pub search_fields: Vec<String>,
	/// Enable case-insensitive search
	pub case_insensitive_search: bool,
}

impl Default for FilterConfig {
	fn default() -> Self {
		Self {
			filterable_fields: Vec::new(),
			search_fields: Vec::new(),
			case_insensitive_search: true,
		}
	}
}

impl FilterConfig {
	/// Create a new filter configuration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_viewsets::FilterConfig;
	///
	/// let config = FilterConfig::new()
	///     .with_filterable_fields(vec!["status", "category"])
	///     .with_search_fields(vec!["title", "description"]);
	/// ```
	pub fn new() -> Self {
		Self::default()
	}

	/// Set filterable fields
	pub fn with_filterable_fields(mut self, fields: Vec<impl Into<String>>) -> Self {
		self.filterable_fields = fields.into_iter().map(|f| f.into()).collect();
		self
	}

	/// Set search fields
	pub fn with_search_fields(mut self, fields: Vec<impl Into<String>>) -> Self {
		self.search_fields = fields.into_iter().map(|f| f.into()).collect();
		self
	}

	/// Enable or disable case-insensitive search
	pub fn case_insensitive(mut self, enabled: bool) -> Self {
		self.case_insensitive_search = enabled;
		self
	}
}

/// Ordering configuration for ViewSets
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct OrderingConfig {
	/// Fields that can be used for ordering
	pub ordering_fields: Vec<String>,
	/// Default ordering (field name with optional '-' prefix for descending)
	pub default_ordering: Vec<String>,
}


impl OrderingConfig {
	/// Create a new ordering configuration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_viewsets::OrderingConfig;
	///
	/// let config = OrderingConfig::new()
	///     .with_ordering_fields(vec!["created_at", "title", "id"])
	///     .with_default_ordering(vec!["-created_at"]);
	/// ```
	pub fn new() -> Self {
		Self::default()
	}

	/// Set orderable fields
	pub fn with_ordering_fields(mut self, fields: Vec<impl Into<String>>) -> Self {
		self.ordering_fields = fields.into_iter().map(|f| f.into()).collect();
		self
	}

	/// Set default ordering
	/// Use '-' prefix for descending order (e.g., "-created_at")
	pub fn with_default_ordering(mut self, ordering: Vec<impl Into<String>>) -> Self {
		self.default_ordering = ordering.into_iter().map(|o| o.into()).collect();
		self
	}
}

/// Trait for ViewSets that support filtering and ordering
#[async_trait]
pub trait FilterableViewSet: Send + Sync {
	/// Get filter configuration for this ViewSet
	fn get_filter_config(&self) -> Option<FilterConfig> {
		None
	}

	/// Get ordering configuration for this ViewSet
	fn get_ordering_config(&self) -> Option<OrderingConfig> {
		None
	}

	/// Extract filter parameters from request
	///
	/// This method parses query parameters and returns a HashMap of field->value filters.
	fn extract_filters(&self, request: &Request) -> HashMap<String, String> {
		let query_string = request.uri.query().unwrap_or("");
		let mut filters = HashMap::new();

		if query_string.is_empty() {
			return filters;
		}

		// Parse query string into key-value pairs
		for pair in query_string.split('&') {
			if let Some((key, value)) = pair.split_once('=') {
				// Skip pagination and ordering parameters
				if key != "page"
					&& key != "page_size"
					&& key != "limit"
					&& key != "offset"
					&& key != "cursor"
					&& key != "ordering"
					&& key != "search"
				{
					// URL decode the value
					if let Ok(decoded_value) = urlencoding::decode(value) {
						filters.insert(key.to_string(), decoded_value.into_owned());
					}
				}
			}
		}

		// Validate against filterable fields if configured
		if let Some(config) = self.get_filter_config()
			&& !config.filterable_fields.is_empty() {
				filters.retain(|key, _| config.filterable_fields.contains(key));
			}

		filters
	}

	/// Extract search parameter from request
	fn extract_search(&self, request: &Request) -> Option<String> {
		let query_string = request.uri.query().unwrap_or("");

		for pair in query_string.split('&') {
			if let Some((key, value)) = pair.split_once('=')
				&& key == "search"
					&& let Ok(decoded_value) = urlencoding::decode(value) {
						return Some(decoded_value.into_owned());
					}
		}

		None
	}

	/// Extract ordering from request
	///
	/// Returns a list of field names with optional '-' prefix for descending order.
	/// Example: ["created_at", "-title"] means order by created_at ASC, then title DESC.
	fn extract_ordering(&self, request: &Request) -> Vec<String> {
		let query_string = request.uri.query().unwrap_or("");

		for pair in query_string.split('&') {
			if let Some((key, value)) = pair.split_once('=')
				&& key == "ordering"
					&& let Ok(decoded_value) = urlencoding::decode(value) {
						let requested_fields: Vec<String> = decoded_value
							.split(',')
							.map(|s| s.trim().to_string())
							.collect();

						// Validate against orderable fields if configured
						if let Some(config) = self.get_ordering_config() {
							if !config.ordering_fields.is_empty() {
								let validated_fields: Vec<String> = requested_fields
									.into_iter()
									.filter(|field| {
										let field_name = field.trim_start_matches('-');
										config.ordering_fields.contains(&field_name.to_string())
									})
									.collect();

								if !validated_fields.is_empty() {
									return validated_fields;
								}
							} else {
								return requested_fields;
							}
						} else {
							return requested_fields;
						}
					}
		}

		// Return default ordering if configured
		if let Some(config) = self.get_ordering_config()
			&& !config.default_ordering.is_empty() {
				return config.default_ordering.clone();
			}

		Vec::new()
	}
}

/// Helper struct for applying filters to in-memory collections
pub struct InMemoryFilter<T> {
	_phantom: PhantomData<T>,
}

impl<T> InMemoryFilter<T> {
	/// Filter items based on field values
	///
	/// This is a simple implementation for in-memory filtering.
	/// For database-backed filtering, use QueryFilter with ORM.
	pub fn filter<F>(items: Vec<T>, predicate: F) -> Vec<T>
	where
		F: Fn(&T) -> bool,
	{
		items.into_iter().filter(predicate).collect()
	}

	/// Sort items based on a comparison function
	pub fn sort<F>(mut items: Vec<T>, compare: F) -> Vec<T>
	where
		F: Fn(&T, &T) -> std::cmp::Ordering,
	{
		items.sort_by(compare);
		items
	}
}
