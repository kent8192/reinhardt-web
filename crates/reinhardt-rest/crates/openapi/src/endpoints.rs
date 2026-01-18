//! OpenAPI endpoint handlers for automatic documentation mounting
//!
//! This module provides endpoint handlers for serving OpenAPI documentation
//! automatically. The OpenAPI JSON is generated once at startup and served
//! from memory - never saved to the filesystem.
//!
//! # Example
//!
//! ```rust,ignore
//! use reinhardt_rest::openapi::endpoints::{swagger_docs, redoc_docs, openapi_json};
//! use reinhardt_http::{Request, Response};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // These handlers can be mounted directly
//! let req = Request::builder().uri("/docs").build().unwrap();
//! let response = swagger_docs(req).await?;
//! # Ok(())
//! # }
//! ```

use crate::generator::SchemaGenerator;
use crate::openapi::OpenApiSchema;
use crate::registry::get_all_schemas;
use crate::swagger::{RedocUI, SwaggerUI};
use reinhardt_http::{Request, Response, Result};
use std::sync::LazyLock;

/// Global OpenAPI schema instance (generated once at startup from memory)
///
/// This schema is generated from the global schema registry populated by
/// `#[derive(Schema)]` macros. It is never saved to the filesystem.
static OPENAPI_SCHEMA: LazyLock<OpenApiSchema> = LazyLock::new(generate_openapi_schema);

/// Global Swagger UI instance
///
/// Configured to fetch OpenAPI JSON from `/api/openapi.json` endpoint.
static SWAGGER_UI: LazyLock<SwaggerUI> = LazyLock::new(|| SwaggerUI::new(OPENAPI_SCHEMA.clone()));

/// Global Redoc UI instance
///
/// Configured to fetch OpenAPI JSON from `/api/openapi.json` endpoint.
static REDOC_UI: LazyLock<RedocUI> = LazyLock::new(|| RedocUI::new(OPENAPI_SCHEMA.clone()));

/// Generate OpenAPI schema from global registry
///
/// This function collects all schemas registered via `#[derive(Schema)]` macros
/// from the global schema registry and generates a complete OpenAPI 3.0 schema.
///
/// The schema is generated once at startup and cached in `OPENAPI_SCHEMA`.
///
/// This function is public to allow `OpenApiRouter` wrapper to generate its own
/// schema instance at wrap time.
pub fn generate_openapi_schema() -> OpenApiSchema {
	let mut generator = SchemaGenerator::new()
		.title("API Documentation")
		.version("1.0.0")
		.description("Auto-generated API documentation");

	// Register all schemas from global registry
	let registry = generator.registry();
	for (name, schema) in get_all_schemas().iter() {
		registry.register(*name, schema.clone());
	}

	// Add function-based endpoints from HTTP method decorators (#[get], #[post], etc.)
	// Collects EndpointMetadata from global inventory via EndpointInspector
	generator = generator.add_function_based_endpoints();

	// NOTE: Server function endpoints should be registered explicitly in each project's
	// config/urls.rs and config/openapi.rs using the `.server_fn(fn_name::marker)` pattern.
	// See examples-twitter/src/config/openapi.rs for reference.

	generator
		.generate()
		.expect("Failed to generate OpenAPI schema")
}

/// Swagger UI endpoint handler
///
/// Serves the Swagger UI HTML page for interactive API documentation.
///
/// # Path
///
/// By default: `/docs`
///
/// # Example
///
/// ```rust,ignore
/// use reinhardt::UnifiedRouter;
/// use reinhardt::Method;
/// use reinhardt_rest::openapi::endpoints::swagger_docs;
///
/// let router = UnifiedRouter::new()
///     .function("/docs", Method::GET, swagger_docs);
/// ```
pub async fn swagger_docs(_req: Request) -> Result<Response> {
	let html = SWAGGER_UI.render_html().map_err(|e| {
		reinhardt_core::exception::Error::Serialization(format!(
			"Failed to render Swagger UI: {}",
			e
		))
	})?;

	Ok(Response::ok()
		.with_header("Content-Type", "text/html; charset=utf-8")
		.with_body(html))
}

/// Redoc UI endpoint handler
///
/// Serves the Redoc UI HTML page for alternative API documentation.
///
/// # Path
///
/// By default: `/docs-redoc`
///
/// # Example
///
/// ```rust,ignore
/// use reinhardt::UnifiedRouter;
/// use reinhardt::Method;
/// use reinhardt_rest::openapi::endpoints::redoc_docs;
///
/// let router = UnifiedRouter::new()
///     .function("/docs-redoc", Method::GET, redoc_docs);
/// ```
pub async fn redoc_docs(_req: Request) -> Result<Response> {
	let html = REDOC_UI.render_html().map_err(|e| {
		reinhardt_core::exception::Error::Serialization(format!("Failed to render Redoc UI: {}", e))
	})?;

	Ok(Response::ok()
		.with_header("Content-Type", "text/html; charset=utf-8")
		.with_body(html))
}

/// OpenAPI JSON endpoint handler
///
/// Serves the OpenAPI 3.0 schema as JSON. The schema is generated once at
/// startup from the global registry and served from memory - never saved
/// to the filesystem.
///
/// # Path
///
/// By default: `/api/openapi.json`
///
/// # Example
///
/// ```rust,ignore
/// use reinhardt::UnifiedRouter;
/// use reinhardt::Method;
/// use reinhardt_rest::openapi::endpoints::openapi_json;
///
/// let router = UnifiedRouter::new()
///     .function("/api/openapi.json", Method::GET, openapi_json);
/// ```
pub async fn openapi_json(_req: Request) -> Result<Response> {
	let json = serde_json::to_string_pretty(&*OPENAPI_SCHEMA).map_err(|e| {
		reinhardt_core::exception::Error::Serialization(format!("JSON serialization error: {}", e))
	})?;

	Ok(Response::ok()
		.with_header("Content-Type", "application/json; charset=utf-8")
		.with_body(json))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_openapi_schema_generation() {
		// Force lazy initialization
		let schema = &*OPENAPI_SCHEMA;
		assert_eq!(schema.info.title, "API Documentation");
		assert_eq!(schema.info.version, "1.0.0");
	}

	#[tokio::test]
	async fn test_swagger_docs_handler() {
		let req = Request::builder().uri("/docs").build().unwrap();
		let response = swagger_docs(req).await.unwrap();

		// Check response contains Swagger UI HTML
		let body_str = String::from_utf8(response.body.to_vec()).unwrap();
		assert!(body_str.contains("swagger-ui"));
	}

	#[tokio::test]
	async fn test_redoc_docs_handler() {
		let req = Request::builder().uri("/docs-redoc").build().unwrap();
		let response = redoc_docs(req).await.unwrap();

		// Check response contains Redoc UI HTML
		let body_str = String::from_utf8(response.body.to_vec()).unwrap();
		assert!(body_str.contains("redoc"));
	}

	#[tokio::test]
	async fn test_openapi_json_handler() {
		let req = Request::builder().uri("/api/openapi.json").build().unwrap();
		let response = openapi_json(req).await.unwrap();

		// Check response is valid JSON
		let body_str = String::from_utf8(response.body.to_vec()).unwrap();
		let _: serde_json::Value = serde_json::from_str(&body_str).unwrap();

		// Check content type
		let content_type = response
			.headers
			.get("content-type")
			.and_then(|h| h.to_str().ok())
			.unwrap_or("");
		assert!(content_type.contains("application/json"));
	}
}
