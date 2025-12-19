//! List view Server Function
//!
//! Provides list view operations for admin models.

use reinhardt_admin_core::{AdminDatabase, AdminRecord, AdminSite, ModelAdmin};
use reinhardt_admin_types::{
	ColumnInfo, FilterChoice, FilterInfo, FilterType, ListQueryParams, ListResponse,
};
use reinhardt_db::orm::{Filter, FilterCondition, FilterOperator, FilterValue};
use reinhardt_pages::server_fn::{ServerFnError, server_fn};
use std::collections::HashMap;
use std::sync::Arc;

use super::error::MapServerFnError;

/// List models with filters and pagination
///
/// Returns a paginated list of model records with optional filtering and search.
///
/// # Server Function
///
/// This function is automatically exposed as an HTTP endpoint by the `#[server_fn]` macro.
/// The AdminSite and AdminDatabase dependencies are automatically injected via the DI system.
/// Uses JSON codec for complex query parameters.
///
/// # Example
///
/// ```ignore
/// use reinhardt_admin_server::list_models;
/// use reinhardt_admin_types::ListQueryParams;
///
/// let params = ListQueryParams {
///     page: Some(1),
///     page_size: Some(50),
///     search: Some("alice".to_string()),
///     ..Default::default()
/// };
///
/// // Client-side usage (automatically generates HTTP request)
/// let response = list_models("users".to_string(), params).await?;
/// println!("Total: {}, Pages: {}", response.count, response.total_pages);
/// ```
#[server_fn(use_inject = true, codec = "json")]
pub async fn list_models(
	model_name: String,
	params: ListQueryParams,
	#[inject] site: Arc<AdminSite>,
	#[inject] db: Arc<AdminDatabase>,
) -> Result<ListResponse, ServerFnError> {
	// Get model configuration
	let model_admin = site.get_model_admin(&model_name).map_server_fn_error()?;

	// Calculate pagination parameters
	let page = params.page.unwrap_or(1).max(1);
	let page_size = params
		.page_size
		.unwrap_or_else(|| model_admin.list_per_page().unwrap_or(100) as u64);
	let offset = (page - 1) * page_size;

	// Build search filter condition (OR across multiple fields)
	let search_condition = if let Some(search) = &params.search {
		let search_fields = model_admin.search_fields();
		if !search_fields.is_empty() {
			// Build OR condition across all search fields
			let search_filters: Vec<Filter> = search_fields
				.iter()
				.map(|field| {
					Filter::new(
						field.to_string(),
						FilterOperator::Contains,
						FilterValue::String(search.clone()),
					)
				})
				.collect();

			// Create OR condition for search (matches any field)
			Some(FilterCondition::or_filters(search_filters))
		} else {
			None
		}
	} else {
		None
	};

	// Build additional filters from query params (AND logic)
	let mut additional_filters = Vec::new();
	let filter_fields = model_admin.list_filter();
	for field in filter_fields {
		if let Some(value) = params.filters.get(field) {
			additional_filters.push(Filter::new(
				field.to_string(),
				FilterOperator::Eq,
				FilterValue::String(value.clone()),
			));
		}
	}

	let table_name = model_admin.table_name();

	// Get total count with conditions
	let count = db
		.count_with_condition::<AdminRecord>(
			table_name,
			search_condition.as_ref(),
			additional_filters.clone(),
		)
		.await
		.map_server_fn_error()?;

	// Get paginated results with conditions
	let results = db
		.list_with_condition::<AdminRecord>(
			table_name,
			search_condition.as_ref(),
			additional_filters,
			params.sort_by.as_deref(),
			offset,
			page_size,
		)
		.await
		.map_server_fn_error()?;

	let total_pages = count.div_ceil(page_size);

	// Generate filter metadata
	let available_filters = generate_filters(&model_name, &params.filters);

	// Generate column info from model admin configuration
	let columns = generate_columns(model_admin.as_ref());

	Ok(ListResponse {
		model_name,
		count,
		page,
		page_size,
		total_pages,
		results,
		available_filters: Some(available_filters),
		columns: Some(columns),
	})
}

/// Convert field name to human-readable label
///
/// Transforms snake_case field names to Title Case labels.
/// Examples: "user_name" -> "User Name", "id" -> "Id"
fn field_to_label(field: &str) -> String {
	field
		.split('_')
		.map(|word| {
			let mut chars = word.chars();
			match chars.next() {
				None => String::new(),
				Some(first) => first.to_uppercase().chain(chars).collect(),
			}
		})
		.collect::<Vec<_>>()
		.join(" ")
}

/// Generate column info from model admin configuration
///
/// Creates column metadata for list view based on list_display() fields.
/// Sortable status is determined by checking if the field is in ordering().
fn generate_columns(model_admin: &dyn ModelAdmin) -> Vec<ColumnInfo> {
	let display_fields = model_admin.list_display();
	let ordering = model_admin.ordering();

	// Extract sortable fields from ordering (strip "-" prefix for descending)
	let sortable_fields: Vec<&str> = ordering
		.iter()
		.map(|f| f.strip_prefix('-').unwrap_or(f))
		.collect();

	display_fields
		.iter()
		.map(|field| ColumnInfo {
			field: field.to_string(),
			label: field_to_label(field),
			sortable: sortable_fields.contains(field),
		})
		.collect()
}

/// Generate filter metadata
///
/// This is a temporary implementation that provides hardcoded filters.
/// In future phases, this will be replaced with dynamic filter generation
/// based on model definitions.
fn generate_filters(
	_model_name: &str,
	active_filters: &HashMap<String, String>,
) -> Vec<FilterInfo> {
	vec![
		FilterInfo {
			field: "status".to_string(),
			title: "Status".to_string(),
			filter_type: FilterType::Choice {
				choices: vec![
					FilterChoice {
						value: "active".to_string(),
						label: "Active".to_string(),
					},
					FilterChoice {
						value: "inactive".to_string(),
						label: "Inactive".to_string(),
					},
				],
			},
			current_value: active_filters.get("status").cloned(),
		},
		FilterInfo {
			field: "is_published".to_string(),
			title: "Published".to_string(),
			filter_type: FilterType::Boolean,
			current_value: active_filters.get("is_published").cloned(),
		},
	]
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_field_to_label() {
		assert_eq!(field_to_label("user_name"), "User Name");
		assert_eq!(field_to_label("id"), "Id");
		assert_eq!(field_to_label("is_active"), "Is Active");
		assert_eq!(field_to_label("created_at"), "Created At");
	}

	#[test]
	fn test_list_response_structure() {
		let response = ListResponse {
			model_name: "User".to_string(),
			count: 100,
			page: 2,
			page_size: 25,
			total_pages: 4,
			results: vec![],
			available_filters: None,
			columns: None,
		};

		assert_eq!(response.model_name, "User");
		assert_eq!(response.count, 100);
		assert_eq!(response.page, 2);
		assert_eq!(response.page_size, 25);
		assert_eq!(response.total_pages, 4);
	}

	#[test]
	fn test_pagination_calculation() {
		// Page 1, size 25: offset = 0
		let page = 1;
		let page_size = 25;
		let offset = (page - 1) * page_size;
		assert_eq!(offset, 0);

		// Page 2, size 25: offset = 25
		let page = 2;
		let offset = (page - 1) * page_size;
		assert_eq!(offset, 25);

		// Total pages: 100 items / 25 per page = 4 pages
		let count = 100;
		let total_pages = count / page_size;
		assert_eq!(total_pages, 4);
	}
}
