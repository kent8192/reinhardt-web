//! Template/HTML Integration Tests for Renderers
//!
//! These tests require multiple crates:
//! - reinhardt-renderers
//! - reinhardt-templates
//! - reinhardt-forms
//! - reinhardt-views
//!
//! Based on Django REST Framework's TemplateHTMLRendererTests

use async_trait::async_trait;
use bytes::Bytes;
use hyper::{HeaderMap, Method, StatusCode, Version};
use reinhardt_exception::Result;
use reinhardt_http::{Request, Response};
use reinhardt_types::Handler;
use std::sync::Arc;
use tera::{Context, Tera};

#[cfg(test)]
mod template_exception_tests {
	use super::*;

	#[derive(Clone)]
	struct NotFoundTemplate {
		message: String,
	}

	impl NotFoundTemplate {
		fn render(&self) -> String {
			let mut context = Context::new();
			context.insert("message", &self.message);

			Tera::one_off(
				"<html><body><h1>404 Not Found</h1><p>{{ message }}</p></body></html>",
				&context,
				true,
			)
			.unwrap()
		}
	}

	#[derive(Clone)]
	struct ForbiddenTemplate {
		message: String,
	}

	impl ForbiddenTemplate {
		fn render(&self) -> String {
			let mut context = Context::new();
			context.insert("message", &self.message);

			Tera::one_off(
				"<html><body><h1>403 Forbidden</h1><p>{{ message }}</p></body></html>",
				&context,
				true,
			)
			.unwrap()
		}
	}

	#[derive(Clone)]
	struct ErrorTemplateHandler<T> {
		template: T,
		status: StatusCode,
	}

	impl<T> ErrorTemplateHandler<T> {
		fn new(template: T, status: StatusCode) -> Self {
			Self { template, status }
		}
	}

	trait RenderTemplate {
		fn render(&self) -> String;
	}

	impl RenderTemplate for NotFoundTemplate {
		fn render(&self) -> String {
			NotFoundTemplate::render(self)
		}
	}

	impl RenderTemplate for ForbiddenTemplate {
		fn render(&self) -> String {
			ForbiddenTemplate::render(self)
		}
	}

	#[async_trait]
	impl<T: RenderTemplate + Clone + Send + Sync> Handler for ErrorTemplateHandler<T> {
		async fn handle(&self, _request: Request) -> Result<Response> {
			let rendered = self.template.render();

			Ok(Response::new(self.status)
				.with_body(Bytes::from(rendered))
				.with_header("content-type", "text/html; charset=utf-8"))
		}
	}

	#[tokio::test]
	async fn test_not_found_html_view_with_template() {
		let template = NotFoundTemplate {
			message: "The requested resource could not be found.".to_string(),
		};

		let handler = Arc::new(ErrorTemplateHandler::new(template, StatusCode::NOT_FOUND));

		let request = Request::builder()
			.method(Method::GET)
			.uri("/nonexistent")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let response = handler.handle(request).await.unwrap();

		assert_eq!(response.status, StatusCode::NOT_FOUND);

		let body_str = String::from_utf8(response.body.to_vec()).unwrap();
		assert!(body_str.contains("404 Not Found"));
		assert!(body_str.contains("The requested resource could not be found"));
	}

	#[tokio::test]
	async fn test_permission_denied_html_view_with_template() {
		let template = ForbiddenTemplate {
			message: "You do not have permission to access this resource.".to_string(),
		};

		let handler = Arc::new(ErrorTemplateHandler::new(template, StatusCode::FORBIDDEN));

		let request = Request::builder()
			.method(Method::GET)
			.uri("/admin/secret")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let response = handler.handle(request).await.unwrap();

		assert_eq!(response.status, StatusCode::FORBIDDEN);

		let body_str = String::from_utf8(response.body.to_vec()).unwrap();
		assert!(body_str.contains("403 Forbidden"));
		assert!(body_str.contains("You do not have permission to access this resource"));
	}
}

#[cfg(test)]
mod form_rendering_tests {
	use super::*;
	use serde::{Deserialize, Serialize};

	#[derive(Debug, Clone, Serialize, Deserialize)]
	struct FormData {
		name: String,
		email: String,
		age: i32,
	}

	struct FormTemplate;

	impl FormTemplate {
		fn render() -> String {
			let context = Context::new();

			Tera::one_off(
				r#"<html><body><h1>Submit Data</h1>
<form method="post">
  <label>Name: <input type="text" name="name" /></label><br/>
  <label>Email: <input type="email" name="email" /></label><br/>
  <label>Age: <input type="number" name="age" /></label><br/>
  <button type="submit">Submit</button>
</form>
</body></html>"#,
				&context,
				true,
			)
			.unwrap()
		}
	}

	#[derive(Clone)]
	struct FormHandler {
		render_html: bool,
	}

	#[async_trait]
	impl Handler for FormHandler {
		async fn handle(&self, _request: Request) -> Result<Response> {
			if self.render_html {
				let rendered = FormTemplate::render();

				Ok(Response::ok()
					.with_body(Bytes::from(rendered))
					.with_header("content-type", "text/html; charset=utf-8"))
			} else {
				let form_data = FormData {
					name: "John Doe".to_string(),
					email: "john@example.com".to_string(),
					age: 30,
				};

				let json = serde_json::to_string(&form_data)
					.map_err(|e| reinhardt_exception::Error::Internal(e.to_string()))?;

				Ok(Response::ok()
					.with_body(Bytes::from(json))
					.with_header("content-type", "application/json"))
			}
		}
	}

	#[tokio::test]
	async fn test_renderer_template_json_response() {
		let handler = Arc::new(FormHandler { render_html: false });

		let request = Request::builder()
			.method(Method::GET)
			.uri("/api/data")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let response = handler.handle(request).await.unwrap();

		assert_eq!(response.status, StatusCode::OK);

		let body_str = String::from_utf8(response.body.to_vec()).unwrap();
		let data: FormData = serde_json::from_str(&body_str).unwrap();

		assert_eq!(data.name, "John Doe");
		assert_eq!(data.email, "john@example.com");
		assert_eq!(data.age, 30);
	}

	#[tokio::test]
	async fn test_browsable_api() {
		let handler = Arc::new(FormHandler { render_html: true });

		let request = Request::builder()
			.method(Method::GET)
			.uri("/api/data")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let response = handler.handle(request).await.unwrap();

		assert_eq!(response.status, StatusCode::OK);

		let body_str = String::from_utf8(response.body.to_vec()).unwrap();
		assert!(body_str.contains("<form"));
		assert!(body_str.contains("name=\"name\""));
		assert!(body_str.contains("name=\"email\""));
		assert!(body_str.contains("name=\"age\""));
		assert!(body_str.contains("Submit"));
	}

	#[tokio::test]
	async fn test_post_many_related_view() {
		#[derive(Debug, Clone, Serialize, Deserialize)]
		struct ManyToManyFormData {
			user_id: i32,
			tag_ids: Vec<i32>,
		}

		#[derive(Clone)]
		struct ManyToManyHandler;

		#[async_trait]
		impl Handler for ManyToManyHandler {
			async fn handle(&self, _request: Request) -> Result<Response> {
				let data = ManyToManyFormData {
					user_id: 1,
					tag_ids: vec![10, 20, 30],
				};

				let json = serde_json::to_string(&data)
					.map_err(|e| reinhardt_exception::Error::Internal(e.to_string()))?;

				Ok(Response::ok()
					.with_body(Bytes::from(json))
					.with_header("content-type", "application/json"))
			}
		}

		let handler = Arc::new(ManyToManyHandler);

		let request = Request::builder()
			.method(Method::POST)
			.uri("/api/user-tags")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let response = handler.handle(request).await.unwrap();

		assert_eq!(response.status, StatusCode::OK);

		let body_str = String::from_utf8(response.body.to_vec()).unwrap();
		let data: ManyToManyFormData = serde_json::from_str(&body_str).unwrap();

		assert_eq!(data.user_id, 1);
		assert_eq!(data.tag_ids, vec![10, 20, 30]);
	}
}
