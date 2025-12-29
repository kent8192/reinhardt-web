# reinhardt-serializers

Type-safe data serialization and validation for Rust, inspired by Django REST Framework.

## Overview

Provides serializers for converting between Rust types and various formats (JSON, XML, etc.), with built-in validation support. Includes automatic model serialization, validators for database constraints, and seamless integration with the ORM for type-safe data transformation.

## Installation

Add `reinhardt` to your `Cargo.toml`:

```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["rest-serializers"] }

# Or use a preset:
# reinhardt = { version = "0.1.0-alpha.1", features = ["standard"] }  # Recommended
# reinhardt = { version = "0.1.0-alpha.1", features = ["full"] }      # All features
```

Then import serializer features:

```rust
use reinhardt::rest::serializers::{Serializer, JsonSerializer, ModelSerializer};
use reinhardt::rest::serializers::{UniqueValidator, UniqueTogetherValidator};
use reinhardt::rest::serializers::{NestedSerializer, HyperlinkedModelSerializer};
```

**Note:** Serializer features are included in the `standard` and `full` feature presets.

## Features

### Implemented ✓

#### Core Serialization

- **`Serializer` trait**: Generic trait for data serialization and deserialization
  - `serialize()`: Convert Rust types to output format
  - `deserialize()`: Parse output format back to Rust types
  - `SerializerError`: Type-safe error handling for serialization failures

- **`JsonSerializer<T>`**: JSON serialization implementation
  - Built on `serde_json` for efficient JSON handling
  - Supports any type implementing `Serialize` and `Deserialize`
  - Automatic conversion between Rust types and JSON strings

- **`Deserializer` trait**: Dedicated deserialization interface
  - Separate trait for read-only deserialization operations
  - Enables more flexible data parsing pipelines

#### Model Serialization

- **`ModelSerializer<M>`**: Automatic serialization for ORM models
  - Django-style automatic field mapping from model definitions
  - Built-in validation support with `validate()` method
  - Seamless integration with `reinhardt-orm::Model` trait
  - JSON serialization/deserialization for database models
  - Extensible validation system for custom business logic
  - **Meta Configuration**: Field inclusion/exclusion, read-only/write-only fields
  - **Field Introspection**: Automatic detection of model fields and types
  - **Nested Serializer Support**: Configure and serialize related objects
  - **Validator Integration**: Built-in database constraint validation

#### Meta Configuration

- **`MetaConfig`**: Django REST Framework-style Meta options
  - `fields`: Explicitly include specific fields
  - `exclude`: Exclude specific fields
  - `read_only_fields`: Mark fields as read-only
  - `write_only_fields`: Mark fields as write-only (e.g., passwords)
  - Builder pattern with chainable methods
  - Comprehensive doctests (4 tests) and unit tests (8 tests)

#### Field Introspection

- **`FieldIntrospector`**: Automatic field discovery and type inference
  - Register fields with `FieldInfo` (name, type, optional, collection, primary key)
  - Query fields: `field_names()`, `required_fields()`, `optional_fields()`, `primary_key_field()`
  - Type mapping with `TypeMapper` for common Rust types
  - Integration with ModelSerializer for automatic field detection
  - Comprehensive unit tests (14 tests) and integration tests (10 tests)

- **`FieldInfo`**: Rich field metadata
  - Field name, type name, optionality, collection status
  - Primary key identification
  - Builder pattern for configuration

#### Nested Serialization

- **`NestedSerializerConfig`**: Configure nested object serialization
  - Per-field depth control
  - Read-only vs writable nested fields
  - Create/update permissions (`allow_create`, `allow_update`)
  - Flexible nested field configuration
  - Comprehensive unit tests (11 tests)

- **`NestedFieldConfig`**: Individual nested field configuration
  - `depth()`: Set nesting depth (default: 1)
  - `read_only()`: Mark as read-only
  - `writable()`: Enable create/update operations
  - `allow_create()`, `allow_update()`: Fine-grained permissions

- **`SerializationContext`**: Circular reference and depth management
  - Track visited objects to prevent infinite loops
  - Max depth enforcement
  - Context-aware traversal methods
  - Comprehensive unit tests (15 tests) and integration tests (17 tests)

- **`RecursiveError`**: Error handling for nested serialization
  - `MaxDepthExceeded`: Nesting too deep
  - `CircularReference`: Circular dependency detected
  - `SerializationError`: Generic serialization failures

#### Validator Configuration

- **`ValidatorConfig<M>`**: Manage validators for ModelSerializer
  - Register `UniqueValidator` and `UniqueTogetherValidator`
  - Query registered validators
  - Type-safe validator management
  - Comprehensive unit tests (4 tests) and integration tests (17 tests)

#### Database Validators

- **`UniqueValidator<M>`**: Enforce field uniqueness in database
  - Async validation against PostgreSQL database
  - Supports update operations (excludes current instance from uniqueness check)
  - Customizable field names and error messages
  - Database-level uniqueness verification
  - Builder pattern with `with_message()` for custom error messages
  - Cloneable and debuggable
  - Comprehensive unit tests (4 tests)

- **`UniqueTogetherValidator<M>`**: Ensure unique field combinations
  - Multi-field uniqueness constraints
  - Async PostgreSQL validation
  - Support for update operations
  - Customizable error messages with `with_message()`
  - Flexible field combinations
  - Cloneable and debuggable
  - Comprehensive unit tests (4 tests)

#### Error Handling

- **`SerializerError`**: Comprehensive error type for all serialization operations
  - `Validation(ValidatorError)`: Validation errors with detailed context
  - `Serde { message }`: Serialization/deserialization errors
  - `Other { message }`: Generic errors
  - Helper constructors: `unique_violation()`, `unique_together_violation()`, `required_field()`, `database_error()`
  - `is_validation_error()`, `as_validator_error()` for error inspection
  - Comprehensive error handling tests (22 tests)

- **`ValidatorError`**: Detailed validation error information
  - `UniqueViolation`: Single field uniqueness violation
  - `UniqueTogetherViolation`: Multi-field uniqueness violation
  - `RequiredField`: Missing required field
  - `FieldValidation`: Field constraint violation (regex, range, etc.)
  - `DatabaseError`: Database operation errors
  - `Custom`: Generic validation errors
  - Rich error context with field names, values, and constraints
  - Methods: `message()`, `field_names()`, `is_database_error()`, `is_uniqueness_violation()`

#### Content Negotiation (Re-exported)

- **`ContentNegotiator`**: Select appropriate response format based on client request
- **`MediaType`**: Parse and compare media type strings

#### Parsers (Re-exported from `reinhardt-parsers`)

- **`JSONParser`**: Parse JSON request bodies
- **`FormParser`**: Parse form-encoded data
- **`MultiPartParser`**: Handle multipart/form-data (file uploads)
- **`FileUploadParser`**: Direct file upload handling
- **`ParseError`**: Error type for parsing failures

#### Field Types

- **`FieldError`**: Comprehensive error types for field validation failures
  - 14 error variants covering all validation scenarios
  - Display implementation for user-friendly error messages
- **`CharField`**: String field with length validation
  - Builder pattern with `min_length()`, `max_length()`, `required()`, `allow_blank()`
  - Default value support
  - Comprehensive doctests (7 tests) and unit tests (3 tests)
- **`IntegerField`**: Integer field with range validation
  - Builder pattern with `min_value()`, `max_value()`, `required()`, `allow_null()`
  - i64 value support
  - Comprehensive doctests (6 tests) and unit tests (3 tests)
- **`FloatField`**: Floating-point field with range validation
  - Builder pattern with `min_value()`, `max_value()`, `required()`, `allow_null()`
  - f64 value support
  - Comprehensive doctests (6 tests) and unit tests (1 test)
- **`BooleanField`**: Boolean field handling
  - Builder pattern with `required()`, `allow_null()`, `default()`
  - Always valid validation (booleans can't be invalid)
  - Comprehensive doctests (3 tests) and unit tests (1 test)
- **`EmailField`**: Email format validation
  - Builder pattern with `required()`, `allow_blank()`, `allow_null()`
  - Basic RFC-compliant email validation (@ sign, domain with dot)
  - Comprehensive doctests (4 tests) and unit tests (2 tests)
- **`URLField`**: URL format validation
  - Builder pattern with `required()`, `allow_blank()`, `allow_null()`
  - HTTP/HTTPS protocol validation
  - Comprehensive doctests (4 tests) and unit tests (2 tests)
- **`ChoiceField`**: Enumerated value validation
  - Builder pattern with `required()`, `allow_blank()`, `allow_null()`
  - Configurable list of valid choices
  - Comprehensive doctests (3 tests) and unit tests (2 tests)

#### Advanced Serialization

- **`SerializerMethodField`**: Compute custom read-only fields
  - Method-based computed fields for serializers
  - Custom method names with `.method_name()`
  - HashMap-based context for method values
  - Read-only field support (always `read_only: true`)
  - Example: `full_name` field computed from `first_name` + `last_name`
  - Comprehensive doctests (2 tests) and unit tests (7 tests)

- **`MethodFieldProvider` trait**: Support for serializers with method fields
  - `compute_method_fields()`: Generate all method field values
  - `compute_method()`: Generate single method field value
  - Integration with serializer context

- **`MethodFieldRegistry`**: Manage multiple method fields
  - Register method fields with `.register()`
  - Retrieve fields with `.get()` and `.contains()`
  - Access all fields with `.all()`

#### Validation System

- **`ValidationError`**: Structured validation error messages
  - `FieldError`: Single field validation errors with field name and message
  - `MultipleErrors`: Collection of multiple validation errors
  - `ObjectError`: Object-level validation errors
  - Helper methods: `field_error()`, `object_error()`, `multiple()`
  - thiserror integration for error handling

- **`FieldValidator` trait**: Field-level validation
  - `validate()`: Validate individual field values
  - Implemented by custom validators (EmailValidator, AgeValidator, etc.)
  - JSON Value-based validation

- **`ObjectValidator` trait**: Object-level validation
  - `validate()`: Validate entire objects with multiple fields
  - Cross-field validation support
  - Example: Password confirmation matching

- **`FieldLevelValidation` trait**: Serializer field-level validation
  - `validate_field()`: Validate specific field by name
  - `get_field_validators()`: Get all registered field validators
  - Django-style `validate_<field>()` pattern support

- **`ObjectLevelValidation` trait**: Serializer object-level validation
  - `validate()`: Validate entire serialized object
  - Called after all field validations pass

- **`validate_fields()` helper**: Validate all fields in a data object
  - Takes HashMap of field validators
  - Returns single error or MultipleErrors
  - Comprehensive doctests (3 tests) and unit tests (13 tests)

### Advanced Relations

#### Hyperlinked Model Serializer

- **HyperlinkedModelSerializer<M>**: Django REST Framework-style hyperlinked serialization
- **UrlReverser Trait**: Automatic URL generation for resources
- **View Name Mapping**: Generate URLs based on view names
- **Custom URL Fields**: Configurable URL field names

```rust
use reinhardt::rest::serializers::HyperlinkedModelSerializer;

let serializer = HyperlinkedModelSerializer::<User>::new("user-detail", None);
// Generates URLs like: {"url": "/api/users/123/", "username": "alice"}

// When using UrlReverser
// let reverser: Arc<dyn UrlReverser> = Arc::new(MyUrlReverser);
// let serializer = HyperlinkedModelSerializer::<User>::new("user-detail", Some(reverser));
```

#### Nested Serializers

- **NestedSerializer<M, R>**: Handle nested object serialization
- **Relationship Fields**: Serialize related models inline
- **Depth Control**: Configure nesting depth
- **Bidirectional Relations**: Support for parent-child relationships

```rust
use reinhardt::rest::serializers::NestedSerializer;

let serializer = NestedSerializer::<Post, User>::new("author", 2);
// Serializes: {"title": "Post", "author": {"id": 1, "username": "alice"}}
```

#### Relation Fields

- **PrimaryKeyRelatedField<T>**: Represent relations using primary keys
- **SlugRelatedField<T>**: Represent relations using slug fields
- **StringRelatedField<T>**: Read-only string representation of related objects
- **Flexible Representation**: Choose the best representation for your API

```rust
use reinhardt::rest::serializers::{PrimaryKeyRelatedField, SlugRelatedField};

// Primary key relation: {"author": 123}
let pk_field = PrimaryKeyRelatedField::<User>::new();

// Slug relation: {"author": "alice-smith"}
let slug_field = SlugRelatedField::<User>::new("slug");
```

### ORM Integration

#### QuerySet Integration

- **`SerializerSaveMixin` trait**: Django-style save interface for serializers
- **`SaveContext`**: Transaction-aware context for save operations
- **Manager Integration**: Automatic ORM create/update operations

```rust
use reinhardt::rest::serializers::{SerializerSaveMixin, SaveContext};
use reinhardt::db::orm::{Model, Manager};

// Create new instance
let context = SaveContext::new();
let user = serializer.save(context).await?;

// Update existing instance
let context = SaveContext::with_instance(existing_user);
let updated_user = serializer.update(validated_data, existing_user).await?;
```

#### Transaction Management

- **`TransactionHelper`**: RAII-based transaction management
- **Automatic Rollback**: Drop-based cleanup on errors
- **Savepoint Support**: Nested transaction handling

```rust
use reinhardt::rest::serializers::TransactionHelper;

// Wrap operations in transaction
TransactionHelper::with_transaction(|| async {
    // All database operations here are atomic
    let user = manager.create(user_data).await?;
    let profile = manager.create(profile_data).await?;
    Ok((user, profile))
}).await?;

// Nested transactions with savepoints
TransactionHelper::savepoint(depth, || async {
    // Nested operation with automatic savepoint
    manager.update(instance).await
}).await?;
```

#### Nested Save Context

- **`NestedSaveContext`**: Depth-aware transaction management
- **Automatic Scope Selection**: Transaction vs savepoint based on depth
- **Hierarchical Operations**: Support for deeply nested serializers

```rust
use reinhardt::rest::serializers::NestedSaveContext;

let context = NestedSaveContext::new(depth);

// Automatically uses transaction (depth=0) or savepoint (depth>0)
context.with_scope(|| async {
    // Nested serializer save operations
    nested_serializer.save(data).await
}).await?;
```

#### Many-to-Many Relationship Management

- **`ManyToManyManager`**: Junction table operations
- **Bulk Operations**: Efficient batch insert/delete
- **Set Operations**: Replace all relationships atomically

```rust
use reinhardt::rest::serializers::ManyToManyManager;

let m2m_manager = ManyToManyManager::<User, Tag>::new(
    "user_tags",      // Junction table
    "user_id",        // Source FK
    "tag_id"          // Target FK
);

// Add multiple relationships
m2m_manager.add_bulk(&user_id, vec![tag1_id, tag2_id, tag3_id]).await?;

// Remove specific relationships
m2m_manager.remove_bulk(&user_id, vec![tag1_id]).await?;

// Replace all relationships atomically
m2m_manager.set(&user_id, vec![tag4_id, tag5_id]).await?;

// Clear all relationships
m2m_manager.clear(&user_id).await?;
```

#### Relation Field Database Lookups

- **`PrimaryKeyRelatedFieldORM`**: Database-backed primary key relations
- **`SlugRelatedFieldORM`**: Database-backed slug field relations
- **Batch Query Optimization**: IN clause for multiple lookups
- **Custom QuerySet Filters**: Additional filtering constraints

```rust
use reinhardt::rest::serializers::{PrimaryKeyRelatedFieldORM, SlugRelatedFieldORM};
use reinhardt::db::orm::{Filter, FilterOperator, FilterValue};

// Primary key relation with database validation
let pk_field = PrimaryKeyRelatedFieldORM::<User>::new();

// Validate existence in database
pk_field.validate_exists(&user_id).await?;

// Fetch single instance
let user = pk_field.get_instance(&user_id).await?;

// Batch fetch (prevents N+1 queries)
let users = pk_field.get_instances(vec![id1, id2, id3]).await?;

// Slug field relation with custom filter
let slug_field = SlugRelatedFieldORM::<User>::new("username")
    .with_queryset_filter(Filter::new(
        "is_active",
        FilterOperator::Eq,
        FilterValue::Bool(true)
    ));

// Validate slug exists
slug_field.validate_exists(&slug_value).await?;

// Fetch by slug
let user = slug_field.get_instance(&slug_value).await?;

// Batch fetch by slugs
let users = slug_field.get_instances(vec!["alice", "bob", "charlie"]).await?;
```

#### Performance Optimization

- **`IntrospectionCache`**: Cache field metadata to avoid repeated introspection
- **`QueryCache`**: TTL-based query result caching
- **`BatchValidator`**: Combine multiple database checks into single queries
- **`PerformanceMetrics`**: Track serialization and validation performance

```rust
use reinhardt::rest::serializers::{IntrospectionCache, QueryCache, BatchValidator, PerformanceMetrics};
use std::time::Duration;

// Cache field introspection results
let cache = IntrospectionCache::new();
if let Some(fields) = cache.get("User") {
    // Use cached fields
} else {
    let fields = introspect_fields();
    cache.set("User".to_string(), fields);
}

// Query result caching with TTL
let query_cache = QueryCache::new(Duration::from_secs(300));
query_cache.set("user:123".to_string(), user_data);

// Batch validation
let mut validator = BatchValidator::new();
validator.add_unique_check("users", "email", "alice@example.com");
validator.add_unique_check("users", "username", "alice");
let failures = validator.execute().await?;

// Performance tracking
let metrics = PerformanceMetrics::new();
metrics.record_serialization(50); // 50ms
let stats = metrics.get_stats();
println!("Average: {}ms", stats.avg_serialization_ms);
```

**Note**: ORM integration features (Django-like QuerySet API and SQLAlchemy-like query builder) are available by default. Both patterns can be used interchangeably for database operations.

#### Additional Field Types

- `DateField`, `DateTimeField`: Date and time handling with chrono integration

#### Advanced Serialization

- `WritableNestedSerializer`: Support updates to nested objects
- `ListSerializer`: Serialize collections of objects

#### Additional Renderers

- `YAMLRenderer`: Render data as YAML
- `CSVRenderer`: Render data as CSV (for list endpoints)
- `OpenAPIRenderer`: Generate OpenAPI/Swagger specifications

#### Meta Options

- Field inclusion/exclusion
- Read-only/write-only fields
- Custom field mappings
- Depth control for nested serialization

## Usage Examples

### Basic JSON Serialization

```rust
use reinhardt::rest::serializers::{JsonSerializer, Serializer};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct User {
    name: String,
    age: i64,
}

let serializer = JsonSerializer::<User>::new();
let user = User { name: "Alice".to_string(), age: 30 };

// Serialize to JSON
let json = serializer.serialize(&user).unwrap();
assert_eq!(json, r#"{"name":"Alice","age":30}"#);

// Deserialize from JSON
let parsed = serializer.deserialize(&json).unwrap();
assert_eq!(parsed.name, "Alice");
```

### ModelSerializer with Validation

```rust
use reinhardt::rest::serializers::{ModelSerializer, Serializer};
use reinhardt::db::orm::Model;

// Assuming you have a User model that implements Model
let serializer = ModelSerializer::<User>::new();

let user = User {
    id: Some(1),
    username: "alice".to_string(),
    email: "alice@example.com".to_string(),
};

// Validate before serialization
assert!(serializer.validate(&user).is_ok());

// Serialize to JSON
let json = serializer.serialize(&user).unwrap();
```

### Unique Field Validation

```rust
use reinhardt::rest::serializers::UniqueValidator;
use sqlx::PgPool;

let pool: PgPool = /* your database connection */;
let validator = UniqueValidator::<User>::new("email");

// Validate that email is unique (for new user)
validator.validate(&pool, "alice@example.com", None).await?;

// Validate for update (exclude current user's ID)
validator.validate(&pool, "alice@example.com", Some(&user_id)).await?;
```

### Unique Together Validation

```rust
use reinhardt::rest::serializers::UniqueTogetherValidator;
use std::collections::HashMap;

let validator = UniqueTogetherValidator::<User>::new(vec!["first_name", "last_name"]);

let mut values = HashMap::new();
values.insert("first_name".to_string(), "Alice".to_string());
values.insert("last_name".to_string(), "Smith".to_string());

validator.validate(&pool, &values, None).await?;
```

### SerializerMethodField for Computed Fields

```rust
use reinhardt::rest::serializers::{SerializerMethodField, MethodFieldProvider, MethodFieldRegistry};
use serde_json::{json, Value};
use std::collections::HashMap;

struct UserSerializer {
    method_fields: MethodFieldRegistry,
}

impl UserSerializer {
    fn new() -> Self {
        let mut method_fields = MethodFieldRegistry::new();
        method_fields.register("full_name", SerializerMethodField::new("full_name"));
        Self { method_fields }
    }
}

impl MethodFieldProvider for UserSerializer {
    fn compute_method_fields(&self, instance: &Value) -> HashMap<String, Value> {
        let mut context = HashMap::new();

        if let Some(obj) = instance.as_object() {
            if let (Some(first), Some(last)) = (
                obj.get("first_name").and_then(|v| v.as_str()),
                obj.get("last_name").and_then(|v| v.as_str()),
            ) {
                let full_name = format!("{} {}", first, last);
                context.insert("full_name".to_string(), json!(full_name));
            }
        }

        context
    }

    fn compute_method(&self, method_name: &str, instance: &Value) -> Option<Value> {
        let context = self.compute_method_fields(instance);
        context.get(method_name).cloned()
    }
}

// Usage
let serializer = UserSerializer::new();
let user_data = json!({
    "first_name": "Alice",
    "last_name": "Johnson"
});

let context = serializer.compute_method_fields(&user_data);
assert_eq!(context.get("full_name").unwrap(), &json!("Alice Johnson"));
```

### Field-Level Validation

```rust
use reinhardt::rest::serializers::{FieldValidator, ValidationResult, ValidationError, validate_fields};
use serde_json::{json, Value};
use std::collections::HashMap;

struct EmailValidator;

impl FieldValidator for EmailValidator {
    fn validate(&self, value: &Value) -> ValidationResult {
        if let Some(email) = value.as_str() {
            if email.contains('@') && email.contains('.') {
                Ok(())
            } else {
                Err(ValidationError::field_error("email", "Invalid email format"))
            }
        } else {
            Err(ValidationError::field_error("email", "Must be a string"))
        }
    }
}

// Register validators
let mut validators: HashMap<String, Box<dyn FieldValidator>> = HashMap::new();
validators.insert("email".to_string(), Box::new(EmailValidator));

// Validate data
let mut data = HashMap::new();
data.insert("email".to_string(), json!("user@example.com"));

let result = validate_fields(&data, &validators);
assert!(result.is_ok());
```

### Object-Level Validation

```rust
use reinhardt::rest::serializers::{ObjectValidator, ValidationResult, ValidationError};
use serde_json::{json, Value};
use std::collections::HashMap;

struct PasswordMatchValidator;

impl ObjectValidator for PasswordMatchValidator {
    fn validate(&self, data: &HashMap<String, Value>) -> ValidationResult {
        let password = data.get("password").and_then(|v| v.as_str());
        let confirm = data.get("password_confirm").and_then(|v| v.as_str());

        if password == confirm {
            Ok(())
        } else {
            Err(ValidationError::object_error("Passwords do not match"))
        }
    }
}

// Validate
let validator = PasswordMatchValidator;
let mut data = HashMap::new();
data.insert("password".to_string(), json!("secret123"));
data.insert("password_confirm".to_string(), json!("secret123"));

assert!(validator.validate(&data).is_ok());
```

### Content Negotiation

```rust
use reinhardt::rest::serializers::{ContentNegotiator, JSONRenderer, XMLRenderer};

let negotiator = ContentNegotiator::new();
negotiator.register(Box::new(JSONRenderer::new()));
negotiator.register(Box::new(XMLRenderer::new()));

// Select renderer based on Accept header
let renderer = negotiator.select("application/json")?;
```

## Dependencies

- `reinhardt-orm`: ORM integration for ModelSerializer
- `reinhardt-parsers`: Request body parsing
- `reinhardt-negotiation`: Content type negotiation
- `serde`, `serde_json`: Serialization infrastructure
- `sqlx`: Database operations for validators
- `chrono`: Date and time handling
- `thiserror`: Error type definitions for validation and method fields
- `async-trait`: Async trait support

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE))
- MIT license ([LICENSE-MIT](../../LICENSE-MIT))

at your option.
