//! utoipa compatibility layer
//!
//! Provides conversion between Reinhardt's OpenAPI types and utoipa types.

use crate::openapi::{
	Components, Contact, Info, License, OpenApiSchema, PathItem, Schema, Server, Tag,
};
use std::collections::HashMap;

/// Convert Reinhardt's OpenApiSchema to utoipa's OpenApi
pub fn convert_to_utoipa_spec(schema: OpenApiSchema) -> utoipa::openapi::OpenApi {
	let mut builder = utoipa::openapi::OpenApiBuilder::new()
		.info(convert_info(schema.info))
		.paths(convert_paths(schema.paths));

	if let Some(components) = schema.components {
		builder = builder.components(convert_components(components));
	}

	if let Some(servers) = schema.servers {
		builder = builder.servers(convert_servers(servers));
	}

	if let Some(tags) = schema.tags {
		builder = builder.tags(convert_tags(tags));
	}

	builder.build()
}

/// Convert Reinhardt's Info to utoipa's Info
fn convert_info(info: Info) -> utoipa::openapi::Info {
	let mut builder = utoipa::openapi::InfoBuilder::new()
		.title(info.title)
		.version(info.version);

	if let Some(description) = info.description {
		builder = builder.description(Some(description));
	}

	if let Some(terms_of_service) = info.terms_of_service {
		builder = builder.terms_of_service(terms_of_service);
	}

	if let Some(contact) = info.contact {
		builder = builder.contact(convert_contact(contact));
	}

	if let Some(license) = info.license {
		builder = builder.license(convert_license(license));
	}

	builder.build()
}

/// Convert Reinhardt's Contact to utoipa's Contact
fn convert_contact(contact: Contact) -> utoipa::openapi::Contact {
	let mut builder = utoipa::openapi::ContactBuilder::new();

	if let Some(name) = contact.name {
		builder = builder.name(name);
	}

	if let Some(url) = contact.url {
		builder = builder.url(url);
	}

	if let Some(email) = contact.email {
		builder = builder.email(email);
	}

	builder.build()
}

/// Convert Reinhardt's License to utoipa's License
fn convert_license(license: License) -> utoipa::openapi::License {
	let mut builder = utoipa::openapi::LicenseBuilder::new().name(license.name);

	if let Some(url) = license.url {
		builder = builder.url(url);
	}

	builder.build()
}

/// Convert Reinhardt's Paths to utoipa's Paths
fn convert_paths(paths: HashMap<String, PathItem>) -> utoipa::openapi::Paths {
	let mut utoipa_paths = utoipa::openapi::Paths::new();

	for (path, path_item) in paths {
		let mut builder = utoipa::openapi::PathItemBuilder::new();

		if let Some(get) = path_item.get {
			builder = builder.get(convert_operation(get));
		}

		if let Some(post) = path_item.post {
			builder = builder.post(convert_operation(post));
		}

		if let Some(put) = path_item.put {
			builder = builder.put(convert_operation(put));
		}

		if let Some(patch) = path_item.patch {
			builder = builder.patch(convert_operation(patch));
		}

		if let Some(delete) = path_item.delete {
			builder = builder.delete(convert_operation(delete));
		}

		utoipa_paths = utoipa_paths.path(&path, builder.build());
	}

	utoipa_paths
}

/// Convert Reinhardt's Operation to utoipa's Operation
fn convert_operation(operation: crate::openapi::Operation) -> utoipa::openapi::Operation {
	let mut builder = utoipa::openapi::OperationBuilder::new();

	if let Some(tags) = operation.tags {
		builder = builder.tags(tags);
	}

	if let Some(summary) = operation.summary {
		builder = builder.summary(summary);
	}

	if let Some(description) = operation.description {
		builder = builder.description(Some(description));
	}

	if let Some(operation_id) = operation.operation_id {
		builder = builder.operation_id(operation_id);
	}

	if let Some(parameters) = operation.parameters {
		builder = builder.parameters(convert_parameters(parameters));
	}

	if let Some(request_body) = operation.request_body {
		builder = builder.request_body(convert_request_body(request_body));
	}

	// Convert responses
	for (status, response) in operation.responses {
		builder = builder.response(status, convert_response(response));
	}

	builder.build()
}

/// Convert Reinhardt's Parameters to utoipa's Parameters
fn convert_parameters(
	parameters: Vec<crate::openapi::Parameter>,
) -> Vec<utoipa::openapi::Parameter> {
	parameters
		.into_iter()
		.map(|param| {
			let mut builder = utoipa::openapi::ParameterBuilder::new()
				.name(param.name)
				.parameter_in(convert_parameter_location(param.location));

			if let Some(description) = param.description {
				builder = builder.description(Some(description));
			}

			if let Some(required) = param.required {
				builder = builder.required(required);
			}

			if let Some(schema) = param.schema {
				builder = builder.schema(convert_schema(schema));
			}

			builder.build()
		})
		.collect()
}

/// Convert Reinhardt's ParameterLocation to utoipa's ParameterIn
fn convert_parameter_location(
	location: crate::openapi::ParameterLocation,
) -> utoipa::openapi::ParameterIn {
	match location {
		crate::openapi::ParameterLocation::Query => utoipa::openapi::ParameterIn::Query,
		crate::openapi::ParameterLocation::Header => utoipa::openapi::ParameterIn::Header,
		crate::openapi::ParameterLocation::Path => utoipa::openapi::ParameterIn::Path,
		crate::openapi::ParameterLocation::Cookie => utoipa::openapi::ParameterIn::Cookie,
	}
}

/// Convert Reinhardt's RequestBody to utoipa's RequestBody
fn convert_request_body(request_body: crate::openapi::RequestBody) -> utoipa::openapi::RequestBody {
	let mut builder = utoipa::openapi::RequestBodyBuilder::new();

	if let Some(description) = request_body.description {
		builder = builder.description(Some(description));
	}

	if let Some(required) = request_body.required {
		builder = builder.required(required);
	}

	// Convert content
	for (content_type, media_type) in request_body.content {
		builder = builder.content(content_type, convert_media_type(media_type));
	}

	builder.build()
}

/// Convert Reinhardt's Response to utoipa's Response
fn convert_response(response: crate::openapi::Response) -> utoipa::openapi::Response {
	let mut builder = utoipa::openapi::ResponseBuilder::new().description(response.description);

	if let Some(content) = response.content {
		for (content_type, media_type) in content {
			builder = builder.content(content_type, convert_media_type(media_type));
		}
	}

	builder.build()
}

/// Convert Reinhardt's MediaType to utoipa's Content
fn convert_media_type(media_type: crate::openapi::MediaType) -> utoipa::openapi::Content {
	let mut builder = utoipa::openapi::ContentBuilder::new();

	if let Some(schema) = media_type.schema {
		builder = builder.schema(convert_schema(schema));
	}

	if let Some(example) = media_type.example {
		builder = builder.example(example);
	}

	builder.build()
}

/// Convert Reinhardt's Schema to utoipa's Schema
fn convert_schema(schema: Schema) -> utoipa::openapi::Schema {
	use utoipa::openapi::schema::{ArrayBuilder, ObjectBuilder};

	// Check if this is a reference
	if let Some(reference) = schema.reference {
		return utoipa::openapi::Schema::Object(
			ObjectBuilder::new()
				.schema_type(utoipa::openapi::SchemaType::Object)
				.description(schema.description)
				.build(),
		);
	}

	// Determine schema type
	let schema_type_str = schema.schema_type.as_deref().unwrap_or("object");

	match schema_type_str {
		"array" => {
			let mut builder = ArrayBuilder::new();

			if let Some(items) = schema.items {
				builder = builder.items(convert_schema(*items));
			}

			if let Some(description) = schema.description {
				builder = builder.description(Some(description));
			}

			utoipa::openapi::Schema::Array(builder.build())
		}
		_ => {
			// Object or primitive types
			let mut builder = ObjectBuilder::new();

			if let Some(schema_type) = schema.schema_type {
				builder = builder.schema_type(convert_schema_type(schema_type));
			}

			if let Some(format) = schema.format {
				builder = builder.format(Some(convert_format(format)));
			}

			if let Some(properties) = schema.properties {
				for (name, prop_schema) in properties {
					builder = builder.property(name, convert_schema(prop_schema));
				}
			}

			if let Some(required) = schema.required {
				builder = builder.required(required);
			}

			if let Some(description) = schema.description {
				builder = builder.description(Some(description));
			}

			if let Some(minimum) = schema.minimum {
				builder = builder.minimum(Some(minimum));
			}

			if let Some(maximum) = schema.maximum {
				builder = builder.maximum(Some(maximum));
			}

			if let Some(pattern) = schema.pattern {
				builder = builder.pattern(Some(pattern));
			}

			if let Some(enum_values) = schema.enum_values {
				builder = builder.enum_values(Some(enum_values));
			}

			if let Some(min_length) = schema.min_length {
				builder = builder.min_length(Some(min_length));
			}

			if let Some(max_length) = schema.max_length {
				builder = builder.max_length(Some(max_length));
			}

			if let Some(default) = schema.default {
				builder = builder.default(Some(default));
			}

			utoipa::openapi::Schema::Object(builder.build())
		}
	}
}

/// Convert string schema type to utoipa SchemaType
fn convert_schema_type(schema_type: String) -> utoipa::openapi::SchemaType {
	match schema_type.as_str() {
		"string" => utoipa::openapi::SchemaType::String,
		"integer" => utoipa::openapi::SchemaType::Integer,
		"number" => utoipa::openapi::SchemaType::Number,
		"boolean" => utoipa::openapi::SchemaType::Boolean,
		"array" => utoipa::openapi::SchemaType::Array,
		"object" => utoipa::openapi::SchemaType::Object,
		_ => utoipa::openapi::SchemaType::String, // Default fallback
	}
}

/// Convert string format to utoipa Format
fn convert_format(format: String) -> utoipa::openapi::SchemaFormat {
	match format.as_str() {
		"date" => utoipa::openapi::SchemaFormat::KnownFormat(utoipa::openapi::KnownFormat::Date),
		"date-time" => {
			utoipa::openapi::SchemaFormat::KnownFormat(utoipa::openapi::KnownFormat::DateTime)
		}
		"email" => utoipa::openapi::SchemaFormat::KnownFormat(utoipa::openapi::KnownFormat::Email),
		"uri" => utoipa::openapi::SchemaFormat::KnownFormat(utoipa::openapi::KnownFormat::Uri),
		"uuid" => utoipa::openapi::SchemaFormat::KnownFormat(utoipa::openapi::KnownFormat::Uuid),
		_ => utoipa::openapi::SchemaFormat::Custom(format),
	}
}

/// Convert Reinhardt's Components to utoipa's Components
fn convert_components(components: Components) -> utoipa::openapi::Components {
	let mut builder = utoipa::openapi::ComponentsBuilder::new();

	if let Some(schemas) = components.schemas {
		for (name, schema) in schemas {
			builder = builder.schema(name, convert_schema(schema));
		}
	}

	if let Some(security_schemes) = components.security_schemes {
		for (name, scheme) in security_schemes {
			builder = builder.security_scheme(name, scheme);
		}
	}

	builder.build()
}

/// Convert Reinhardt's Servers to utoipa's Servers
fn convert_servers(servers: Vec<Server>) -> Vec<utoipa::openapi::Server> {
	servers
		.into_iter()
		.map(|server| {
			let mut builder = utoipa::openapi::ServerBuilder::new().url(server.url);

			if let Some(description) = server.description {
				builder = builder.description(Some(description));
			}

			if let Some(variables) = server.variables {
				for (name, variable) in variables {
					builder = builder.variable(name, convert_server_variable(variable));
				}
			}

			builder.build()
		})
		.collect()
}

/// Convert Reinhardt's ServerVariable to utoipa's ServerVariable
fn convert_server_variable(
	variable: crate::openapi::ServerVariable,
) -> utoipa::openapi::ServerVariable {
	let mut builder = utoipa::openapi::ServerVariableBuilder::new().default_value(variable.default);

	if let Some(enum_values) = variable.enum_values {
		builder = builder.enum_values(enum_values);
	}

	if let Some(description) = variable.description {
		builder = builder.description(description);
	}

	builder.build()
}

/// Convert Reinhardt's Tags to utoipa's Tags
fn convert_tags(tags: Vec<Tag>) -> Vec<utoipa::openapi::Tag> {
	tags.into_iter()
		.map(|tag| {
			let mut builder = utoipa::openapi::TagBuilder::new().name(tag.name);

			if let Some(description) = tag.description {
				builder = builder.description(description);
			}

			builder.build()
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::openapi::{OpenApiSchema, Schema};

	#[test]
	fn test_convert_minimal_schema() {
		let schema = OpenApiSchema::new("Test API", "1.0.0");
		let utoipa_spec = convert_to_utoipa_spec(schema);

		assert_eq!(utoipa_spec.info.title, "Test API");
		assert_eq!(utoipa_spec.info.version, "1.0.0");
	}

	#[test]
	fn test_convert_schema_with_description() {
		let mut schema = OpenApiSchema::new("Test API", "1.0.0");
		schema.info.description = Some("Test description".to_string());

		let utoipa_spec = convert_to_utoipa_spec(schema);

		assert_eq!(
			utoipa_spec.info.description,
			Some("Test description".to_string())
		);
	}

	#[test]
	fn test_convert_schema_types() {
		let string_schema = Schema::string();
		let utoipa_schema = convert_schema(string_schema);

		assert_eq!(
			utoipa_schema.schema_type,
			Some(utoipa::openapi::SchemaType::String)
		);
	}
}
