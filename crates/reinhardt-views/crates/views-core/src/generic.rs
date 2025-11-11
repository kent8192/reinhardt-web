//! Generic API views

use async_trait::async_trait;
use reinhardt_core::apps::{Request, Response};
use reinhardt_core::exception::Result;

/// Base trait for all generic views
#[async_trait]
pub trait View: Send + Sync {
	async fn dispatch(&self, request: Request) -> Result<Response>;

	/// Returns the list of HTTP methods allowed by this view
	fn allowed_methods(&self) -> Vec<&'static str> {
		vec!["GET", "HEAD", "OPTIONS"]
	}
}

pub struct ListAPIView;

impl ListAPIView {
	pub fn new() -> Self {
		Self
	}
}

impl Default for ListAPIView {
	fn default() -> Self {
		Self::new()
	}
}

pub struct CreateAPIView;

impl CreateAPIView {
	pub fn new() -> Self {
		Self
	}
}

impl Default for CreateAPIView {
	fn default() -> Self {
		Self::new()
	}
}

pub struct UpdateAPIView;

impl UpdateAPIView {
	pub fn new() -> Self {
		Self
	}
}

impl Default for UpdateAPIView {
	fn default() -> Self {
		Self::new()
	}
}

pub struct DestroyAPIView;

impl DestroyAPIView {
	pub fn new() -> Self {
		Self
	}
}

impl Default for DestroyAPIView {
	fn default() -> Self {
		Self::new()
	}
}

pub struct ListCreateAPIView;

impl ListCreateAPIView {
	pub fn new() -> Self {
		Self
	}
}

impl Default for ListCreateAPIView {
	fn default() -> Self {
		Self::new()
	}
}

pub struct RetrieveUpdateAPIView;

impl RetrieveUpdateAPIView {
	pub fn new() -> Self {
		Self
	}
}

impl Default for RetrieveUpdateAPIView {
	fn default() -> Self {
		Self::new()
	}
}

pub struct RetrieveDestroyAPIView;

impl RetrieveDestroyAPIView {
	pub fn new() -> Self {
		Self
	}
}

impl Default for RetrieveDestroyAPIView {
	fn default() -> Self {
		Self::new()
	}
}

pub struct RetrieveUpdateDestroyAPIView;

impl RetrieveUpdateDestroyAPIView {
	pub fn new() -> Self {
		Self
	}
}

impl Default for RetrieveUpdateDestroyAPIView {
	fn default() -> Self {
		Self::new()
	}
}
