//! Schema generation metadata for ViewSets
//!
//! Provides enhanced OpenAPI schema generation for ViewSets including
//! request/response schemas, parameter descriptions, and action documentation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Schema metadata for a field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSchema {
	/// Field type (string, integer, boolean, etc.)
	pub field_type: String,
	/// Field description
	pub description: Option<String>,
	/// Whether this field is required
	pub required: bool,
	/// Example value
	pub example: Option<serde_json::Value>,
	/// Format (e.g., "email", "date-time", "uuid")
	pub format: Option<String>,
	/// Minimum value (for numbers)
	pub minimum: Option<f64>,
	/// Maximum value (for numbers)
	pub maximum: Option<f64>,
	/// Minimum length (for strings/arrays)
	pub min_length: Option<usize>,
	/// Maximum length (for strings/arrays)
	pub max_length: Option<usize>,
	/// Pattern (regex for strings)
	pub pattern: Option<String>,
	/// Enum values
	pub enum_values: Option<Vec<serde_json::Value>>,
	/// Items schema (for arrays)
	pub items: Option<Box<FieldSchema>>,
	/// Properties (for objects)
	pub properties: Option<HashMap<String, FieldSchema>>,
}

impl FieldSchema {
	/// Create a string field schema
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_viewsets::FieldSchema;
	///
	/// let schema = FieldSchema::string()
	///     .with_description("User's email address")
	///     .with_format("email")
	///     .required();
	/// assert_eq!(schema.field_type, "string");
	/// assert!(schema.required);
	/// ```
	pub fn string() -> Self {
		Self {
			field_type: "string".to_string(),
			description: None,
			required: false,
			example: None,
			format: None,
			minimum: None,
			maximum: None,
			min_length: None,
			max_length: None,
			pattern: None,
			enum_values: None,
			items: None,
			properties: None,
		}
	}

	/// Create an integer field schema
	pub fn integer() -> Self {
		Self {
			field_type: "integer".to_string(),
			description: None,
			required: false,
			example: None,
			format: None,
			minimum: None,
			maximum: None,
			min_length: None,
			max_length: None,
			pattern: None,
			enum_values: None,
			items: None,
			properties: None,
		}
	}

	/// Create a number field schema
	pub fn number() -> Self {
		Self {
			field_type: "number".to_string(),
			description: None,
			required: false,
			example: None,
			format: None,
			minimum: None,
			maximum: None,
			min_length: None,
			max_length: None,
			pattern: None,
			enum_values: None,
			items: None,
			properties: None,
		}
	}

	/// Create a boolean field schema
	pub fn boolean() -> Self {
		Self {
			field_type: "boolean".to_string(),
			description: None,
			required: false,
			example: None,
			format: None,
			minimum: None,
			maximum: None,
			min_length: None,
			max_length: None,
			pattern: None,
			enum_values: None,
			items: None,
			properties: None,
		}
	}

	/// Create an array field schema
	pub fn array(items: FieldSchema) -> Self {
		Self {
			field_type: "array".to_string(),
			description: None,
			required: false,
			example: None,
			format: None,
			minimum: None,
			maximum: None,
			min_length: None,
			max_length: None,
			pattern: None,
			enum_values: None,
			items: Some(Box::new(items)),
			properties: None,
		}
	}

	/// Create an object field schema
	pub fn object() -> Self {
		Self {
			field_type: "object".to_string(),
			description: None,
			required: false,
			example: None,
			format: None,
			minimum: None,
			maximum: None,
			min_length: None,
			max_length: None,
			pattern: None,
			enum_values: None,
			items: None,
			properties: Some(HashMap::new()),
		}
	}

	/// Mark field as required
	pub fn required(mut self) -> Self {
		self.required = true;
		self
	}

	/// Add description
	pub fn with_description(mut self, description: impl Into<String>) -> Self {
		self.description = Some(description.into());
		self
	}

	/// Add example value
	pub fn with_example(mut self, example: serde_json::Value) -> Self {
		self.example = Some(example);
		self
	}

	/// Add format
	pub fn with_format(mut self, format: impl Into<String>) -> Self {
		self.format = Some(format.into());
		self
	}

	/// Add minimum value
	pub fn with_minimum(mut self, min: f64) -> Self {
		self.minimum = Some(min);
		self
	}

	/// Add maximum value
	pub fn with_maximum(mut self, max: f64) -> Self {
		self.maximum = Some(max);
		self
	}

	/// Add minimum length
	pub fn with_min_length(mut self, min: usize) -> Self {
		self.min_length = Some(min);
		self
	}

	/// Add maximum length
	pub fn with_max_length(mut self, max: usize) -> Self {
		self.max_length = Some(max);
		self
	}

	/// Add pattern
	pub fn with_pattern(mut self, pattern: impl Into<String>) -> Self {
		self.pattern = Some(pattern.into());
		self
	}

	/// Add enum values
	pub fn with_enum(mut self, values: Vec<serde_json::Value>) -> Self {
		self.enum_values = Some(values);
		self
	}

	/// Add property to object schema
	pub fn add_property(mut self, name: impl Into<String>, schema: FieldSchema) -> Self {
		if let Some(ref mut props) = self.properties {
			props.insert(name.into(), schema);
		}
		self
	}
}

/// Request schema metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestSchema {
	/// Content type (e.g., "application/json")
	pub content_type: String,
	/// Schema for the request body
	pub schema: ModelSchema,
	/// Examples
	pub examples: Option<HashMap<String, serde_json::Value>>,
}

/// Response schema metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseSchema {
	/// HTTP status code
	pub status_code: u16,
	/// Response description
	pub description: String,
	/// Content type (e.g., "application/json")
	pub content_type: String,
	/// Schema for the response body
	pub schema: ModelSchema,
	/// Examples
	pub examples: Option<HashMap<String, serde_json::Value>>,
}

/// Model schema metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSchema {
	/// Model name
	pub name: String,
	/// Model description
	pub description: Option<String>,
	/// Fields
	pub fields: HashMap<String, FieldSchema>,
}

impl ModelSchema {
	/// Create a new model schema
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_viewsets::{ModelSchema, FieldSchema};
	///
	/// let schema = ModelSchema::new("User")
	///     .with_description("User model")
	///     .add_field("id", FieldSchema::integer().required())
	///     .add_field("name", FieldSchema::string().required())
	///     .add_field("email", FieldSchema::string().with_format("email"));
	/// assert_eq!(schema.fields.len(), 3);
	/// ```
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			description: None,
			fields: HashMap::new(),
		}
	}

	/// Add description
	pub fn with_description(mut self, description: impl Into<String>) -> Self {
		self.description = Some(description.into());
		self
	}

	/// Add a field
	pub fn add_field(mut self, name: impl Into<String>, schema: FieldSchema) -> Self {
		self.fields.insert(name.into(), schema);
		self
	}
}

/// ViewSet schema metadata
#[derive(Debug, Clone)]
pub struct ViewSetSchema {
	/// ViewSet name
	pub name: String,
	/// ViewSet description
	pub description: Option<String>,
	/// Model schema (for model-based viewsets)
	pub model_schema: Option<ModelSchema>,
	/// Custom request schemas per action
	pub request_schemas: HashMap<String, RequestSchema>,
	/// Custom response schemas per action
	pub response_schemas: HashMap<String, Vec<ResponseSchema>>,
	/// Tags for OpenAPI
	pub tags: Vec<String>,
}

impl ViewSetSchema {
	/// Create a new ViewSet schema
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_viewsets::ViewSetSchema;
	///
	/// let schema = ViewSetSchema::new("UserViewSet")
	///     .with_description("User management endpoints")
	///     .with_tags(vec!["users".to_string()]);
	/// assert_eq!(schema.name, "UserViewSet");
	/// assert_eq!(schema.tags.len(), 1);
	/// ```
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			description: None,
			model_schema: None,
			request_schemas: HashMap::new(),
			response_schemas: HashMap::new(),
			tags: Vec::new(),
		}
	}

	/// Add description
	pub fn with_description(mut self, description: impl Into<String>) -> Self {
		self.description = Some(description.into());
		self
	}

	/// Set model schema
	pub fn with_model_schema(mut self, schema: ModelSchema) -> Self {
		self.model_schema = Some(schema);
		self
	}

	/// Add request schema for an action
	pub fn add_request_schema(mut self, action: impl Into<String>, schema: RequestSchema) -> Self {
		self.request_schemas.insert(action.into(), schema);
		self
	}

	/// Add response schema for an action
	pub fn add_response_schema(
		mut self,
		action: impl Into<String>,
		schema: ResponseSchema,
	) -> Self {
		self.response_schemas
			.entry(action.into())
			.or_insert_with(Vec::new)
			.push(schema);
		self
	}

	/// Set tags
	pub fn with_tags(mut self, tags: Vec<String>) -> Self {
		self.tags = tags;
		self
	}

	/// Add tag
	pub fn add_tag(mut self, tag: impl Into<String>) -> Self {
		self.tags.push(tag.into());
		self
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_field_schema_string() {
		let schema = FieldSchema::string()
			.with_description("Test field")
			.with_format("email")
			.required();

		assert_eq!(schema.field_type, "string");
		assert_eq!(schema.description, Some("Test field".to_string()));
		assert_eq!(schema.format, Some("email".to_string()));
		assert!(schema.required);
	}

	#[test]
	fn test_field_schema_integer() {
		let schema = FieldSchema::integer()
			.with_minimum(0.0)
			.with_maximum(100.0)
			.required();

		assert_eq!(schema.field_type, "integer");
		assert_eq!(schema.minimum, Some(0.0));
		assert_eq!(schema.maximum, Some(100.0));
		assert!(schema.required);
	}

	#[test]
	fn test_field_schema_array() {
		let items = FieldSchema::string();
		let schema = FieldSchema::array(items)
			.with_min_length(1)
			.with_max_length(10);

		assert_eq!(schema.field_type, "array");
		assert!(schema.items.is_some());
		assert_eq!(schema.min_length, Some(1));
		assert_eq!(schema.max_length, Some(10));
	}

	#[test]
	fn test_field_schema_object() {
		let schema = FieldSchema::object()
			.add_property("id", FieldSchema::integer().required())
			.add_property("name", FieldSchema::string().required());

		assert_eq!(schema.field_type, "object");
		assert!(schema.properties.is_some());
		let props = schema.properties.unwrap();
		assert_eq!(props.len(), 2);
		assert!(props.contains_key("id"));
		assert!(props.contains_key("name"));
	}

	#[test]
	fn test_model_schema() {
		let schema = ModelSchema::new("User")
			.with_description("User model")
			.add_field("id", FieldSchema::integer().required())
			.add_field("name", FieldSchema::string().required())
			.add_field("email", FieldSchema::string().with_format("email"));

		assert_eq!(schema.name, "User");
		assert_eq!(schema.description, Some("User model".to_string()));
		assert_eq!(schema.fields.len(), 3);
		assert!(schema.fields.get("id").unwrap().required);
		assert!(schema.fields.get("name").unwrap().required);
		assert!(!schema.fields.get("email").unwrap().required);
	}

	#[test]
	fn test_viewset_schema() {
		let model_schema = ModelSchema::new("User")
			.add_field("id", FieldSchema::integer().required())
			.add_field("name", FieldSchema::string().required());

		let schema = ViewSetSchema::new("UserViewSet")
			.with_description("User management")
			.with_model_schema(model_schema)
			.with_tags(vec!["users".to_string()])
			.add_tag("auth".to_string());

		assert_eq!(schema.name, "UserViewSet");
		assert_eq!(schema.description, Some("User management".to_string()));
		assert!(schema.model_schema.is_some());
		assert_eq!(schema.tags.len(), 2);
	}

	#[test]
	fn test_request_schema() {
		let model_schema = ModelSchema::new("CreateUser")
			.add_field("name", FieldSchema::string().required())
			.add_field(
				"email",
				FieldSchema::string().with_format("email").required(),
			);

		let request = RequestSchema {
			content_type: "application/json".to_string(),
			schema: model_schema,
			examples: None,
		};

		assert_eq!(request.content_type, "application/json");
		assert_eq!(request.schema.name, "CreateUser");
		assert_eq!(request.schema.fields.len(), 2);
	}

	#[test]
	fn test_response_schema() {
		let model_schema = ModelSchema::new("User")
			.add_field("id", FieldSchema::integer().required())
			.add_field("name", FieldSchema::string().required());

		let response = ResponseSchema {
			status_code: 200,
			description: "Success".to_string(),
			content_type: "application/json".to_string(),
			schema: model_schema,
			examples: None,
		};

		assert_eq!(response.status_code, 200);
		assert_eq!(response.description, "Success");
		assert_eq!(response.schema.name, "User");
	}

	#[test]
	fn test_field_schema_enum() {
		let schema = FieldSchema::string().with_enum(vec![
			serde_json::json!("active"),
			serde_json::json!("inactive"),
			serde_json::json!("pending"),
		]);

		assert!(schema.enum_values.is_some());
		assert_eq!(schema.enum_values.unwrap().len(), 3);
	}

	#[test]
	fn test_field_schema_pattern() {
		let schema = FieldSchema::string()
			.with_pattern("^[a-zA-Z0-9]+$")
			.with_min_length(3)
			.with_max_length(20);

		assert_eq!(schema.pattern, Some("^[a-zA-Z0-9]+$".to_string()));
		assert_eq!(schema.min_length, Some(3));
		assert_eq!(schema.max_length, Some(20));
	}
}
