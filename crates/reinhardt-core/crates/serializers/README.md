# reinhardt-serializers

Core serialization and deserialization functionality for Reinhardt framework

## Overview

Provides foundational serialization infrastructure that is ORM-agnostic and can be used across different layers of the framework. Includes Django REST Framework-inspired field types, validation system, recursive serialization with circular reference detection, and high-performance arena allocation.

## Installation

Add `reinhardt` to your `Cargo.toml`:

```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["serializers"] }

# Or use a preset:
# reinhardt = { version = "0.1.0-alpha.1", features = ["standard"] }  # Recommended
# reinhardt = { version = "0.1.0-alpha.1", features = ["full"] }      # All features
```

Then import serializer features:

```rust
use reinhardt::core::serializers::{Serializer, JsonSerializer};
use reinhardt::core::serializers::fields::{CharField, IntegerField, EmailField};
use reinhardt::core::serializers::{FieldValidator, ValidationResult};
```

**Note:** Serializer features are included in the `standard` and `full` feature presets.

## Implemented ✓

### Core Serialization

- **Serializer Trait** - Generic serialization interface
  - `serialize()` - Convert data to serialized format
  - Support for custom serializers via trait implementation
- **Deserializer Trait** - Generic deserialization interface
  - `deserialize()` - Parse serialized data into typed structures
- **JsonSerializer<T>** - JSON serialization implementation
  - Built on `serde_json`
  - Generic over any `Serialize` + `Deserialize` type
- **Error Handling**
  - `SerializerError` - Serialization/deserialization errors
  - `ValidatorError` - Validation errors with detailed messages

### Validation System

- **Field Validators** - Per-field validation logic
  - `FieldValidator` trait for custom field validators
  - `FieldLevelValidation` trait for field-level validation hooks
- **Object Validators** - Cross-field validation
  - `ObjectValidator` trait for custom object validators
  - `ObjectLevelValidation` trait for object-level validation hooks
- **Validation Helpers**
  - `validate_fields()` - Utility function for field validation
  - `ValidationResult<T>` - Type alias for validation results
  - `ValidationError` - Detailed error messages with field information

### Field Types

Django REST Framework-inspired field types with built-in validation:

#### String Fields

- **CharField** - String field with length constraints
  - `required` - Whether field is required (default: true)
  - `allow_null` - Allow null values (default: false)
  - `allow_blank` - Allow empty strings (default: false)
  - `min_length` - Minimum string length
  - `max_length` - Maximum string length
  - `default` - Default value when not provided
- **EmailField** - Email format validation
  - Inherits CharField configuration
  - Validates email format (RFC 5322)
- **URLField** - URL format validation
  - Inherits CharField configuration
  - Validates URL format
- **ChoiceField** - Enumerated value validation
  - `choices` - List of allowed values
  - Validates value is in allowed set

#### Numeric Fields

- **IntegerField** - Integer field with range constraints
  - `required` - Whether field is required
  - `allow_null` - Allow null values
  - `min_value` - Minimum integer value
  - `max_value` - Maximum integer value
  - `default` - Default value
- **FloatField** - Float field with range constraints
  - Same configuration as IntegerField for floats

#### Other Fields

- **BooleanField** - Boolean field validation
  - `required` - Whether field is required
  - `allow_null` - Allow null values
  - `default` - Default value
- **DateField** - Date parsing and validation
  - `required` - Whether field is required
  - `allow_null` - Allow null values
  - `format` - Date format string
  - `default` - Default value
- **DateTimeField** - DateTime parsing and validation
  - `required` - Whether field is required
  - `allow_null` - Allow null values
  - `format` - DateTime format string
  - `default` - Default value

#### Field Error Handling

- **FieldError** - Field-specific error types
  - Required field missing
  - Type mismatch
  - Validation failure
  - Range violations

### Recursive Serialization

Advanced features for handling nested and circular data structures:

- **SerializationContext** - Depth tracking for nested structures
  - Maximum depth configuration
  - Current depth tracking
  - Depth overflow detection
- **Circular Reference Detection** - Pointer-based object identity tracking
  - `ObjectIdentifiable` trait for objects that can be tracked
  - Automatic circular reference detection
  - Prevention of infinite loops
- **Depth Management**
  - `depth::can_descend()` - Check if can descend further
  - `depth::try_descend()` - Attempt to descend with error on overflow
  - `depth::descend_with<F, T>()` - Execute function at increased depth
- **Circular Helpers**
  - `circular::visit_with()` - Visit object with automatic cleanup
  - Ensures visited set is cleaned up even on error
- **Error Types**
  - `RecursiveError` - Recursive serialization errors
  - Maximum depth exceeded
  - Circular reference detected

### Arena Allocation

High-performance memory management for serialization:

- **SerializationArena<'a>** - Memory-efficient arena allocator
  - Arena-bound references for safety
  - Automatic memory management within arena lifetime
- **SerializedValue<'a>** - Arena-allocated serialized values
  - Object, Array, String, Number, Boolean, Null variants
  - All allocations from arena pool
- **FieldValue** - Field value representation
  - Type-safe field value storage
- **Performance Characteristics**
  - 60-90% reduction in allocations vs. traditional approach
  - 1.6-10x faster for deeply nested structures
  - Space complexity: O(nodes) instead of O(depth×nodes)
  - Especially beneficial for recursive and deeply nested data

## Usage

### Basic JSON Serialization

```rust
use reinhardt::core::serializers::{Serializer, JsonSerializer};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct User {
    id: i64,
    name: String,
}

let user = User { id: 1, name: "Alice".to_string() };
let serializer = JsonSerializer::<User>::new();
let json = serializer.serialize(&user)?;
```

### Field Validation

```rust
use reinhardt::core::serializers::fields::{CharField, IntegerField, EmailField};

// String field with length constraints
let name_field = CharField::new()
    .min_length(3)
    .max_length(50)
    .required(true);

// Integer field with range constraints
let age_field = IntegerField::new()
    .min_value(0)
    .max_value(150);

// Email field with format validation
let email_field = EmailField::new()
    .required(true);

// Validate values
name_field.validate("Alice")?;
age_field.validate(30)?;
email_field.validate("alice@example.com")?;
```

### Custom Validators

```rust
use reinhardt::core::serializers::{FieldValidator, ValidationResult};
use serde_json::Value;

struct CustomValidator;

impl FieldValidator for CustomValidator {
    fn validate(&self, value: &Value) -> ValidationResult {
        // Custom validation logic
        if value.as_str().map_or(false, |s| s.starts_with("admin_")) {
            Ok(())
        } else {
            Err(ValidationError::custom("Must start with 'admin_'"))
        }
    }
}
```

### Recursive Serialization with Depth Control

```rust
use reinhardt::core::serializers::recursive::{SerializationContext, circular};

struct Post {
    id: i64,
    title: String,
}

let mut context = SerializationContext::new(5); // Max depth: 5
let post = Post { id: 1, title: "Example".to_string() };

let result = circular::visit_with(&mut context, &post, |ctx| {
    // Serialize with circular reference detection
    Ok(())
});
```

### Arena-Based Serialization (High Performance)

```rust
use reinhardt::core::serializers::arena::{SerializationArena, FieldValue};

let arena = SerializationArena::new();

// Serialize with arena allocation (60-90% reduction in allocations)
let post = Post { id: 1, title: "Example".to_string() };
let serialized = arena.serialize_model(&post, 5);
let json = arena.to_json(&serialized);
```

## Feature Flags

- `json` (default) - JSON serialization support
- `xml` - XML serialization support (requires `quick-xml`, `serde-xml-rs`)
- `yaml` - YAML serialization support (requires `serde_yaml`)
- `full` - Enable all serialization formats

## Architecture

This crate is **ORM-agnostic** and serves as the foundation for higher-level serialization in:

- **reinhardt-rest** - REST API serializers with ORM integration
- **reinhardt-views** - View-level serialization
- Custom application serializers

The separation allows for flexible serialization strategies across different layers of the framework without coupling to specific data sources.
