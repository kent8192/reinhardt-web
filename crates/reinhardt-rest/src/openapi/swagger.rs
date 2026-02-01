//! Swagger UI integration
//!
//! Provides Swagger UI for browsing generated OpenAPI schemas.

use super::SchemaResult;
use crate::OpenApiSchema;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use once_cell::sync::Lazy;
use reinhardt_http::{Request, Response, Result};
use serde::Serialize;
use std::sync::Arc;
use tera::Tera;

/// Embedded favicon for Swagger UI (loaded at compile time)
const SWAGGER_FAVICON_PNG: &[u8] = include_bytes!("../../assets/swagger.png");

/// Embedded favicon for Redoc UI (loaded at compile time)
const REDOC_FAVICON_PNG: &[u8] = include_bytes!("../../assets/redoc.png");

/// Lazy-initialized Tera instance
static TEMPLATES: Lazy<Tera> = Lazy::new(|| {
	let mut tera = Tera::default();

	// Add embedded templates
	tera.add_raw_template("swagger_ui.tpl", include_str!("templates/swagger_ui.tpl"))
		.expect("Failed to add swagger_ui.tpl template");

	tera.add_raw_template("redoc_ui.tpl", include_str!("templates/redoc_ui.tpl"))
		.expect("Failed to add redoc_ui.tpl template");

	tera
});

/// Swagger UI template data
#[derive(Serialize)]
struct SwaggerUITemplate<'a> {
	title: &'a str,
	spec_url: &'a str,
}

/// Redoc UI template data
#[derive(Serialize)]
struct RedocUITemplate<'a> {
	title: &'a str,
	spec_url: &'a str,
}

/// Swagger UI handler
pub struct SwaggerUI {
	openapi_spec: Arc<utoipa::openapi::OpenApi>,
}

impl SwaggerUI {
	/// Create a new Swagger UI handler
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_rest::openapi::{OpenApiSchema, SwaggerUI};
	///
	/// let schema = OpenApiSchema::new("My API", "1.0.0");
	/// let swagger_ui = SwaggerUI::new(schema);
	/// ```
	pub fn new(schema: OpenApiSchema) -> Self {
		// OpenApiSchema is already utoipa's OpenApi, no conversion needed
		Self {
			openapi_spec: Arc::new(schema),
		}
	}
	/// Generate Swagger UI HTML
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_rest::openapi::{OpenApiSchema, SwaggerUI};
	///
	/// let schema = OpenApiSchema::new("My API", "1.0.0");
	/// let swagger_ui = SwaggerUI::new(schema);
	/// let html = swagger_ui.render_html().unwrap();
	/// ```
	pub fn render_html(&self) -> SchemaResult<String> {
		// Render Swagger UI HTML using Tera template
		let context = SwaggerUITemplate {
			title: &self.openapi_spec.info.title,
			spec_url: "/api/openapi.json",
		};

		// Encode favicon as base64 for data URL embedding
		let favicon_base64 = STANDARD.encode(SWAGGER_FAVICON_PNG);

		let mut tera_context = tera::Context::new();
		tera_context.insert("title", &context.title);
		tera_context.insert("spec_url", &context.spec_url);
		tera_context.insert("favicon_base64", &favicon_base64);

		TEMPLATES
			.render("swagger_ui.tpl", &tera_context)
			.map_err(|e| super::SchemaError::InvalidSchema(format!("Template error: {}", e)))
	}
	/// Handle Swagger UI request
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_rest::openapi::{OpenApiSchema, SwaggerUI};
	/// use reinhardt_apps::Request;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let schema = OpenApiSchema::new("My API", "1.0.0");
	/// let swagger_ui = SwaggerUI::new(schema);
	/// let request = Request::new(/* ... */);
	/// let response = swagger_ui.handle(request).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn handle(&self, request: Request) -> Result<Response> {
		let path = request.uri.path();

		match path {
			p if p.starts_with("/swagger-ui/") => {
				// Serve Swagger UI assets
				self.serve_swagger_asset(path).await
			}
			"/api/openapi.json" => {
				// Serve OpenAPI spec
				self.serve_openapi_spec().await
			}
			_ => Ok(Response::not_found()),
		}
	}
	/// Get the schema JSON
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_rest::openapi::{OpenApiSchema, SwaggerUI};
	///
	/// let schema = OpenApiSchema::new("My API", "1.0.0");
	/// let swagger_ui = SwaggerUI::new(schema);
	/// let json = swagger_ui.schema_json().unwrap();
	/// ```
	pub fn schema_json(&self) -> SchemaResult<String> {
		Ok(serde_json::to_string_pretty(&*self.openapi_spec)?)
	}

	/// Serve Swagger UI assets
	async fn serve_swagger_asset(&self, _path: &str) -> Result<Response> {
		// Assets are served via CDN (unpkg.com)
		// No local asset serving needed
		Ok(Response::not_found())
	}

	/// Serve OpenAPI spec
	async fn serve_openapi_spec(&self) -> Result<Response> {
		let json = self.schema_json().map_err(|e| {
			reinhardt_core::exception::Error::Serialization(format!("Schema error: {}", e))
		})?;

		Ok(Response::ok()
			.with_body(json)
			.with_header("Content-Type", "application/json"))
	}
}

/// Redoc UI handler (alternative to Swagger UI)
///
/// This generates a complete Redoc HTML page with proper CDN links,
/// configuration options, and responsive design. Redoc provides a
/// three-panel documentation layout optimized for browsing large APIs.
pub struct RedocUI {
	openapi_spec: Arc<utoipa::openapi::OpenApi>,
}

impl RedocUI {
	/// Create a new Redoc UI handler
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_rest::openapi::{OpenApiSchema, RedocUI};
	///
	/// let schema = OpenApiSchema::new("My API", "1.0.0");
	/// let redoc_ui = RedocUI::new(schema);
	/// ```
	pub fn new(schema: OpenApiSchema) -> Self {
		// OpenApiSchema is already utoipa's OpenApi, no conversion needed
		Self {
			openapi_spec: Arc::new(schema),
		}
	}

	/// Generate Redoc HTML
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_rest::openapi::{OpenApiSchema, RedocUI};
	///
	/// let schema = OpenApiSchema::new("My API", "1.0.0");
	/// let redoc_ui = RedocUI::new(schema);
	/// let html = redoc_ui.render_html().unwrap();
	/// ```
	pub fn render_html(&self) -> SchemaResult<String> {
		// Render Redoc UI HTML using Tera template
		let context = RedocUITemplate {
			title: &self.openapi_spec.info.title,
			spec_url: "/api/openapi.json",
		};

		// Encode favicon as base64 for data URL embedding
		let favicon_base64 = STANDARD.encode(REDOC_FAVICON_PNG);

		let mut tera_context = tera::Context::new();
		tera_context.insert("title", &context.title);
		tera_context.insert("spec_url", &context.spec_url);
		tera_context.insert("favicon_base64", &favicon_base64);

		TEMPLATES
			.render("redoc_ui.tpl", &tera_context)
			.map_err(|e| super::SchemaError::InvalidSchema(format!("Template error: {}", e)))
	}

	/// Handle Redoc request
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_rest::openapi::{OpenApiSchema, RedocUI};
	/// use reinhardt_apps::Request;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let schema = OpenApiSchema::new("My API", "1.0.0");
	/// let redoc_ui = RedocUI::new(schema);
	/// let request = Request::new(/* ... */);
	/// let response = redoc_ui.handle(request).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn handle(&self, _request: Request) -> Result<Response> {
		let html = self.render_html().map_err(|e| {
			reinhardt_core::exception::Error::Serialization(format!("Schema error: {}", e))
		})?;

		Ok(Response::ok()
			.with_body(html)
			.with_header("Content-Type", "text/html; charset=utf-8"))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::Info;
	use utoipa::openapi::PathsBuilder;

	fn create_test_schema() -> OpenApiSchema {
		let info = Info::new("Test API", "1.0.0");
		let paths = PathsBuilder::new().build();
		OpenApiSchema::new(info, paths)
	}

	#[test]
	fn test_swagger_ui_render() {
		let schema = create_test_schema();
		let ui = SwaggerUI::new(schema);

		let html = ui.render_html().unwrap();
		assert!(html.contains("swagger-ui"));
		assert!(html.contains("Test API"));
	}

	#[test]
	fn test_redoc_render() {
		let schema = create_test_schema();
		let ui = RedocUI::new(schema);

		let html = ui.render_html().unwrap();
		assert!(html.contains("redoc"));
		assert!(html.contains("Test API"));
	}

	#[test]
	fn test_schema_json() {
		let schema = create_test_schema();
		let ui = SwaggerUI::new(schema);

		let json = ui.schema_json().unwrap();
		assert!(json.contains("Test API"));
		assert!(json.contains("1.0.0"));
	}
}
