//! Concrete endpoints and a ViewSet used by the scaffold routing examples.

use async_trait::async_trait;
use reinhardt::core::exception::Result;
use reinhardt::views::viewsets::{Action, ViewSet};
use reinhardt::{Request, Response, get};

#[get("/health/", name = "health-check")]
pub async fn health_check() -> Result<Response> {
	Ok(Response::ok().with_body("healthy"))
}

pub struct UserViewSet;

impl UserViewSet {
	pub fn new() -> Self {
		Self
	}
}

#[async_trait]
impl ViewSet for UserViewSet {
	fn get_basename(&self) -> &str {
		"users"
	}

	async fn dispatch(&self, _request: Request, action: Action) -> Result<Response> {
		Ok(Response::ok().with_body(format!("{:?}", action.action_type)))
	}
}
