//! API endpoint implementations

use super::client::{ApiClient, ApiError};
use reinhardt_admin_types::{
	DashboardResponse, DetailResponse, ListQueryParams, ListResponse, MutationRequest,
	MutationResponse,
};

impl ApiClient {
	/// Get dashboard information (list of registered models)
	pub async fn get_dashboard(&self) -> Result<DashboardResponse, ApiError> {
		self.get("/").await
	}

	/// List model instances with pagination, search, and filters
	pub async fn list_models(
		&self,
		model: &str,
		params: &ListQueryParams,
	) -> Result<ListResponse, ApiError> {
		let mut query_parts = Vec::new();

		if let Some(page) = params.page {
			query_parts.push(format!("page={}", page));
		}

		if let Some(page_size) = params.page_size {
			query_parts.push(format!("page_size={}", page_size));
		}

		if let Some(ref search) = params.search {
			query_parts.push(format!("search={}", urlencoding::encode(search)));
		}

		for (key, value) in &params.filters {
			query_parts.push(format!("{}={}", key, urlencoding::encode(value)));
		}

		let query_string = if query_parts.is_empty() {
			String::new()
		} else {
			format!("?{}", query_parts.join("&"))
		};

		self.get(&format!("/{}/{}", model, query_string)).await
	}

	/// Get a single model instance by ID
	pub async fn get_detail(&self, model: &str, id: &str) -> Result<DetailResponse, ApiError> {
		self.get(&format!("/{}/{}/", model, id)).await
	}

	/// Create a new model instance
	pub async fn create(
		&self,
		model: &str,
		data: &MutationRequest,
	) -> Result<MutationResponse, ApiError> {
		self.post(&format!("/{}/", model), data).await
	}

	/// Update an existing model instance
	pub async fn update(
		&self,
		model: &str,
		id: &str,
		data: &MutationRequest,
	) -> Result<MutationResponse, ApiError> {
		self.put(&format!("/{}/{}/", model, id), data).await
	}
}
