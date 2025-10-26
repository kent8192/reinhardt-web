//! OpenAPI schema generator with registry integration
//!
//! This module provides the main schema generator that integrates with the schema registry,
//! enum schema builder, and serde attributes support.

use crate::registry::SchemaRegistry;
use crate::{openapi::OpenApiSchema, SchemaError};

/// Schema generator for OpenAPI schemas
///
/// This is a builder for creating OpenAPI 3.0 schemas with support for:
/// - Schema registry for component reuse
/// - Advanced enum handling
/// - Serde attributes integration
///
/// # Example
///
/// ```rust
/// use reinhardt_openapi::generator::SchemaGenerator;
/// use reinhardt_openapi::{Schema, SchemaExt};
///
/// let mut generator = SchemaGenerator::new()
///     .title("My API")
///     .version("1.0.0")
///     .description("API documentation");
///
/// // Register schemas
/// generator.registry().register("User", Schema::object_with_properties(
///     vec![
///         ("id", Schema::integer()),
///         ("name", Schema::string()),
///     ],
///     vec!["id", "name"],
/// ));
///
/// // Generate OpenAPI schema
/// let schema = generator.generate().unwrap();
/// ```
pub struct SchemaGenerator {
    title: String,
    version: String,
    description: Option<String>,
    registry: SchemaRegistry,
}

impl SchemaGenerator {
    /// Create a new schema generator
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_openapi::generator::SchemaGenerator;
    ///
    /// let generator = SchemaGenerator::new();
    /// ```
    pub fn new() -> Self {
        Self {
            title: String::new(),
            version: "1.0.0".to_string(),
            description: None,
            registry: SchemaRegistry::new(),
        }
    }

    /// Set the API title
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_openapi::generator::SchemaGenerator;
    ///
    /// let generator = SchemaGenerator::new()
    ///     .title("My API");
    /// ```
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the API version
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_openapi::generator::SchemaGenerator;
    ///
    /// let generator = SchemaGenerator::new()
    ///     .version("2.0.0");
    /// ```
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Set the API description
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_openapi::generator::SchemaGenerator;
    ///
    /// let generator = SchemaGenerator::new()
    ///     .description("My awesome API");
    /// ```
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Get a reference to the schema registry
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_openapi::generator::SchemaGenerator;
    /// use reinhardt_openapi::{Schema, SchemaExt};
    ///
    /// let mut generator = SchemaGenerator::new();
    /// generator.registry().register("User", Schema::object());
    ///
    /// assert!(generator.registry().contains("User"));
    /// ```
    pub fn registry(&mut self) -> &mut SchemaRegistry {
        &mut self.registry
    }

    /// Get the schema registry
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_openapi::generator::SchemaGenerator;
    ///
    /// let generator = SchemaGenerator::new();
    /// let registry = generator.get_registry();
    /// assert!(registry.is_empty());
    /// ```
    pub fn get_registry(&self) -> &SchemaRegistry {
        &self.registry
    }

    /// Generate the OpenAPI schema
    ///
    /// This generates an OpenAPI 3.0 schema with all registered components.
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_openapi::generator::SchemaGenerator;
    /// use reinhardt_openapi::{Schema, SchemaExt};
    ///
    /// let mut generator = SchemaGenerator::new()
    ///     .title("My API")
    ///     .version("1.0.0");
    ///
    /// generator.registry().register("User", Schema::object());
    ///
    /// let schema = generator.generate().unwrap();
    /// assert_eq!(schema.info.title, "My API");
    /// ```
    pub fn generate(&self) -> Result<OpenApiSchema, SchemaError> {
        use utoipa::openapi::{InfoBuilder, OpenApiBuilder};

        let mut info_builder = InfoBuilder::new().title(&self.title).version(&self.version);

        if let Some(desc) = &self.description {
            info_builder = info_builder.description(Some(desc.as_str()));
        }

        let components = self.registry.to_components();

        Ok(OpenApiBuilder::new()
            .info(info_builder.build())
            .components(Some(components))
            .build())
    }

    /// Generate OpenAPI schema as JSON string
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_openapi::generator::SchemaGenerator;
    ///
    /// let generator = SchemaGenerator::new()
    ///     .title("My API")
    ///     .version("1.0.0");
    ///
    /// let json = generator.to_json().unwrap();
    /// assert!(json.contains("\"title\":\"My API\""));
    /// ```
    pub fn to_json(&self) -> Result<String, SchemaError> {
        let schema = self.generate()?;
        serde_json::to_string_pretty(&schema).map_err(SchemaError::from)
    }

    /// Generate OpenAPI schema as YAML string
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_openapi::generator::SchemaGenerator;
    ///
    /// let generator = SchemaGenerator::new()
    ///     .title("My API")
    ///     .version("1.0.0");
    ///
    /// let yaml = generator.to_yaml().unwrap();
    /// assert!(yaml.contains("title: My API"));
    /// ```
    pub fn to_yaml(&self) -> Result<String, SchemaError> {
        let schema = self.generate()?;
        serde_yaml::to_string(&schema).map_err(|e| SchemaError::SerializationError(e.to_string()))
    }
}

impl Default for SchemaGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openapi::Schema;
    use crate::openapi::SchemaExt;

    #[test]
    fn test_new_generator() {
        let generator = SchemaGenerator::new();
        assert_eq!(generator.version, "1.0.0");
        assert!(generator.title.is_empty());
        assert!(generator.description.is_none());
    }

    #[test]
    fn test_builder_pattern() {
        let generator = SchemaGenerator::new()
            .title("Test API")
            .version("2.0.0")
            .description("Test description");

        assert_eq!(generator.title, "Test API");
        assert_eq!(generator.version, "2.0.0");
        assert_eq!(generator.description, Some("Test description".to_string()));
    }

    #[test]
    fn test_generate_basic_schema() {
        let generator = SchemaGenerator::new()
            .title("My API")
            .version("1.0.0")
            .description("Test API");

        let schema = generator.generate().unwrap();
        assert_eq!(schema.info.title, "My API");
        assert_eq!(schema.info.version, "1.0.0");
        assert_eq!(schema.info.description, Some("Test API".to_string()));
    }

    #[test]
    fn test_registry_integration() {
        let mut generator = SchemaGenerator::new();

        generator.registry().register("User", Schema::object());
        generator.registry().register("Post", Schema::object());

        assert!(generator.registry().contains("User"));
        assert!(generator.registry().contains("Post"));

        let schema = generator.generate().unwrap();
        assert!(schema.components.is_some());

        let components = schema.components.unwrap();
        assert_eq!(components.schemas.len(), 2);
        assert!(components.schemas.contains_key("User"));
        assert!(components.schemas.contains_key("Post"));
    }

    #[test]
    fn test_to_json() {
        let generator = SchemaGenerator::new().title("My API").version("1.0.0");

        let json = generator.to_json().unwrap();
        assert!(json.contains("\"title\":\"My API\""));
        assert!(json.contains("\"version\":\"1.0.0\""));
    }

    #[test]
    fn test_to_yaml() {
        let generator = SchemaGenerator::new().title("My API").version("1.0.0");

        let yaml = generator.to_yaml().unwrap();
        assert!(yaml.contains("title: My API"));
        assert!(yaml.contains("version: 1.0.0"));
    }

    #[test]
    fn test_get_registry() {
        let mut generator = SchemaGenerator::new();
        generator.registry().register("User", Schema::object());

        let registry = generator.get_registry();
        assert!(registry.contains("User"));
    }

    #[test]
    fn test_registry_with_nested_schemas() {
        let mut generator = SchemaGenerator::new();

        // Register User schema
        generator.registry().register(
            "User",
            Schema::object_with_properties(
                vec![("id", Schema::integer()), ("name", Schema::string())],
                vec!["id", "name"],
            ),
        );

        // Get a reference to User in another schema
        let user_ref = generator.registry().get_ref("User").unwrap();

        generator.registry().register(
            "Post",
            Schema::object_with_properties(
                vec![
                    ("id", Schema::integer()),
                    ("title", Schema::string()),
                    ("author", user_ref.into()),
                ],
                vec!["id", "title", "author"],
            ),
        );

        let schema = generator.generate().unwrap();
        let components = schema.components.unwrap();

        assert_eq!(components.schemas.len(), 2);
        assert!(components.schemas.contains_key("User"));
        assert!(components.schemas.contains_key("Post"));
    }

    #[test]
    fn test_empty_registry() {
        let generator = SchemaGenerator::new().title("Empty API").version("1.0.0");

        let schema = generator.generate().unwrap();
        assert!(schema.components.is_some());

        let components = schema.components.unwrap();
        assert_eq!(components.schemas.len(), 0);
    }
}
