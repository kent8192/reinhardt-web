//! OpenAPI schema generator

use crate::SchemaError;
use crate::openapi::OpenApiSchema;

/// Schema generator for OpenAPI schemas
///
/// This is a builder for creating OpenAPI 3.0 schemas.
pub struct SchemaGenerator {
    title: String,
    version: String,
    description: Option<String>,
}

impl SchemaGenerator {
    /// Create a new schema generator
    pub fn new() -> Self {
        Self {
            title: String::new(),
            version: "1.0.0".to_string(),
            description: None,
        }
    }

    /// Set the API title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the API version
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Set the API description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Generate the OpenAPI schema
    pub fn generate(&self) -> Result<OpenApiSchema, SchemaError> {
        use utoipa::openapi::{InfoBuilder, OpenApiBuilder};

        let mut info_builder = InfoBuilder::new().title(&self.title).version(&self.version);

        if let Some(desc) = &self.description {
            info_builder = info_builder.description(Some(desc.as_str()));
        }

        Ok(OpenApiBuilder::new().info(info_builder.build()).build())
    }
}

impl Default for SchemaGenerator {
    fn default() -> Self {
        Self::new()
    }
}
