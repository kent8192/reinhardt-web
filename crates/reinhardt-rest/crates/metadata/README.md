# reinhardt-metadata

API metadata and schema generation for OPTIONS requests in Reinhardt framework.

## Overview

Generates comprehensive metadata about API endpoints including available actions, field information, and validation rules. This metadata is used by browsable APIs and automatic documentation generation. Inspired by Django REST Framework's metadata classes.

## Implemented ✓

### Core Metadata System

- **BaseMetadata Trait**: Base trait for all metadata providers with `determine_metadata` async method
- **SimpleMetadata**: Default metadata implementation that returns view and field information
  - Configurable action inclusion (`include_actions`)
  - Automatic action detection for POST/PUT/PATCH methods
  - Request-based metadata generation

### Field Type System

Comprehensive field types supporting various data types:

- Basic types: `Field`, `Boolean`, `String`, `Integer`, `Float`, `Decimal`
- Date/Time types: `Date`, `DateTime`, `Time`, `Duration`
- Special types: `Email`, `Url`, `Uuid`
- Selection types: `Choice`, `MultipleChoice`
- File types: `File`, `Image`
- Complex types: `List`, `NestedObject`

### Field Metadata

- **FieldInfo**: Detailed field metadata with:
  - Field type and required status
  - Read-only flag
  - Human-readable labels and help text
  - Validation constraints (min/max length, min/max value)
  - Choice options for selection fields
  - Child field for list types
  - Children fields for nested objects

### Builder Pattern

- **FieldInfoBuilder**: Fluent API for constructing field metadata with:
  - Type-safe field configuration
  - Optional constraint setting
  - Choice configuration
  - Nested structure support

### Metadata Response

- **MetadataResponse**: Complete metadata response structure
  - View name and description
  - Supported render formats (e.g., `application/json`)
  - Supported parse formats
  - Available actions with field information

### Configuration

- **MetadataOptions**: Configurable options for metadata generation
  - View name and description
  - Allowed HTTP methods
  - Render and parse formats
  - Default configuration support

### Error Handling

- **MetadataError**: Specialized error types
  - `DeterminationError`: Metadata determination failures
  - `SerializerNotAvailable`: Missing serializer errors

## Planned

### OpenAPI Integration

- OpenAPI 3.0 schema generation from field metadata
- Automatic schema inference from Rust types
- Schema validation and documentation

### Advanced Metadata Providers

- Serializer-aware metadata generation
- Model-based metadata introspection
- Custom metadata class support

### Extended Field Features

- Regular expression validation patterns
- Custom field validators
- Field dependencies and conditional requirements
- Default value specification

## Usage Example

```rust
use reinhardt_metadata::{
    BaseMetadata, SimpleMetadata, MetadataOptions,
    FieldInfoBuilder, FieldType, ChoiceInfo
};
use std::collections::HashMap;

// Create metadata provider
let metadata = SimpleMetadata::new();

// Configure metadata options
let options = MetadataOptions {
    name: "User List".to_string(),
    description: "List all users".to_string(),
    allowed_methods: vec!["GET".to_string(), "POST".to_string()],
    renders: vec!["application/json".to_string()],
    parses: vec!["application/json".to_string()],
};

// Build field metadata
let mut fields = HashMap::new();
fields.insert(
    "username".to_string(),
    FieldInfoBuilder::new(FieldType::String)
        .required(true)
        .label("Username")
        .min_length(3)
        .max_length(50)
        .build()
);

fields.insert(
    "status".to_string(),
    FieldInfoBuilder::new(FieldType::Choice)
        .required(true)
        .choices(vec![
            ChoiceInfo {
                value: "active".to_string(),
                display_name: "Active".to_string(),
            },
            ChoiceInfo {
                value: "inactive".to_string(),
                display_name: "Inactive".to_string(),
            },
        ])
        .build()
);

// Generate action metadata
let actions = metadata.determine_actions(&options.allowed_methods, &fields);
```

## Dependencies

- `reinhardt-apps`: Core application types and request handling
- `reinhardt-serializers`: Serialization support
- `async-trait`: Async trait support
- `serde`: Serialization framework
- `thiserror`: Error handling
