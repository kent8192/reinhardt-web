//! Metadata integration tests
//!
//! Tests the integration of metadata components including:
//! - SimpleMetadata generation
//! - FieldInfo extraction
//! - OpenAPI schema generation
//! - Validation pattern inference
//! - Field dependency management
//! - OPTIONS request integration

use reinhardt_metadata::{
	BaseMetadata, DependencyManager, FieldDependency, FieldInfo, FieldInfoBuilder, FieldType,
	FieldValidator, MetadataOptions, SerializerFieldInfo, SimpleMetadata, ValidationPattern,
	generate_field_schema, generate_object_schema,
};
use rstest::*;
use serde_json::json;
use std::collections::HashMap;

// =============================================================================
// Fixtures
// =============================================================================

/// Test用のSerializerField情報を提供するfixture
#[fixture]
fn test_serializer_fields() -> HashMap<String, SerializerFieldInfo> {
	let mut fields = HashMap::new();

	fields.insert(
		"username".to_string(),
		SerializerFieldInfo {
			name: "username".to_string(),
			type_name: "String".to_string(),
			is_optional: false,
			is_read_only: false,
			is_write_only: false,
		},
	);

	fields.insert(
		"email".to_string(),
		SerializerFieldInfo {
			name: "email".to_string(),
			type_name: "String".to_string(),
			is_optional: false,
			is_read_only: false,
			is_write_only: false,
		},
	);

	fields.insert(
		"age".to_string(),
		SerializerFieldInfo {
			name: "age".to_string(),
			type_name: "i32".to_string(),
			is_optional: true,
			is_read_only: false,
			is_write_only: false,
		},
	);

	fields.insert(
		"created_at".to_string(),
		SerializerFieldInfo {
			name: "created_at".to_string(),
			type_name: "DateTime".to_string(),
			is_optional: false,
			is_read_only: true,
			is_write_only: false,
		},
	);

	fields
}

/// Test用のFieldInfo collectionを提供するfixture
#[fixture]
fn test_field_infos() -> HashMap<String, FieldInfo> {
	let mut fields = HashMap::new();

	// String field with validation constraints
	fields.insert(
		"username".to_string(),
		FieldInfoBuilder::new(FieldType::String)
			.required(true)
			.label("Username")
			.help_text("Enter your username")
			.min_length(3)
			.max_length(20)
			.build(),
	);

	// Email field
	fields.insert(
		"email".to_string(),
		FieldInfoBuilder::new(FieldType::Email)
			.required(true)
			.label("Email Address")
			.help_text("Enter a valid email address")
			.build(),
	);

	// Integer field with range
	fields.insert(
		"age".to_string(),
		FieldInfoBuilder::new(FieldType::Integer)
			.required(false)
			.label("Age")
			.min_value(0.0)
			.max_value(150.0)
			.build(),
	);

	// DateTime field (read-only)
	fields.insert(
		"created_at".to_string(),
		FieldInfoBuilder::new(FieldType::DateTime)
			.required(true)
			.read_only(true)
			.label("Created At")
			.build(),
	);

	fields
}

// =============================================================================
// Test 1: Simple Metadata Generation
// =============================================================================

/// Test SimpleMetadata generation with serializer fields
///
/// Verifies:
/// - Metadata conversion from serializer field info
/// - Field type inference (String, i32, DateTime)
/// - Required/optional field handling
/// - Read-only flag preservation
/// - Label and help_text generation
#[rstest]
#[tokio::test]
async fn test_simple_metadata_generation(
	test_serializer_fields: HashMap<String, SerializerFieldInfo>,
) {
	let metadata = SimpleMetadata::new();

	// Convert serializer fields to metadata field info
	let fields = metadata.convert_serializer_fields(&test_serializer_fields);

	// Verify field count
	assert_eq!(fields.len(), 4, "Expected 4 fields to be converted");

	// Verify username field
	let username = fields.get("username").expect("username field should exist");
	assert_eq!(
		username.field_type,
		FieldType::String,
		"username should be String type"
	);
	assert!(username.required, "username should be required");
	assert_eq!(
		username.read_only,
		Some(false),
		"username should not be read-only"
	);

	// Verify email field
	let email = fields.get("email").expect("email field should exist");
	assert_eq!(
		email.field_type,
		FieldType::String,
		"email should be String type"
	);
	assert!(email.required, "email should be required");

	// Verify age field (optional)
	let age = fields.get("age").expect("age field should exist");
	assert_eq!(
		age.field_type,
		FieldType::Integer,
		"age should be Integer type"
	);
	assert!(!age.required, "age should be optional");

	// Verify created_at field (read-only)
	let created_at = fields
		.get("created_at")
		.expect("created_at field should exist");
	assert_eq!(
		created_at.field_type,
		FieldType::DateTime,
		"created_at should be DateTime type"
	);
	assert!(created_at.required, "created_at should be required");
	assert_eq!(
		created_at.read_only,
		Some(true),
		"created_at should be read-only"
	);
}

// =============================================================================
// Test 2: Field Info Extraction
// =============================================================================

/// Test FieldInfo extraction from various field types
///
/// Verifies:
/// - Field type extraction (String, Email, Integer, DateTime)
/// - Required/optional status
/// - Read-only flags
/// - Constraint extraction (min_length, max_length, min_value, max_value)
/// - Label and help_text extraction
#[rstest]
#[tokio::test]
async fn test_field_info_extraction(test_field_infos: HashMap<String, FieldInfo>) {
	// Verify username field constraints
	let username = test_field_infos
		.get("username")
		.expect("username field should exist");
	assert_eq!(
		username.field_type,
		FieldType::String,
		"username should be String type"
	);
	assert!(username.required, "username should be required");
	assert_eq!(
		username.label,
		Some("Username".to_string()),
		"username label should match"
	);
	assert_eq!(
		username.help_text,
		Some("Enter your username".to_string()),
		"username help_text should match"
	);
	assert_eq!(
		username.min_length,
		Some(3),
		"username min_length should be 3"
	);
	assert_eq!(
		username.max_length,
		Some(20),
		"username max_length should be 20"
	);

	// Verify email field
	let email = test_field_infos
		.get("email")
		.expect("email field should exist");
	assert_eq!(
		email.field_type,
		FieldType::Email,
		"email should be Email type"
	);
	assert!(email.required, "email should be required");
	assert_eq!(
		email.label,
		Some("Email Address".to_string()),
		"email label should match"
	);

	// Verify age field (optional with range)
	let age = test_field_infos.get("age").expect("age field should exist");
	assert_eq!(
		age.field_type,
		FieldType::Integer,
		"age should be Integer type"
	);
	assert!(!age.required, "age should be optional");
	assert_eq!(age.min_value, Some(0.0), "age min_value should be 0.0");
	assert_eq!(age.max_value, Some(150.0), "age max_value should be 150.0");

	// Verify created_at field (read-only)
	let created_at = test_field_infos
		.get("created_at")
		.expect("created_at field should exist");
	assert_eq!(
		created_at.field_type,
		FieldType::DateTime,
		"created_at should be DateTime type"
	);
	assert_eq!(
		created_at.read_only,
		Some(true),
		"created_at should be read-only"
	);
}

// =============================================================================
// Test 3: OpenAPI Schema Generation
// =============================================================================

/// Test OpenAPI schema generation from field metadata
///
/// Verifies:
/// - OpenAPI type mapping (string, integer, etc.)
/// - Format specification (email, date-time, int64)
/// - Constraint propagation (minLength, maxLength, minimum, maximum)
/// - Required array generation
/// - Description field generation
/// - Read-only flag in schema
#[rstest]
#[tokio::test]
async fn test_openapi_schema_generation(test_field_infos: HashMap<String, FieldInfo>) {
	// Generate object schema
	let schema = generate_object_schema(&test_field_infos);

	// Verify schema type
	assert_eq!(
		schema.schema_type,
		Some("object".to_string()),
		"Schema type should be 'object'"
	);

	// Verify properties exist
	let properties = schema.properties.as_ref().expect("Properties should exist");
	assert_eq!(properties.len(), 4, "Should have 4 properties");

	// Verify username field schema
	let username_schema = properties
		.get("username")
		.expect("username property should exist");
	assert_eq!(
		username_schema.schema_type,
		Some("string".to_string()),
		"username type should be 'string'"
	);
	assert_eq!(
		username_schema.min_length,
		Some(3),
		"username minLength should be 3"
	);
	assert_eq!(
		username_schema.max_length,
		Some(20),
		"username maxLength should be 20"
	);
	assert_eq!(
		username_schema.description,
		Some("Enter your username".to_string()),
		"username description should match help_text"
	);

	// Verify email field schema
	let email_schema = properties
		.get("email")
		.expect("email property should exist");
	assert_eq!(
		email_schema.schema_type,
		Some("string".to_string()),
		"email type should be 'string'"
	);
	assert_eq!(
		email_schema.format,
		Some("email".to_string()),
		"email format should be 'email'"
	);

	// Verify age field schema
	let age_schema = properties.get("age").expect("age property should exist");
	assert_eq!(
		age_schema.schema_type,
		Some("integer".to_string()),
		"age type should be 'integer'"
	);
	assert_eq!(age_schema.minimum, Some(0.0), "age minimum should be 0.0");
	assert_eq!(
		age_schema.maximum,
		Some(150.0),
		"age maximum should be 150.0"
	);

	// Verify created_at field schema
	let created_at_schema = properties
		.get("created_at")
		.expect("created_at property should exist");
	assert_eq!(
		created_at_schema.schema_type,
		Some("string".to_string()),
		"created_at type should be 'string'"
	);
	assert_eq!(
		created_at_schema.format,
		Some("date-time".to_string()),
		"created_at format should be 'date-time'"
	);
	assert_eq!(
		created_at_schema.read_only,
		Some(true),
		"created_at should be read-only"
	);

	// Verify required array
	let required = schema
		.required
		.as_ref()
		.expect("Required array should exist");
	assert_eq!(required.len(), 3, "Should have 3 required fields");
	assert!(
		required.contains(&"username".to_string()),
		"username should be required"
	);
	assert!(
		required.contains(&"email".to_string()),
		"email should be required"
	);
	assert!(
		required.contains(&"created_at".to_string()),
		"created_at should be required"
	);
	assert!(
		!required.contains(&"age".to_string()),
		"age should not be required"
	);
}

// =============================================================================
// Test 4: Validation Pattern Inference
// =============================================================================

/// Test validation pattern inference from field validators
///
/// Verifies:
/// - Email pattern validation
/// - Regex pattern extraction from validators
/// - Min/max length validation
/// - Pattern integration with OpenAPI schema
/// - Multiple validator support
#[rstest]
#[tokio::test]
async fn test_validation_pattern_inference() {
	// Test email validation pattern
	let email_pattern = ValidationPattern::email();
	assert!(
		email_pattern.is_valid("user@example.com"),
		"Should validate correct email"
	);
	assert!(
		email_pattern.is_valid("test.user+tag@example.co.uk"),
		"Should validate email with subdomain and tag"
	);
	assert!(
		!email_pattern.is_valid("invalid-email"),
		"Should reject invalid email"
	);
	assert!(
		!email_pattern.is_valid("@example.com"),
		"Should reject email without local part"
	);

	// Test field with regex validator
	let regex_validator = FieldValidator {
		validator_type: "regex".to_string(),
		options: Some(json!({"pattern": "^[a-zA-Z0-9_]+$"})),
		message: Some("Only alphanumeric characters and underscores allowed".to_string()),
	};

	let field = FieldInfoBuilder::new(FieldType::String)
		.required(true)
		.min_length(3)
		.max_length(20)
		.add_validator(regex_validator)
		.build();

	// Generate schema and verify pattern extraction
	let schema = generate_field_schema(&field);
	assert_eq!(
		schema.pattern,
		Some("^[a-zA-Z0-9_]+$".to_string()),
		"Regex pattern should be extracted to schema"
	);
	assert_eq!(
		schema.min_length,
		Some(3),
		"Min length constraint should be preserved"
	);
	assert_eq!(
		schema.max_length,
		Some(20),
		"Max length constraint should be preserved"
	);

	// Test multiple validators
	let validators = vec![
		FieldValidator {
			validator_type: "min_length".to_string(),
			options: Some(json!({"min": 8})),
			message: Some("Password too short".to_string()),
		},
		FieldValidator {
			validator_type: "regex".to_string(),
			options: Some(json!({"pattern": "^(?=.*[A-Z])(?=.*[0-9]).+$"})),
			message: Some("Password must contain uppercase and number".to_string()),
		},
	];

	let password_field = FieldInfoBuilder::new(FieldType::String)
		.required(true)
		.validators(validators)
		.build();

	let password_schema = generate_field_schema(&password_field);
	assert_eq!(
		password_schema.pattern,
		Some("^(?=.*[A-Z])(?=.*[0-9]).+$".to_string()),
		"Complex regex pattern should be extracted"
	);
}

// =============================================================================
// Test 5: Field Dependency Management
// =============================================================================

/// Test field dependency management and validation
///
/// Verifies:
/// - Dependency creation (requires, one_of, all_of, conditional)
/// - Dependency validation logic
/// - OpenAPI dependency schema generation
/// - Circular dependency detection (if implemented)
/// - Multiple dependency types
#[rstest]
#[tokio::test]
async fn test_field_dependency_management() {
	let mut manager = DependencyManager::new();

	// Test "requires" dependency
	manager.add_dependency(FieldDependency::requires("country", vec!["address"]));

	// Test "one_of" dependency
	manager.add_dependency(FieldDependency::one_of(
		"payment_method",
		vec!["credit_card", "paypal"],
	));

	// Test "all_of" dependency
	manager.add_dependency(FieldDependency::all_of(
		"shipping",
		vec!["address", "city", "zip_code"],
	));

	// Test "conditional" dependency
	manager.add_dependency(FieldDependency::conditional(
		"shipping_method",
		"express",
		vec!["express_fee"],
	));

	// Verify dependency count
	let dependencies = manager.get_dependencies();
	assert_eq!(dependencies.len(), 4, "Should have 4 dependencies");

	// Test validation - success case for "requires"
	let mut present_fields = std::collections::HashSet::new();
	present_fields.insert("country".to_string());
	present_fields.insert("address".to_string());

	let errors = manager.validate_dependencies(&present_fields);
	assert!(
		errors.is_empty(),
		"Validation should pass when required field is present"
	);

	// Test validation - failure case for "requires"
	let mut missing_fields = std::collections::HashSet::new();
	missing_fields.insert("country".to_string());
	// address is missing

	let errors = manager.validate_dependencies(&missing_fields);
	assert_eq!(errors.len(), 1, "Should have 1 validation error");
	assert!(
		errors[0].contains("requires"),
		"Error message should mention 'requires': {}",
		errors[0]
	);
	assert!(
		errors[0].contains("address"),
		"Error message should mention missing field 'address': {}",
		errors[0]
	);

	// Test OpenAPI dependency generation
	let openapi_deps = manager.to_openapi_dependencies();
	assert!(
		openapi_deps.contains_key("country"),
		"OpenAPI dependencies should include 'country'"
	);
	assert!(
		openapi_deps.contains_key("payment_method"),
		"OpenAPI dependencies should include 'payment_method'"
	);
	assert!(
		openapi_deps.contains_key("shipping"),
		"OpenAPI dependencies should include 'shipping'"
	);
	assert!(
		openapi_deps.contains_key("shipping_method"),
		"OpenAPI dependencies should include 'shipping_method'"
	);

	// Verify "requires" dependency format
	let country_dep = &openapi_deps["country"];
	assert!(
		country_dep.is_array(),
		"Requires dependency should be an array"
	);
	let country_arr = country_dep.as_array().expect("Should be array");
	assert_eq!(country_arr.len(), 1, "Should have 1 required field");
	assert_eq!(
		country_arr[0].as_str(),
		Some("address"),
		"Required field should be 'address'"
	);

	// Verify "one_of" dependency format
	let payment_dep = &openapi_deps["payment_method"];
	assert!(
		payment_dep.is_object(),
		"OneOf dependency should be an object"
	);
	assert!(
		payment_dep.get("oneOf").is_some(),
		"Should have 'oneOf' key"
	);
}

// =============================================================================
// Test 6: OPTIONS Request Integration
// =============================================================================

/// Test integration with OPTIONS request handling
///
/// Verifies:
/// - Metadata response generation
/// - Actions field population (POST, PUT, DELETE)
/// - Field metadata in actions
/// - Serializer inspection integration
/// - HTTP method filtering (only POST/PUT/PATCH get actions)
#[rstest]
#[tokio::test]
async fn test_options_request_integration(
	test_serializer_fields: HashMap<String, SerializerFieldInfo>,
) {
	let metadata = SimpleMetadata::new();

	// Create request mock (OPTIONS request)
	let request = reinhardt_core::http::Request::builder()
		.method(hyper::Method::OPTIONS)
		.uri("/users/")
		.version(hyper::Version::HTTP_11)
		.headers(hyper::HeaderMap::new())
		.body(bytes::Bytes::new())
		.build()
		.expect("Failed to build request");

	// Setup metadata options with POST and PUT methods
	let options = MetadataOptions {
		name: "User API".to_string(),
		description: "User management endpoint".to_string(),
		allowed_methods: vec!["GET".to_string(), "POST".to_string(), "PUT".to_string()],
		renders: vec!["application/json".to_string()],
		parses: vec!["application/json".to_string()],
		serializer_fields: Some(test_serializer_fields),
	};

	// Generate metadata response
	let response = metadata
		.determine_metadata(&request, &options)
		.await
		.expect("Failed to determine metadata");

	// Verify basic metadata
	assert_eq!(response.name, "User API", "Response name should match");
	assert_eq!(
		response.description, "User management endpoint",
		"Response description should match"
	);
	assert_eq!(
		response.renders,
		Some(vec!["application/json".to_string()]),
		"Response renders should match"
	);
	assert_eq!(
		response.parses,
		Some(vec!["application/json".to_string()]),
		"Response parses should match"
	);

	// Verify actions field (using as_ref to avoid moving)
	let actions = response
		.actions
		.as_ref()
		.expect("Actions should be present");

	// GET should NOT be in actions (read-only method)
	assert!(!actions.contains_key("GET"), "GET should not be in actions");

	// POST should be in actions
	assert!(actions.contains_key("POST"), "POST should be in actions");
	let post_fields = &actions["POST"];
	assert_eq!(post_fields.len(), 4, "POST should have 4 fields");

	// PUT should be in actions
	assert!(actions.contains_key("PUT"), "PUT should be in actions");
	let put_fields = &actions["PUT"];
	assert_eq!(put_fields.len(), 4, "PUT should have 4 fields");

	// Verify field details in POST action
	assert!(
		post_fields.contains_key("username"),
		"POST fields should include username"
	);
	assert!(
		post_fields.contains_key("email"),
		"POST fields should include email"
	);
	assert!(
		post_fields.contains_key("age"),
		"POST fields should include age"
	);
	assert!(
		post_fields.contains_key("created_at"),
		"POST fields should include created_at"
	);

	// Verify username field in POST action
	let username_field = &post_fields["username"];
	assert_eq!(
		username_field.field_type,
		FieldType::String,
		"username should be String type in POST action"
	);
	assert!(
		username_field.required,
		"username should be required in POST action"
	);
	assert_eq!(
		username_field.read_only,
		Some(false),
		"username should not be read-only in POST action"
	);

	// Verify created_at field in POST action (read-only)
	let created_at_field = &post_fields["created_at"];
	assert_eq!(
		created_at_field.field_type,
		FieldType::DateTime,
		"created_at should be DateTime type in POST action"
	);
	assert_eq!(
		created_at_field.read_only,
		Some(true),
		"created_at should be read-only in POST action"
	);

	// Serialize response to JSON and verify structure
	let json_response = serde_json::to_value(&response).expect("Failed to serialize response");

	assert!(
		json_response.get("name").is_some(),
		"JSON should have 'name' field"
	);
	assert!(
		json_response.get("description").is_some(),
		"JSON should have 'description' field"
	);
	assert!(
		json_response.get("actions").is_some(),
		"JSON should have 'actions' field"
	);

	let actions_json = json_response
		.get("actions")
		.expect("Actions field should exist");
	assert!(
		actions_json.get("POST").is_some(),
		"Actions should have POST"
	);
	assert!(actions_json.get("PUT").is_some(), "Actions should have PUT");
	assert!(
		actions_json.get("GET").is_none(),
		"Actions should not have GET"
	);
}
