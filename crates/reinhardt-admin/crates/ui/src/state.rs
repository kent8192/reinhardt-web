//! Global application state

use crate::api::ApiClient;
use crate::components::common::ConfirmModalState;
use futures_signals::signal::Mutable;
use futures_signals::signal_vec::MutableVec;
use reinhardt_admin_types::{
	ColumnInfo, DashboardResponse, FieldInfo, FilterInfo, ListQueryParams, ModelInfo,
	MutationRequest, MutationResponse,
};
use std::collections::HashMap;
use std::sync::Arc;
use wasm_bindgen_futures::spawn_local;

/// Global application state
pub struct AppState {
	/// API client for backend communication
	pub api_client: Arc<ApiClient>,
	/// Site name
	pub site_name: Mutable<String>,
	/// URL prefix for admin
	pub url_prefix: Mutable<String>,
	/// List of registered models
	pub models: MutableVec<ModelInfo>,
	/// Active list view states (model_name -> ListViewState)
	pub list_views: Mutable<HashMap<String, Arc<ListViewState>>>,
	/// Active detail view states (key: model_name:id)
	pub detail_views: Mutable<HashMap<String, Arc<DetailViewState>>>,
	/// Active form states (key: model_name:mode:id)
	pub forms: Mutable<HashMap<String, Arc<FormState>>>,
	/// Error messages
	pub error: Mutable<Option<String>>,
	/// Loading state
	pub is_loading: Mutable<bool>,
}

impl AppState {
	/// Create a new application state
	pub fn new(base_url: String) -> Arc<Self> {
		Arc::new(Self {
			api_client: Arc::new(ApiClient::new(base_url)),
			site_name: Mutable::new("Admin Panel".to_string()),
			url_prefix: Mutable::new("/admin".to_string()),
			models: MutableVec::new(),
			list_views: Mutable::new(HashMap::new()),
			detail_views: Mutable::new(HashMap::new()),
			forms: Mutable::new(HashMap::new()),
			error: Mutable::new(None),
			is_loading: Mutable::new(false),
		})
	}

	/// Load dashboard data from the API
	pub fn load_dashboard(self: &Arc<Self>) {
		let state = Arc::clone(self);

		spawn_local(async move {
			state.is_loading.set(true);
			state.error.set(None);

			match state.api_client.get_dashboard().await {
				Ok(dashboard) => {
					state.handle_dashboard_response(dashboard);
				}
				Err(err) => {
					state
						.error
						.set(Some(format!("Failed to load dashboard: {}", err.message)));
				}
			}

			state.is_loading.set(false);
		});
	}

	/// Handle dashboard response
	fn handle_dashboard_response(&self, response: DashboardResponse) {
		self.site_name.set(response.site_name);
		self.url_prefix.set(response.url_prefix);

		// Clear and populate models
		self.models.lock_mut().clear();
		for model in response.models {
			self.models.lock_mut().push_cloned(model);
		}

		// Extract and set CSRF token if provided
		if let Some(csrf_token) = response.csrf_token {
			self.api_client.set_csrf_token(csrf_token);
		}
	}

	/// Get or create list view state for a model
	pub fn get_list_view_state(&self, model_name: &str) -> Arc<ListViewState> {
		let mut views = self.list_views.lock_mut();

		if let Some(state) = views.get(model_name) {
			Arc::clone(state)
		} else {
			let state = ListViewState::new(model_name.to_string(), Arc::clone(&self.api_client));
			views.insert(model_name.to_string(), Arc::clone(&state));
			state
		}
	}

	/// Get or create detail view state for a model instance
	pub fn get_detail_view_state(&self, model_name: &str, id: &str) -> Arc<DetailViewState> {
		let key = format!("{}:{}", model_name, id);
		let mut views = self.detail_views.lock_mut();

		if let Some(state) = views.get(&key) {
			Arc::clone(state)
		} else {
			let state = DetailViewState::new(
				model_name.to_string(),
				id.to_string(),
				Arc::clone(&self.api_client),
			);
			views.insert(key, Arc::clone(&state));
			state
		}
	}

	/// Get or create form state for creating a model
	pub fn get_create_form_state(&self, model_name: &str) -> Arc<FormState> {
		let key = format!("{}:create", model_name);
		let mut forms = self.forms.lock_mut();

		if let Some(state) = forms.get(&key) {
			Arc::clone(state)
		} else {
			let state = FormState::new_create(model_name.to_string(), Arc::clone(&self.api_client));
			forms.insert(key, Arc::clone(&state));
			state
		}
	}

	/// Get or create form state for editing a model instance
	pub fn get_edit_form_state(&self, model_name: &str, id: &str) -> Arc<FormState> {
		let key = format!("{}:edit:{}", model_name, id);
		let mut forms = self.forms.lock_mut();

		if let Some(state) = forms.get(&key) {
			Arc::clone(state)
		} else {
			let state = FormState::new_edit(
				model_name.to_string(),
				id.to_string(),
				Arc::clone(&self.api_client),
			);
			forms.insert(key, Arc::clone(&state));
			state
		}
	}
}

/// List view state for a specific model
pub struct ListViewState {
	/// Model name
	pub model_name: String,
	/// Current page (1-indexed)
	pub current_page: Mutable<u64>,
	/// Page size
	pub page_size: Mutable<u64>,
	/// Total count
	pub total_count: Mutable<u64>,
	/// Total pages
	pub total_pages: Mutable<u64>,
	/// Search query
	pub search_query: Mutable<String>,
	/// Current sort field and direction (e.g., "-created_at" for descending)
	pub sort_by: Mutable<Option<String>>,
	/// Filter key-value pairs
	pub filters: Mutable<HashMap<String, String>>,
	/// Available filters metadata from API
	pub available_filters: MutableVec<FilterInfo>,
	/// Column definitions from API
	pub columns: MutableVec<ColumnInfo>,
	/// List items
	pub items: MutableVec<HashMap<String, serde_json::Value>>,
	/// Loading state
	pub is_loading: Mutable<bool>,
	/// Error message
	pub error: Mutable<Option<String>>,
	/// Confirmation modal state
	pub confirm_modal: Arc<ConfirmModalState>,
	/// API client reference
	api_client: Arc<ApiClient>,
}

impl ListViewState {
	/// Create a new list view state
	pub fn new(model_name: String, api_client: Arc<ApiClient>) -> Arc<Self> {
		Arc::new(Self {
			model_name,
			current_page: Mutable::new(1),
			page_size: Mutable::new(25),
			total_count: Mutable::new(0),
			total_pages: Mutable::new(0),
			search_query: Mutable::new(String::new()),
			sort_by: Mutable::new(None),
			filters: Mutable::new(HashMap::new()),
			available_filters: MutableVec::new(),
			columns: MutableVec::new(),
			items: MutableVec::new(),
			is_loading: Mutable::new(false),
			error: Mutable::new(None),
			confirm_modal: ConfirmModalState::new(),
			api_client,
		})
	}

	/// Load data from the API
	pub fn load_data(self: &Arc<Self>) {
		let state = Arc::clone(self);

		spawn_local(async move {
			state.is_loading.set(true);
			state.error.set(None);

			// Build query parameters
			let params = ListQueryParams {
				page: Some(state.current_page.get()),
				page_size: Some(state.page_size.get()),
				search: if state.search_query.get_cloned().is_empty() {
					None
				} else {
					Some(state.search_query.get_cloned())
				},
				sort_by: state.sort_by.get_cloned(),
				filters: state.filters.lock_ref().clone(),
			};

			match state
				.api_client
				.list_models(&state.model_name, &params)
				.await
			{
				Ok(response) => {
					// Update pagination info
					state.total_count.set(response.count);
					state.total_pages.set(response.total_pages);

					// Update available filters
					state.available_filters.lock_mut().clear();
					if let Some(filters) = response.available_filters {
						for filter in filters {
							state.available_filters.lock_mut().push_cloned(filter);
						}
					}

					// Update column definitions
					state.columns.lock_mut().clear();
					if let Some(columns) = response.columns {
						for column in columns {
							state.columns.lock_mut().push_cloned(column);
						}
					}

					// Update items
					state.items.lock_mut().clear();
					for item in response.results {
						state.items.lock_mut().push_cloned(item);
					}
				}
				Err(err) => {
					state
						.error
						.set(Some(format!("Failed to load data: {}", err.message)));
				}
			}

			state.is_loading.set(false);
		});
	}

	/// Go to a specific page
	pub fn goto_page(self: &Arc<Self>, page: u64) {
		self.current_page.set(page);
		self.load_data();
	}

	/// Set search query and reload
	pub fn set_search(self: &Arc<Self>, query: String) {
		self.search_query.set(query);
		self.current_page.set(1); // Reset to first page
		self.load_data();
	}

	/// Set a filter value
	pub fn set_filter(self: &Arc<Self>, key: String, value: String) {
		self.filters.lock_mut().insert(key, value);
		self.current_page.set(1); // Reset to first page
		self.load_data();
	}

	/// Clear a specific filter
	pub fn clear_filter(self: &Arc<Self>, key: &str) {
		self.filters.lock_mut().remove(key);
		self.current_page.set(1);
		self.load_data();
	}

	/// Set sort field and direction
	pub fn set_sort(self: &Arc<Self>, field: String, descending: bool) {
		let sort_str = if descending {
			format!("-{}", field)
		} else {
			field
		};
		self.sort_by.set(Some(sort_str));
		self.load_data();
	}

	/// Set page size and reload
	pub fn set_page_size(self: &Arc<Self>, size: u64) {
		self.page_size.set(size);
		self.current_page.set(1); // Reset to first page
		self.load_data();
	}

	/// Delete a specific item
	///
	/// This method sends a DELETE request to the API and reloads the data on success.
	pub fn delete_item(self: &Arc<Self>, id: String) {
		let state = Arc::clone(self);

		spawn_local(async move {
			state.is_loading.set(true);
			state.error.set(None);

			let path = format!("/{}/{}/", state.model_name, id);
			match state.api_client.delete::<MutationResponse>(&path).await {
				Ok(_) => {
					// Reload data to reflect the deletion
					state.load_data();
				}
				Err(err) => {
					state
						.error
						.set(Some(format!("Failed to delete: {}", err.message)));
					state.is_loading.set(false);
				}
			}
		});
	}

	/// Show delete confirmation modal
	///
	/// Displays a confirmation dialog before deleting an item.
	pub fn show_delete_confirmation(self: &Arc<Self>, id: String, item_name: Option<String>) {
		let state = Arc::clone(self);
		let display_name = item_name.unwrap_or_else(|| format!("ID: {}", id));

		self.confirm_modal.show(
			format!("Delete {}?", self.model_name),
			format!(
				"Are you sure you want to delete \"{}\"? This action cannot be undone.",
				display_name
			),
			move || {
				state.delete_item(id.clone());
			},
		);
	}
}

/// Detail view state for a specific model instance
pub struct DetailViewState {
	/// Model name
	pub model_name: String,
	/// Instance ID
	pub id: String,
	/// Instance data
	pub data: Mutable<Option<HashMap<String, serde_json::Value>>>,
	/// Loading state
	pub is_loading: Mutable<bool>,
	/// Error message
	pub error: Mutable<Option<String>>,
	/// API client reference
	api_client: Arc<ApiClient>,
}

impl DetailViewState {
	/// Create a new detail view state
	pub fn new(model_name: String, id: String, api_client: Arc<ApiClient>) -> Arc<Self> {
		Arc::new(Self {
			model_name,
			id,
			data: Mutable::new(None),
			is_loading: Mutable::new(false),
			error: Mutable::new(None),
			api_client,
		})
	}

	/// Load data from the API
	pub fn load_data(self: &Arc<Self>) {
		let state = Arc::clone(self);

		spawn_local(async move {
			state.is_loading.set(true);
			state.error.set(None);

			match state
				.api_client
				.get_detail(&state.model_name, &state.id)
				.await
			{
				Ok(response) => {
					state.data.set(Some(response.data));
				}
				Err(err) => {
					state
						.error
						.set(Some(format!("Failed to load data: {}", err.message)));
				}
			}

			state.is_loading.set(false);
		});
	}

	/// Delete the current item
	pub fn delete_item<F>(self: &Arc<Self>, on_success: F)
	where
		F: Fn() + 'static,
	{
		let state = Arc::clone(self);

		spawn_local(async move {
			state.is_loading.set(true);
			state.error.set(None);

			let path = format!("/{}/{}/", state.model_name, state.id);
			match state.api_client.delete::<MutationResponse>(&path).await {
				Ok(_) => {
					on_success();
				}
				Err(err) => {
					state
						.error
						.set(Some(format!("Failed to delete: {}", err.message)));
				}
			}

			state.is_loading.set(false);
		});
	}

	/// Clear the current error
	pub fn clear_error(&self) {
		self.error.set(None);
	}
}

/// Form mode (Create or Edit)
#[derive(Debug, Clone, PartialEq)]
pub enum FormMode {
	Create,
	Edit,
}

/// Form state for creating or editing a model instance
pub struct FormState {
	/// Model name
	pub model_name: String,
	/// Form mode (Create or Edit)
	pub mode: FormMode,
	/// Instance ID (for Edit mode)
	pub id: Option<String>,
	/// Form data (field_name -> value)
	pub data: Mutable<HashMap<String, serde_json::Value>>,
	/// Field-level errors (field_name -> error_message)
	pub field_errors: Mutable<HashMap<String, String>>,
	/// Form-level error
	pub form_error: Mutable<Option<String>>,
	/// Loading state
	pub is_loading: Mutable<bool>,
	/// API client reference
	api_client: Arc<ApiClient>,
}

impl FormState {
	/// Create a new form state for creating a model
	pub fn new_create(model_name: String, api_client: Arc<ApiClient>) -> Arc<Self> {
		Arc::new(Self {
			model_name,
			mode: FormMode::Create,
			id: None,
			data: Mutable::new(HashMap::new()),
			field_errors: Mutable::new(HashMap::new()),
			form_error: Mutable::new(None),
			is_loading: Mutable::new(false),
			api_client,
		})
	}

	/// Create a new form state for editing a model instance
	pub fn new_edit(model_name: String, id: String, api_client: Arc<ApiClient>) -> Arc<Self> {
		let state = Arc::new(Self {
			model_name,
			mode: FormMode::Edit,
			id: Some(id.clone()),
			data: Mutable::new(HashMap::new()),
			field_errors: Mutable::new(HashMap::new()),
			form_error: Mutable::new(None),
			is_loading: Mutable::new(false),
			api_client: Arc::clone(&api_client),
		});

		// Load existing data
		let load_state = Arc::clone(&state);
		spawn_local(async move {
			load_state.is_loading.set(true);
			match load_state
				.api_client
				.get_detail(&load_state.model_name, &id)
				.await
			{
				Ok(response) => {
					load_state.data.set(response.data);
				}
				Err(err) => {
					load_state
						.form_error
						.set(Some(format!("Failed to load data: {}", err.message)));
				}
			}
			load_state.is_loading.set(false);
		});

		state
	}

	/// Set a field value
	pub fn set_field(&self, field_name: String, value: serde_json::Value) {
		self.data.lock_mut().insert(field_name.clone(), value);
		// Clear field error when user modifies the field
		self.field_errors.lock_mut().remove(&field_name);
	}

	/// Validate the form
	pub fn validate(&self, fields: &[FieldInfo]) -> bool {
		let mut errors = HashMap::new();
		let data = self.data.lock_ref();

		for field in fields {
			if field.required {
				let value = data.get(&field.name);
				let is_empty = match value {
					None => true,
					Some(v) => {
						if let Some(s) = v.as_str() {
							s.trim().is_empty()
						} else {
							false
						}
					}
				};

				if is_empty {
					errors.insert(field.name.clone(), "This field is required".to_string());
				}
			}
		}

		let has_errors = !errors.is_empty();
		self.field_errors.set(errors);
		!has_errors
	}

	/// Submit the form
	pub fn submit<F>(self: &Arc<Self>, fields: &[FieldInfo], on_success: F)
	where
		F: Fn(MutationResponse) + 'static,
	{
		// Client-side validation
		if !self.validate(fields) {
			return;
		}

		let state = Arc::clone(self);
		let data = self.data.lock_ref().clone();

		spawn_local(async move {
			state.is_loading.set(true);
			state.form_error.set(None);

			let request = MutationRequest { data };

			let result = match state.mode {
				FormMode::Create => state.api_client.create(&state.model_name, &request).await,
				FormMode::Edit => {
					let id = state.id.as_ref().unwrap();
					state
						.api_client
						.update(&state.model_name, id, &request)
						.await
				}
			};

			match result {
				Ok(response) => {
					on_success(response);
				}
				Err(err) => {
					state
						.form_error
						.set(Some(format!("Failed to submit: {}", err.message)));
				}
			}

			state.is_loading.set(false);
		});
	}

	/// Clear the current error
	pub fn clear_error(&self) {
		self.form_error.set(None);
	}
}
