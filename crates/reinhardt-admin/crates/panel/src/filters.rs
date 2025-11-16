//! List filtering functionality for admin views
//!
//! This module provides the infrastructure for filtering querysets in admin list views.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Specification for a single filter option
///
/// # Examples
///
/// ```
/// use reinhardt_admin_panel::FilterSpec;
///
/// let filter = FilterSpec {
///     field: "status".to_string(),
///     lookup: "exact".to_string(),
///     value: "active".to_string(),
///     display: "Active".to_string(),
/// };
///
/// assert_eq!(filter.field, "status");
/// assert_eq!(filter.lookup, "exact");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FilterSpec {
	/// Field name to filter on
	pub field: String,
	/// Lookup type (exact, contains, gte, lte, etc.)
	pub lookup: String,
	/// Filter value
	pub value: String,
	/// Display text for the filter option
	pub display: String,
}

impl FilterSpec {
	/// Create a new filter specification
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin_panel::FilterSpec;
	///
	/// let filter = FilterSpec::new("status", "exact", "active", "Active Items");
	/// assert_eq!(filter.field, "status");
	/// ```
	pub fn new(
		field: impl Into<String>,
		lookup: impl Into<String>,
		value: impl Into<String>,
		display: impl Into<String>,
	) -> Self {
		Self {
			field: field.into(),
			lookup: lookup.into(),
			value: value.into(),
			display: display.into(),
		}
	}

	/// Convert to query parameter format
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin_panel::FilterSpec;
	///
	/// let filter = FilterSpec::new("status", "exact", "active", "Active");
	/// assert_eq!(filter.to_query_param(), "status__exact=active");
	/// ```
	pub fn to_query_param(&self) -> String {
		format!("{}__{}={}", self.field, self.lookup, self.value)
	}
}

/// Trait for list filters
///
/// Implement this trait to create custom filters for admin list views.
///
/// # Examples
///
/// ```
/// use reinhardt_admin_panel::{ListFilter, FilterSpec};
///
/// struct StatusFilter {
///     field: String,
/// }
///
/// impl ListFilter for StatusFilter {
///     fn field_name(&self) -> &str {
///         &self.field
///     }
///
///     fn title(&self) -> &str {
///         "Status"
///     }
///
///     fn choices(&self) -> Vec<FilterSpec> {
///         vec![
///             FilterSpec::new("status", "exact", "active", "Active"),
///             FilterSpec::new("status", "exact", "inactive", "Inactive"),
///         ]
///     }
/// }
/// ```
pub trait ListFilter: Send + Sync {
	/// Get the field name this filter applies to
	fn field_name(&self) -> &str;

	/// Get the filter title displayed in UI
	fn title(&self) -> &str;

	/// Get available filter choices
	fn choices(&self) -> Vec<FilterSpec>;

	/// Get the lookup type (default: "exact")
	fn lookup_type(&self) -> &str {
		"exact"
	}

	/// Check if a value is selected
	fn is_selected(&self, value: &str, current_filters: &HashMap<String, String>) -> bool {
		current_filters
			.get(self.field_name())
			.map(|v| v == value)
			.unwrap_or(false)
	}
}

/// Simple boolean filter
///
/// # Examples
///
/// ```
/// use reinhardt_admin_panel::{BooleanFilter, ListFilter};
///
/// let filter = BooleanFilter::new("is_active", "Active Status");
/// assert_eq!(filter.field_name(), "is_active");
/// assert_eq!(filter.title(), "Active Status");
/// ```
#[derive(Debug, Clone)]
pub struct BooleanFilter {
	field: String,
	title: String,
}

impl BooleanFilter {
	/// Create a new boolean filter
	pub fn new(field: impl Into<String>, title: impl Into<String>) -> Self {
		Self {
			field: field.into(),
			title: title.into(),
		}
	}
}

impl ListFilter for BooleanFilter {
	fn field_name(&self) -> &str {
		&self.field
	}

	fn title(&self) -> &str {
		&self.title
	}

	fn choices(&self) -> Vec<FilterSpec> {
		vec![
			FilterSpec::new(&self.field, "exact", "true", "Yes"),
			FilterSpec::new(&self.field, "exact", "false", "No"),
		]
	}
}

/// Choice filter for fields with predefined values
///
/// # Examples
///
/// ```
/// use reinhardt_admin_panel::{ChoiceFilter, ListFilter};
///
/// let filter = ChoiceFilter::new("status", "Status")
///     .add_choice("active", "Active")
///     .add_choice("inactive", "Inactive")
///     .add_choice("pending", "Pending");
///
/// assert_eq!(filter.choices().len(), 3);
/// ```
#[derive(Debug, Clone)]
pub struct ChoiceFilter {
	field: String,
	title: String,
	choices: Vec<(String, String)>, // (value, display)
}

impl ChoiceFilter {
	/// Create a new choice filter
	pub fn new(field: impl Into<String>, title: impl Into<String>) -> Self {
		Self {
			field: field.into(),
			title: title.into(),
			choices: Vec::new(),
		}
	}

	/// Add a choice option
	pub fn add_choice(mut self, value: impl Into<String>, display: impl Into<String>) -> Self {
		self.choices.push((value.into(), display.into()));
		self
	}

	/// Set all choices at once
	pub fn with_choices(mut self, choices: Vec<(String, String)>) -> Self {
		self.choices = choices;
		self
	}
}

impl ListFilter for ChoiceFilter {
	fn field_name(&self) -> &str {
		&self.field
	}

	fn title(&self) -> &str {
		&self.title
	}

	fn choices(&self) -> Vec<FilterSpec> {
		self.choices
			.iter()
			.map(|(value, display)| FilterSpec::new(&self.field, "exact", value, display))
			.collect()
	}
}

/// Date range filter
///
/// # Examples
///
/// ```
/// use reinhardt_admin_panel::{DateRangeFilter, ListFilter};
///
/// let filter = DateRangeFilter::new("created_at", "Created Date");
/// let choices = filter.choices();
///
/// // Has options like "Today", "This week", "This month"
/// assert!(!choices.is_empty());
/// ```
#[derive(Debug, Clone)]
pub struct DateRangeFilter {
	field: String,
	title: String,
}

impl DateRangeFilter {
	/// Create a new date range filter
	pub fn new(field: impl Into<String>, title: impl Into<String>) -> Self {
		Self {
			field: field.into(),
			title: title.into(),
		}
	}
}

impl ListFilter for DateRangeFilter {
	fn field_name(&self) -> &str {
		&self.field
	}

	fn title(&self) -> &str {
		&self.title
	}

	fn choices(&self) -> Vec<FilterSpec> {
		use chrono::{Datelike, Duration, Local};

		let now = Local::now();
		let today = now.date_naive();

		// Calculate date boundaries
		let week_start = today - Duration::days(today.weekday().num_days_from_monday() as i64);
		let month_start = today.with_day(1).unwrap();
		let year_start = today.with_month(1).and_then(|d| d.with_day(1)).unwrap();

		// Last 7 days and last 30 days
		let last_7_days = today - Duration::days(7);
		let last_30_days = today - Duration::days(30);

		vec![
			FilterSpec::new(&self.field, "gte", today.to_string(), "Today"),
			FilterSpec::new(&self.field, "gte", week_start.to_string(), "This week"),
			FilterSpec::new(&self.field, "gte", month_start.to_string(), "This month"),
			FilterSpec::new(&self.field, "gte", year_start.to_string(), "This year"),
			FilterSpec::new(&self.field, "gte", last_7_days.to_string(), "Last 7 days"),
			FilterSpec::new(&self.field, "gte", last_30_days.to_string(), "Last 30 days"),
		]
	}

	fn lookup_type(&self) -> &str {
		"gte" // Greater than or equal to
	}
}

/// Number range filter
///
/// # Examples
///
/// ```
/// use reinhardt_admin_panel::{NumberRangeFilter, ListFilter};
///
/// let filter = NumberRangeFilter::new("price", "Price Range");
/// let choices = filter.choices();
///
/// // Has options like "Less than 100", "100-1000", "More than 1000"
/// assert!(!choices.is_empty());
/// ```
#[derive(Debug, Clone)]
pub struct NumberRangeFilter {
	field: String,
	title: String,
	ranges: Vec<(Option<i64>, Option<i64>, String)>, // (min, max, display)
}

impl NumberRangeFilter {
	/// Create a new number range filter with default ranges
	pub fn new(field: impl Into<String>, title: impl Into<String>) -> Self {
		Self {
			field: field.into(),
			title: title.into(),
			ranges: vec![
				(Some(0), Some(100), "0-100".to_string()),
				(Some(100), Some(1000), "100-1,000".to_string()),
				(Some(1000), Some(10000), "1,000-10,000".to_string()),
				(Some(10000), None, "10,000+".to_string()),
			],
		}
	}

	/// Create a number range filter with custom ranges
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_admin_panel::NumberRangeFilter;
	///
	/// let filter = NumberRangeFilter::with_ranges(
	///     "price",
	///     "Price",
	///     vec![
	///         (Some(0), Some(50), "Under $50".to_string()),
	///         (Some(50), Some(100), "$50-$100".to_string()),
	///         (Some(100), None, "Over $100".to_string()),
	///     ]
	/// );
	/// ```
	pub fn with_ranges(
		field: impl Into<String>,
		title: impl Into<String>,
		ranges: Vec<(Option<i64>, Option<i64>, String)>,
	) -> Self {
		Self {
			field: field.into(),
			title: title.into(),
			ranges,
		}
	}
}

impl ListFilter for NumberRangeFilter {
	fn field_name(&self) -> &str {
		&self.field
	}

	fn title(&self) -> &str {
		&self.title
	}

	fn choices(&self) -> Vec<FilterSpec> {
		self.ranges
			.iter()
			.map(|(min, max, display)| {
				let value = match (min, max) {
					(Some(min_val), Some(max_val)) => format!("{}-{}", min_val, max_val),
					(Some(min_val), None) => format!("{}+", min_val),
					(None, Some(max_val)) => format!("-{}", max_val),
					(None, None) => "all".to_string(),
				};
				FilterSpec::new(&self.field, "range", value, display)
			})
			.collect()
	}

	fn lookup_type(&self) -> &str {
		"range"
	}
}

/// Filter manager for handling multiple filters
///
/// # Examples
///
/// ```
/// use reinhardt_admin_panel::{FilterManager, BooleanFilter, ChoiceFilter};
///
/// let manager = FilterManager::new()
///     .add_filter(BooleanFilter::new("is_active", "Active"))
///     .add_filter(
///         ChoiceFilter::new("status", "Status")
///             .add_choice("draft", "Draft")
///             .add_choice("published", "Published")
///     );
///
/// assert_eq!(manager.filter_count(), 2);
/// ```
pub struct FilterManager {
	filters: Vec<Box<dyn ListFilter>>,
	// Cache for filter lookups by field name
	filter_cache: std::sync::Arc<dashmap::DashMap<String, usize>>,
}

impl FilterManager {
	/// Create a new filter manager
	pub fn new() -> Self {
		Self {
			filters: Vec::new(),
			filter_cache: std::sync::Arc::new(dashmap::DashMap::new()),
		}
	}

	/// Add a filter
	pub fn add_filter(mut self, filter: impl ListFilter + 'static) -> Self {
		let field_name = filter.field_name().to_string();
		let index = self.filters.len();
		self.filters.push(Box::new(filter));
		// Update cache with new filter index
		self.filter_cache.insert(field_name, index);
		self
	}

	/// Get all filters
	pub fn filters(&self) -> &[Box<dyn ListFilter>] {
		&self.filters
	}

	/// Get number of filters
	pub fn filter_count(&self) -> usize {
		self.filters.len()
	}

	/// Check if any filters are present
	pub fn is_empty(&self) -> bool {
		self.filters.is_empty()
	}

	/// Get filter by field name (cached lookup)
	pub fn get_filter(&self, field_name: &str) -> Option<&dyn ListFilter> {
		// Try cache first
		if let Some(index) = self.filter_cache.get(field_name) {
			return self.filters.get(*index).map(|b| &**b);
		}

		// Fallback to linear search and update cache
		for (idx, filter) in self.filters.iter().enumerate() {
			if filter.field_name() == field_name {
				self.filter_cache.insert(field_name.to_string(), idx);
				return Some(&**filter);
			}
		}

		None
	}

	/// Apply filters to generate query parameters
	pub fn apply_filters(&self, selected: &HashMap<String, String>) -> Vec<String> {
		selected
			.iter()
			.filter_map(|(field, value)| {
				self.get_filter(field)
					.map(|filter| format!("{}__{}={}", field, filter.lookup_type(), value))
			})
			.collect()
	}
}

impl Default for FilterManager {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_filter_spec_new() {
		let filter = FilterSpec::new("status", "exact", "active", "Active");
		assert_eq!(filter.field, "status");
		assert_eq!(filter.lookup, "exact");
		assert_eq!(filter.value, "active");
		assert_eq!(filter.display, "Active");
	}

	#[test]
	fn test_filter_spec_to_query_param() {
		let filter = FilterSpec::new("status", "exact", "active", "Active");
		assert_eq!(filter.to_query_param(), "status__exact=active");

		let filter = FilterSpec::new("created_at", "gte", "2025-01-01", "After 2025");
		assert_eq!(filter.to_query_param(), "created_at__gte=2025-01-01");
	}

	#[test]
	fn test_boolean_filter() {
		let filter = BooleanFilter::new("is_active", "Active Status");
		assert_eq!(filter.field_name(), "is_active");
		assert_eq!(filter.title(), "Active Status");

		let choices = filter.choices();
		assert_eq!(choices.len(), 2);
		assert_eq!(choices[0].value, "true");
		assert_eq!(choices[1].value, "false");
	}

	#[test]
	fn test_choice_filter() {
		let filter = ChoiceFilter::new("status", "Status")
			.add_choice("draft", "Draft")
			.add_choice("published", "Published")
			.add_choice("archived", "Archived");

		assert_eq!(filter.field_name(), "status");
		assert_eq!(filter.title(), "Status");

		let choices = filter.choices();
		assert_eq!(choices.len(), 3);
		assert_eq!(choices[0].value, "draft");
		assert_eq!(choices[1].value, "published");
		assert_eq!(choices[2].value, "archived");
	}

	#[test]
	fn test_choice_filter_with_choices() {
		let choices = vec![
			("active".to_string(), "Active".to_string()),
			("inactive".to_string(), "Inactive".to_string()),
		];

		let filter = ChoiceFilter::new("status", "Status").with_choices(choices);

		let filter_choices = filter.choices();
		assert_eq!(filter_choices.len(), 2);
	}

	#[test]
	fn test_date_range_filter() {
		let filter = DateRangeFilter::new("created_at", "Created Date");
		assert_eq!(filter.field_name(), "created_at");
		assert_eq!(filter.title(), "Created Date");
		assert_eq!(filter.lookup_type(), "gte");

		let choices = filter.choices();
		assert_eq!(choices.len(), 6); // Now includes "Last 7 days" and "Last 30 days"

		// Verify choice displays (values will be actual dates)
		assert_eq!(choices[0].display, "Today");
		assert_eq!(choices[1].display, "This week");
		assert_eq!(choices[2].display, "This month");
		assert_eq!(choices[3].display, "This year");
		assert_eq!(choices[4].display, "Last 7 days");
		assert_eq!(choices[5].display, "Last 30 days");
	}

	#[test]
	fn test_number_range_filter_default() {
		let filter = NumberRangeFilter::new("price", "Price Range");
		assert_eq!(filter.field_name(), "price");
		assert_eq!(filter.title(), "Price Range");
		assert_eq!(filter.lookup_type(), "range");

		let choices = filter.choices();
		assert_eq!(choices.len(), 4);
		assert_eq!(choices[0].value, "0-100");
		assert_eq!(choices[0].display, "0-100");
		assert_eq!(choices[1].value, "100-1000");
		assert_eq!(choices[2].value, "1000-10000");
		assert_eq!(choices[3].value, "10000+");
	}

	#[test]
	fn test_number_range_filter_custom() {
		let filter = NumberRangeFilter::with_ranges(
			"price",
			"Price",
			vec![
				(Some(0), Some(50), "Under $50".to_string()),
				(Some(50), Some(100), "$50-$100".to_string()),
				(Some(100), None, "Over $100".to_string()),
				(None, Some(10), "Less than $10".to_string()),
			],
		);

		let choices = filter.choices();
		assert_eq!(choices.len(), 4);
		assert_eq!(choices[0].value, "0-50");
		assert_eq!(choices[0].display, "Under $50");
		assert_eq!(choices[1].value, "50-100");
		assert_eq!(choices[1].display, "$50-$100");
		assert_eq!(choices[2].value, "100+");
		assert_eq!(choices[2].display, "Over $100");
		assert_eq!(choices[3].value, "-10");
		assert_eq!(choices[3].display, "Less than $10");
	}

	#[test]
	fn test_filter_manager_new() {
		let manager = FilterManager::new();
		assert!(manager.is_empty());
		assert_eq!(manager.filter_count(), 0);
	}

	#[test]
	fn test_filter_manager_add_filter() {
		let manager = FilterManager::new()
			.add_filter(BooleanFilter::new("is_active", "Active"))
			.add_filter(ChoiceFilter::new("status", "Status").add_choice("draft", "Draft"));

		assert_eq!(manager.filter_count(), 2);
		assert!(!manager.is_empty());
	}

	#[test]
	fn test_filter_manager_get_filter() {
		let manager = FilterManager::new()
			.add_filter(BooleanFilter::new("is_active", "Active"))
			.add_filter(ChoiceFilter::new("status", "Status"));

		let filter = manager.get_filter("is_active");
		assert!(filter.is_some());
		assert_eq!(filter.unwrap().field_name(), "is_active");

		let filter = manager.get_filter("nonexistent");
		assert!(filter.is_none());
	}

	#[test]
	fn test_filter_manager_apply_filters() {
		let manager = FilterManager::new()
			.add_filter(BooleanFilter::new("is_active", "Active"))
			.add_filter(ChoiceFilter::new("status", "Status"));

		let mut selected = HashMap::new();
		selected.insert("is_active".to_string(), "true".to_string());
		selected.insert("status".to_string(), "published".to_string());

		let params = manager.apply_filters(&selected);
		assert_eq!(params.len(), 2);
		assert!(params.contains(&"is_active__exact=true".to_string()));
		assert!(params.contains(&"status__exact=published".to_string()));
	}

	#[test]
	fn test_list_filter_is_selected() {
		let filter = BooleanFilter::new("is_active", "Active");

		let mut current = HashMap::new();
		current.insert("is_active".to_string(), "true".to_string());

		assert!(filter.is_selected("true", &current));
		assert!(!filter.is_selected("false", &current));
	}
}
