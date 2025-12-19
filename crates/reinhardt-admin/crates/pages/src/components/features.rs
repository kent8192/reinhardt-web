//! Feature-specific components
//!
//! Provides feature-specific UI components:
//! - `Dashboard` - Dashboard view
//! - `ListView` - List view with filters and pagination
//! - `DetailView` - Detail view for a single record
//! - `ModelForm` - Form for creating/editing records
//! - `Filters` - Filter panel
//! - `DataTable` - Data table component

use reinhardt_pages::component::{ElementView, IntoView, View};

/// Model information for dashboard cards
#[derive(Debug, Clone)]
pub struct DashboardModel {
	/// Model name (display name)
	pub name: String,
	/// URL path for the model list view
	pub url: String,
}

/// Dashboard component
///
/// Displays the admin dashboard with model cards.
///
/// # Example
///
/// ```ignore
/// use reinhardt_admin_pages::components::features::{dashboard, DashboardModel};
///
/// let models = vec![
///     DashboardModel { name: "Users".to_string(), url: "/admin/users/".to_string() },
///     DashboardModel { name: "Posts".to_string(), url: "/admin/posts/".to_string() },
/// ];
/// dashboard("My Admin Panel", &models)
/// ```
pub fn dashboard(site_name: &str, models: &[DashboardModel]) -> View {
	ElementView::new("div")
		.attr("class", "dashboard")
		.child(
			ElementView::new("h1")
				.attr("class", "mb-4")
				.child(format!("{} Dashboard", site_name)),
		)
		.child(
			ElementView::new("div")
				.attr("class", "row")
				.child(models_grid(models)),
		)
		.into_view()
}

/// Generates a grid of model cards
fn models_grid(models: &[DashboardModel]) -> View {
	if models.is_empty() {
		return ElementView::new("div")
			.attr("class", "col-12")
			.child(
				ElementView::new("div")
					.attr("class", "alert alert-info")
					.child("No models registered. Add models to AdminSite to see them here."),
			)
			.into_view();
	}

	let card_views: Vec<View> = models
		.iter()
		.map(|model| {
			ElementView::new("div")
				.attr("class", "col-md-4")
				.child(model_card(&model.name, &model.url))
				.into_view()
		})
		.collect();

	ElementView::new("div")
		.attr("class", "col-12")
		.child(
			ElementView::new("div")
				.attr("class", "row g-4")
				.children(card_views),
		)
		.into_view()
}

/// Generates a single model card
fn model_card(name: &str, url: &str) -> View {
	ElementView::new("div")
		.attr("class", "card h-100")
		.child(
			ElementView::new("div")
				.attr("class", "card-body")
				.child(
					ElementView::new("h5")
						.attr("class", "card-title")
						.child(name),
				)
				.child(
					ElementView::new("p")
						.attr("class", "card-text")
						.child(format!("Manage {} records", name)),
				)
				.child(
					ElementView::new("a")
						.attr("class", "btn btn-primary")
						.attr("href", url)
						.child(format!("View {}", name)),
				),
		)
		.into_view()
}

/// Column definition for list view
#[derive(Debug, Clone)]
pub struct Column {
	/// Column field name
	pub field: String,
	/// Column display label
	pub label: String,
	/// Whether this column is sortable
	pub sortable: bool,
}

/// List view data structure
#[derive(Debug, Clone)]
pub struct ListViewData {
	/// Model name
	pub model_name: String,
	/// Column definitions
	pub columns: Vec<Column>,
	/// Record data (each record is a HashMap of field -> value)
	pub records: Vec<std::collections::HashMap<String, String>>,
	/// Current page number (1-indexed)
	pub current_page: u64,
	/// Total number of pages
	pub total_pages: u64,
	/// Total number of records
	pub total_count: u64,
}

/// List view component
///
/// Displays a paginated list of records with filters and search.
///
/// # Example
///
/// ```ignore
/// use reinhardt_admin_pages::components::features::{list_view, ListViewData, Column};
/// use reinhardt_pages::Signal;
///
/// let data = ListViewData {
///     model_name: "User".to_string(),
///     columns: vec![
///         Column { field: "id".to_string(), label: "ID".to_string(), sortable: true },
///         Column { field: "username".to_string(), label: "Username".to_string(), sortable: true },
///     ],
///     records: vec![/* ... */],
///     current_page: 1,
///     total_pages: 5,
///     total_count: 42,
/// };
/// let page_signal = Signal::new(1u64);
/// list_view(&data, page_signal)
/// ```
pub fn list_view(data: &ListViewData, current_page_signal: reinhardt_pages::Signal<u64>) -> View {
	ElementView::new("div")
		.attr("class", "list-view")
		.child(
			ElementView::new("h1")
				.attr("class", "mb-4")
				.child(format!("{} List", data.model_name)),
		)
		.child(ElementView::new("div").attr("class", "mb-3").child(format!(
			"Showing {} {} (Page {} of {})",
			data.total_count, data.model_name, data.current_page, data.total_pages
		)))
		.child(data_table(&data.columns, &data.records, &data.model_name))
		.child(super::super::components::common::pagination(
			current_page_signal,
			data.total_pages,
		))
		.into_view()
}

/// Generates a data table
fn data_table(
	columns: &[Column],
	records: &[std::collections::HashMap<String, String>],
	model_name: &str,
) -> View {
	// Table header
	let header_cells: Vec<View> = columns
		.iter()
		.map(|col| ElementView::new("th").child(col.label.clone()).into_view())
		.chain(std::iter::once(
			ElementView::new("th").child("Actions").into_view(),
		))
		.collect();

	let thead = ElementView::new("thead").child(ElementView::new("tr").children(header_cells));

	// Table body
	let body_rows: Vec<View> = records
		.iter()
		.map(|record| table_row(columns, record, model_name))
		.collect();

	let tbody = ElementView::new("tbody").children(body_rows);

	ElementView::new("div")
		.attr("class", "table-responsive")
		.child(
			ElementView::new("table")
				.attr("class", "table table-striped table-hover")
				.child(thead)
				.child(tbody),
		)
		.into_view()
}

/// Generates a table row for a single record
fn table_row(
	columns: &[Column],
	record: &std::collections::HashMap<String, String>,
	model_name: &str,
) -> View {
	// Data cells
	let data_cells: Vec<View> = columns
		.iter()
		.map(|col| {
			let value = record.get(&col.field).map(|s| s.as_str()).unwrap_or("-");
			ElementView::new("td").child(value).into_view()
		})
		.collect();

	// Actions cell
	let record_id = record.get("id").map(|s| s.as_str()).unwrap_or("0");
	let actions_cell = ElementView::new("td")
		.child(action_buttons(model_name, record_id))
		.into_view();

	ElementView::new("tr")
		.children(data_cells)
		.child(actions_cell)
		.into_view()
}

/// Generates action buttons for a record
fn action_buttons(model_name: &str, record_id: &str) -> View {
	use reinhardt_pages::component::Component;
	use reinhardt_pages::router::Link;

	let detail_url = format!("/admin/{}/{}/", model_name.to_lowercase(), record_id);
	let edit_url = format!("/admin/{}/{}/change/", model_name.to_lowercase(), record_id);

	ElementView::new("div")
		.attr("class", "btn-group btn-group-sm")
		.attr("role", "group")
		.child(
			Link::new(detail_url.clone(), "View")
				.class("btn btn-outline-primary")
				.render(),
		)
		.child(
			Link::new(edit_url.clone(), "Edit")
				.class("btn btn-outline-secondary")
				.render(),
		)
		.into_view()
}

/// Form field definition for model forms
#[derive(Debug, Clone)]
pub struct FormField {
	/// Field name (corresponds to database column)
	pub name: String,
	/// Field display label
	pub label: String,
	/// HTML input type (text, email, number, etc.)
	pub field_type: String,
	/// Whether this field is required
	pub required: bool,
	/// Current field value (for edit forms)
	pub value: String,
}

/// Detail view component
///
/// Displays detailed information about a single record.
///
/// # Example
///
/// ```ignore
/// use reinhardt_admin_pages::components::features::detail_view;
/// use std::collections::HashMap;
///
/// let mut record = HashMap::new();
/// record.insert("id".to_string(), "1".to_string());
/// record.insert("username".to_string(), "john_doe".to_string());
/// detail_view("User", "1", &record)
/// ```
pub fn detail_view(
	model_name: &str,
	record_id: &str,
	record: &std::collections::HashMap<String, String>,
) -> View {
	use reinhardt_pages::component::Component;
	use reinhardt_pages::router::Link;

	let edit_url = format!("/admin/{}/{}/change/", model_name.to_lowercase(), record_id);
	let list_url = format!("/admin/{}/", model_name.to_lowercase());

	ElementView::new("div")
		.attr("class", "detail-view")
		.child(
			ElementView::new("h1")
				.attr("class", "mb-4")
				.child(format!("{} Detail", model_name)),
		)
		.child(detail_table(record))
		.child(
			ElementView::new("div")
				.attr("class", "mt-4")
				.child(
					Link::new(edit_url, "Edit")
						.class("btn btn-primary me-2")
						.render(),
				)
				.child(
					Link::new(list_url, "Back to List")
						.class("btn btn-secondary")
						.render(),
				),
		)
		.into_view()
}

/// Generates a detail table for record fields
fn detail_table(record: &std::collections::HashMap<String, String>) -> View {
	let rows: Vec<View> = record
		.iter()
		.map(|(key, value)| {
			ElementView::new("tr")
				.child(
					ElementView::new("th")
						.attr("class", "w-25")
						.child(key.clone()),
				)
				.child(ElementView::new("td").child(value.clone()))
				.into_view()
		})
		.collect();

	ElementView::new("div")
		.attr("class", "table-responsive")
		.child(
			ElementView::new("table")
				.attr("class", "table table-bordered")
				.child(ElementView::new("tbody").children(rows)),
		)
		.into_view()
}

/// Model form component
///
/// Displays a form for creating or editing a record.
///
/// # Example
///
/// ```ignore
/// use reinhardt_admin_pages::components::features::{model_form, FormField};
///
/// let fields = vec![
///     FormField {
///         name: "username".to_string(),
///         label: "Username".to_string(),
///         field_type: "text".to_string(),
///         required: true,
///         value: "".to_string(),
///     },
/// ];
/// model_form("User", &fields, None)
/// ```
pub fn model_form(model_name: &str, fields: &[FormField], record_id: Option<&str>) -> View {
	use reinhardt_pages::component::Component;
	use reinhardt_pages::router::Link;

	let form_title = if record_id.is_some() {
		format!("Edit {}", model_name)
	} else {
		format!("Create {}", model_name)
	};

	let list_url = format!("/admin/{}/", model_name.to_lowercase());

	// Add form fields
	let form_groups: Vec<View> = fields.iter().map(|field| form_group(field)).collect();

	ElementView::new("div")
		.attr("class", "model-form")
		.child(
			ElementView::new("h1")
				.attr("class", "mb-4")
				.child(form_title),
		)
		.child(
			ElementView::new("form")
				.attr("class", "needs-validation")
				.attr("novalidate", "true")
				.children(form_groups)
				.child(
					ElementView::new("div")
						.attr("class", "mt-4")
						.child(
							ElementView::new("button")
								.attr("class", "btn btn-primary me-2")
								.attr("type", "submit")
								.child("Save"),
						)
						.child(
							Link::new(list_url, "Cancel")
								.class("btn btn-secondary")
								.render(),
						),
				),
		)
		.into_view()
}

/// Generates a form group (label + input) for a field
fn form_group(field: &FormField) -> View {
	let input_id = format!("field-{}", field.name);

	ElementView::new("div")
		.attr("class", "mb-3")
		.child(
			ElementView::new("label")
				.attr("class", "form-label")
				.attr("for", input_id.clone())
				.child(field.label.clone()),
		)
		.child(form_element(field, &input_id))
		.into_view()
}

/// Generates an input element for a form field
fn form_element(field: &FormField, input_id: &str) -> View {
	let mut input_builder = ElementView::new("input")
		.attr("class", "form-control")
		.attr("type", field.field_type.clone())
		.attr("id", input_id.to_string())
		.attr("name", field.name.clone())
		.attr("value", field.value.clone());

	if field.required {
		input_builder = input_builder.attr("required", "true");
	}

	input_builder.into_view()
}

/// Filters component
///
/// Displays filter controls for list views.
pub fn filters() {
	// TODO: Implement Filters component
	todo!("Implement Filters component using reinhardt-pages view! macro and Signal")
}
