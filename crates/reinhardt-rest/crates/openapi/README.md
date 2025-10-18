# reinhardt-openapi

OpenAPI schema generation and Swagger UI integration

## Overview

Automatic OpenAPI 3.0 schema generation from API endpoints, serializers, and viewsets. Generates complete API documentation including request/response schemas, authentication requirements, and parameter descriptions. Includes built-in Swagger UI integration using `utoipa-swagger-ui`.

## Features

### Implemented ✓

#### OpenAPI 3.0 Core Types
- **Complete OpenAPI 3.0 Specification**: Full support for OpenAPI 3.0 types via `utoipa` re-exports
  - Info, Contact, License metadata
  - Paths, PathItem, Operation definitions
  - Parameter definitions (Query, Header, Path, Cookie locations)
  - Request/Response body schemas
  - Components and reusable schemas
  - Security schemes (HTTP, ApiKey, OAuth2)
  - Server definitions with variables
  - Tag definitions for API organization

#### Schema Generation
- **SchemaGenerator**: Builder pattern for creating OpenAPI schemas
  - Fluent API for setting title, version, description
  - Direct generation to `OpenApiSchema` (utoipa's `OpenApi` type)

#### Documentation UI
- **Swagger UI Integration**: Built-in Swagger UI via `utoipa-swagger-ui` and `askama` templates
  - HTML rendering with customizable title and spec URL
  - Request handler for serving Swagger UI pages
  - Automatic OpenAPI spec serving at `/api-docs/openapi.json`
  - Schema JSON export functionality
- **Redoc UI Support**: Alternative documentation interface
  - HTML rendering for Redoc
  - Request handler for serving Redoc pages
  - Uses same OpenAPI spec endpoint

#### Format Export
- **JSON Export**: Serialize OpenAPI schemas to JSON format
- **YAML Export**: Support via `serde_yaml` dependency (capability present in dependencies)

#### utoipa Compatibility Layer
- **Bidirectional Type Conversion**: Complete conversion utilities between Reinhardt and utoipa types
  - Schema type conversions (Object, Array, primitives)
  - Parameter and request/response body conversions
  - Security scheme conversions (HTTP, ApiKey, OAuth2)
  - Server and tag conversions
  - Format and schema type mappings
  - Comprehensive test coverage

#### Auto-Schema Derivation
- **ToSchema Trait**: Core trait for types that can generate OpenAPI schemas
  - `schema()` method returns OpenAPI schema representation
  - `schema_name()` method returns optional schema identifier
  - Implemented for all Rust primitive types (i8-i64, u8-u64, f32-f64, bool, String)
  - Generic implementations for `Option<T>` and `Vec<T>`

- **Schema Derive Macro**: `#[derive(Schema)]` procedural macro for automatic schema generation
  - Automatic field metadata extraction (type, required, nullable)
  - Support for struct types with named fields
  - Automatic required field detection (`Option<T>` fields are optional)
  - Doc comment extraction for field descriptions
  - Support for enum types with string variant generation
  - Compatible with utoipa 5.4 ObjectBuilder pattern

### Planned

#### Extended Auto-Schema Features
- **Attribute Macro Support**: Advanced schema customization
  - Field configuration: `#[schema(example = "...", description = "...")]`
  - Nested schema generation with `$ref` references
  - Advanced enum handling (tagged, adjacently tagged, untagged)
  - Integration with serde attributes (`#[serde(rename)]`, `#[serde(skip)]`)
  - Schema registry for component reuse
  - Validation constraint reflection (min, max, pattern)
  - Example value generation
- **HashMap Support**: `HashMap<K,V>` schema generation
- **Tuple Struct Support**: Schema generation for tuple structs

#### ViewSet Integration
- **ViewSet Inspector**: Automatic schema extraction from ViewSets
  - Introspect ViewSet methods and serializers
  - Generate paths and operations from ViewSet definitions
  - Extract parameter information from method signatures
  - Automatic request/response schema generation from serializers

## Usage

### Schema Derive Macro

```rust
use reinhardt_macros::Schema;
use reinhardt_openapi::ToSchema;

#[derive(Schema)]
struct User {
    /// User's unique identifier
    id: i64,
    /// User's username
    name: String,
    /// Optional email address
    email: Option<String>,
}

#[derive(Schema)]
enum Status {
    Active,
    Inactive,
    Pending,
}

fn main() {
    // Generate schema for User struct
    let user_schema = User::schema();
    let user_name = User::schema_name(); // Some("User")

    // Generate schema for Status enum
    let status_schema = Status::schema();
    let status_name = Status::schema_name(); // Some("Status")
}
```

**Key Features:**
- Doc comments (`///`) are automatically extracted as field descriptions
- `Option<T>` fields are automatically marked as optional (not required)
- Non-optional fields are automatically marked as required
- Enum variants are converted to string schemas with enum values
- Compatible with utoipa 5.4 and later

### Basic Schema Generation

```rust
use reinhardt_openapi::{SchemaGenerator, OpenApiSchema};

// Generate schema from ViewSets
let generator = SchemaGenerator::new("My API", "1.0.0")
    .description("API documentation");

let schema = generator.generate()?;
let json = schema.to_json()?;
```

## Swagger UI Integration

```rustuse reinhardt_openapi::{SchemaGenerator, SwaggerUI};
use reinhardt_apps::{Request, Response};

// Create schemalet schema = SchemaGenerator::new("My API", "1.0.0")
    .description("API documentation")
    .generate()?;

// Create Swagger UI handlerlet swagger_ui = SwaggerUI::new(schema);

// Handle requestsasync fn handle_swagger_request(request: Request) -> Result<Response> {
    swagger_ui.handle(request).await
}
```

## Redoc UI Integration

```rustuse reinhardt_openapi::{SchemaGenerator, RedocUI};

// Create schemalet schema = SchemaGenerator::new("My API", "1.0.0")
    .description("API documentation")
    .generate()?;

// Create Redoc UI handlerlet redoc_ui = RedocUI::new(schema);

// Generate HTMLlet html = redoc_ui.render_html()?;
```

## API Endpoints

When using SwaggerUI, the following endpoints are automatically available:

- `/swagger-ui/` - Swagger UI HTML interface
- `/swagger-ui/swagger-ui-init.js` - Swagger UI initialization script
- `/swagger-ui/swagger-ui.css` - Swagger UI styles
- `/swagger-ui/swagger-ui-bundle.js` - Swagger UI JavaScript bundle
- `/api-docs/openapi.json` - OpenAPI specification in JSON format

## Migration from Previous Version

This version uses `utoipa-swagger-ui` instead of custom templates. The API remains largely compatible, but some internal implementation details have changed:

- Templates are no longer used (askama dependency removed)
- Swagger UI assets are served directly from `utoipa-swagger-ui`
- OpenAPI schemas are converted to `utoipa` format internally
- All existing public APIs remain unchanged
