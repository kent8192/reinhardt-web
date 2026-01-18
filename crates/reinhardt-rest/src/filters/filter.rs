use async_trait::async_trait;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FilterError {
	#[error("Invalid filter parameter: {0}")]
	InvalidParameter(String),
	#[error("Filter error: {0}")]
	FilterError(String),
	#[error("Invalid query: {0}")]
	InvalidQuery(String),
}

pub type FilterResult<T> = Result<T, FilterError>;

#[async_trait]
pub trait FilterBackend: Send + Sync {
	async fn filter_queryset(
		&self,
		query_params: &HashMap<String, String>,
		sql: String,
	) -> FilterResult<String>;
}
