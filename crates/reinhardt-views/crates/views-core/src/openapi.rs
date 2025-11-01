//! OpenAPI schema generation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAPISpec {
	pub openapi: String,
	pub info: Info,
	pub paths: HashMap<String, PathItem>,
	pub components: Option<Components>,
}

impl OpenAPISpec {
	pub fn new(info: Info) -> Self {
		Self {
			openapi: "3.0.0".to_string(),
			info,
			paths: HashMap::new(),
			components: None,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
	pub title: String,
	pub version: String,
	pub description: Option<String>,
}

impl Info {
	pub fn new(title: String, version: String) -> Self {
		Self {
			title,
			version,
			description: None,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathItem {
	pub get: Option<Operation>,
	pub post: Option<Operation>,
	pub put: Option<Operation>,
	pub delete: Option<Operation>,
	pub patch: Option<Operation>,
}

impl PathItem {
	pub fn new() -> Self {
		Self {
			get: None,
			post: None,
			put: None,
			delete: None,
			patch: None,
		}
	}
}

impl Default for PathItem {
	fn default() -> Self {
		Self::new()
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
	pub summary: Option<String>,
	pub description: Option<String>,
	pub parameters: Vec<Parameter>,
	pub responses: HashMap<String, Response>,
}

impl Operation {
	pub fn new() -> Self {
		Self {
			summary: None,
			description: None,
			parameters: Vec::new(),
			responses: HashMap::new(),
		}
	}
}

impl Default for Operation {
	fn default() -> Self {
		Self::new()
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
	pub name: String,
	pub location: ParameterLocation,
	pub required: bool,
	pub schema: Schema,
}

impl Parameter {
	pub fn new(name: String, location: ParameterLocation) -> Self {
		Self {
			name,
			location,
			required: false,
			schema: Schema::new("string".to_string()),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterLocation {
	Query,
	Header,
	Path,
	Cookie,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
	pub description: String,
	pub content: Option<HashMap<String, MediaType>>,
}

impl Response {
	pub fn new(description: String) -> Self {
		Self {
			description,
			content: None,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaType {
	pub schema: Schema,
}

impl MediaType {
	pub fn new(schema: Schema) -> Self {
		Self { schema }
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
	#[serde(rename = "type")]
	pub schema_type: String,
	pub properties: Option<HashMap<String, Schema>>,
}

impl Schema {
	pub fn new(schema_type: String) -> Self {
		Self {
			schema_type,
			properties: None,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Components {
	pub schemas: HashMap<String, Schema>,
}

impl Components {
	pub fn new() -> Self {
		Self {
			schemas: HashMap::new(),
		}
	}
}

impl Default for Components {
	fn default() -> Self {
		Self::new()
	}
}

pub struct SchemaGenerator;

impl SchemaGenerator {
	pub fn new() -> Self {
		Self
	}

	pub fn generate(&self) -> Schema {
		Schema::new("object".to_string())
	}
}

impl Default for SchemaGenerator {
	fn default() -> Self {
		Self::new()
	}
}

#[derive(Debug, Clone)]
pub struct EndpointInfo {
	pub path: String,
	pub method: String,
	pub operation: Operation,
}

impl EndpointInfo {
	pub fn new(path: String, method: String) -> Self {
		Self {
			path,
			method,
			operation: Operation::new(),
		}
	}
}
