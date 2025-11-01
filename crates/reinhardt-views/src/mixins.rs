//! Mixins for common view patterns.

use async_trait::async_trait;
use reinhardt_apps::Request;
use reinhardt_exception::Result;
use reinhardt_orm::Model;
use serde::Serialize;
use serde_json::json;

use crate::core::Context;

/// Trait for views that work with multiple objects
#[async_trait]
pub trait MultipleObjectMixin<T>: Send + Sync
where
	T: Model + Serialize + Send + Sync + Clone,
{
	/// Get objects for this view
	async fn get_objects(&self) -> Result<Vec<T>>;

	/// Get the ordering for the queryset
	fn get_ordering(&self) -> Option<Vec<String>> {
		None
	}

	/// Whether to allow empty result sets
	fn allow_empty(&self) -> bool {
		true
	}

	/// Get the number of items per page
	fn get_paginate_by(&self) -> Option<usize> {
		None
	}

	/// Get the context object name
	fn get_context_object_name(&self) -> Option<&str> {
		None
	}

	/// Build context data for the view
	fn get_context_data(&self, object_list: Vec<T>) -> Result<Context> {
		let mut context = Context::new();
		context.insert("object_list".to_string(), json!(object_list));

		if let Some(name) = self.get_context_object_name() {
			context.insert(name.to_string(), json!(object_list));
		}

		Ok(context)
	}
}

/// Trait for views that work with a single object
#[async_trait]
pub trait SingleObjectMixin<T>: Send + Sync
where
	T: Model + Serialize + Send + Sync + Clone,
{
	/// Get the slug field name
	fn get_slug_field(&self) -> &str {
		"slug"
	}

	/// Get the primary key URL parameter name
	fn pk_url_kwarg(&self) -> &str {
		"pk"
	}

	/// Get the slug URL parameter name
	fn slug_url_kwarg(&self) -> &str {
		"slug"
	}

	/// Get a single object
	async fn get_object(&self, request: &Request) -> Result<T>;

	/// Get the context object name
	fn get_context_object_name(&self) -> Option<&str> {
		None
	}

	/// Build context data for the view
	fn get_context_data(&self, object: T) -> Result<Context> {
		let mut context = Context::new();
		context.insert("object".to_string(), json!(object));

		if let Some(name) = self.get_context_object_name() {
			context.insert(name.to_string(), json!(object));
		}

		Ok(context)
	}
}
