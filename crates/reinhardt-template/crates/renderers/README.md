# reinhardt-renderers

Response renderers for the Reinhardt framework, inspired by Django REST Framework.

## Overview

Renderers for converting response data to different formats. Includes JSONRenderer, BrowsableAPIRenderer for HTML interface, and support for custom renderers. Handles content negotiation based on Accept headers.

**Runtime template rendering with Tera** for flexible and dynamic template processing!

## Implemented ✓

### Core Renderers

#### JSONRenderer

Renders responses as JSON with configurable formatting options.

**Features:**

- Standard JSON output
- Pretty printing support with `.pretty(true)`
- ASCII encoding control with `.ensure_ascii(true/false)`
- Configurable formatting options

**Example:**

```rust
use reinhardt_renderers::{JSONRenderer, Renderer};
use serde_json::json;

let renderer = JSONRenderer::new()
    .pretty(true)
    .ensure_ascii(false);

let data = json!({"name": "test", "value": 123});
let result = renderer.render(&data, None).await?;
```

#### XMLRenderer

Renders responses as XML with customizable root element.

**Features:**

- Automatic JSON to XML conversion
- Configurable root element name
- XML declaration included
- Proper indentation and formatting

**Example:**

```rust
use reinhardt_renderers::XMLRenderer;

let renderer = XMLRenderer::new()
    .root_name("data");

let result = renderer.render(&data, None).await?;
```

#### BrowsableAPIRenderer

HTML self-documenting API interface (re-exported from `reinhardt-browsable-api`).

**Features:**

- Interactive web interface for API exploration
- Form-based API testing
- Authentication support in browser
- Syntax highlighting for responses
- Human-friendly HTML rendering

### Specialized Renderers

#### AdminRenderer

Django-like admin interface renderer for resource management.

**Features:**

- Admin-style HTML interface
- Automatic table generation from data
- Resource creation confirmation messages
- Configurable base URL
- Support for both object and array data
- Automatic detail URL generation

**Example:**

```rust
use reinhardt_renderers::AdminRenderer;

let renderer = AdminRenderer::new()
    .base_url("/custom-admin");
```

#### StaticHTMLRenderer

Returns pre-defined static HTML content, ignoring input data.

**Features:**

- Static HTML content serving
- Data-independent rendering
- Useful for static pages and templates
- Simple content configuration

**Example:**

```rust
use reinhardt_renderers::StaticHTMLRenderer;

let content = "<html><body><h1>Hello</h1></body></html>";
let renderer = StaticHTMLRenderer::new(content);
```

#### DocumentationRenderer

Renders API documentation from OpenAPI schemas.

**Features:**

- HTML documentation generation
- Markdown documentation generation
- OpenAPI schema parsing
- Endpoint listing with methods and descriptions
- Configurable output format (HTML or Markdown)

**Example:**

```rust
use reinhardt_renderers::DocumentationRenderer;

// HTML format (default)
let renderer = DocumentationRenderer::new();

// Markdown format
let renderer = DocumentationRenderer::new()
    .format_type("markdown");
```

#### SchemaJSRenderer

Renders OpenAPI schemas as JavaScript for Schema.js library.

**Features:**

- OpenAPI to JavaScript conversion
- Helper function generation (`getEndpoint`, `getAllPaths`)
- CommonJS module export support
- Proper JavaScript object notation
- Valid identifier handling

**Example:**

```rust
use reinhardt_renderers::SchemaJSRenderer;

let renderer = SchemaJSRenderer::new();
let js_output = renderer.render(&openapi_schema, None).await?;
```

#### CSVRenderer

Renders tabular data as CSV format with customizable options.

**Features:**

- Array of objects to CSV conversion
- Customizable delimiter (default: `,`)
- Optional header row control
- Automatic type handling (String, Number, Bool, Null)
- Proper CSV escaping and quoting

**Example:**

```rust
use reinhardt_renderers::CSVRenderer;
use serde_json::json;

let renderer = CSVRenderer::new()
    .delimiter(b';')
    .include_header(true);

let data = json!([
    {"name": "Alice", "age": 30},
    {"name": "Bob", "age": 25}
]);

let result = renderer.render(&data, None).await?;
```

#### YAMLRenderer

Renders data as YAML format.

**Features:**

- JSON to YAML conversion
- Clean, human-readable output
- Proper YAML syntax
- Support for complex nested structures

**Example:**

```rust
use reinhardt_renderers::YAMLRenderer;
use serde_json::json;

let renderer = YAMLRenderer::new();
let data = json!({"key": "value", "nested": {"foo": "bar"}});
let result = renderer.render(&data, None).await?;
```

#### OpenAPIRenderer

Renders OpenAPI 3.0 specifications in JSON or YAML format.

**Features:**

- JSON format output (default)
- YAML format output via `.format("yaml")`
- Pretty printing support via `.pretty(true)`
- Full OpenAPI 3.0 schema support

**Example:**

```rust
use reinhardt_renderers::OpenAPIRenderer;

// JSON format (default)
let json_renderer = OpenAPIRenderer::new()
    .pretty(true);

// YAML format
let yaml_renderer = OpenAPIRenderer::new()
    .format("yaml");

let openapi_spec = json!({"openapi": "3.0.0", ...});
let result = json_renderer.render(&openapi_spec, None).await?;
```

### Core Traits

#### Renderer Trait

Base trait for all renderers with async support.

**Methods:**

- `media_types()` - Returns supported MIME types
- `render()` - Async rendering of data to bytes
- `format()` - Optional format identifier

#### RendererContext

Context information passed to renderers during rendering.

### Template Renderers

#### TeraRenderer

**Runtime template renderer** using Tera for flexible template processing.

**Features:**

- Runtime template compilation and rendering
- Dynamic template loading
- User-provided template support
- Flexible template sources (string, file, database)
- Full Jinja2-compatible syntax
- Template inheritance and includes
- Custom filters and functions
- Context-based variable rendering

**Use Cases:**

- View templates (HTML pages)
- Email templates
- User-provided templates
- Configuration file templates
- Database-stored templates
- Dynamic template generation

**Example:**

```rust,no_run
# use reinhardt_renderers::TeraRenderer;
# use serde_json::json;
let renderer = TeraRenderer::new();
let context = json!({
    "name": "Alice",
    "email": "alice@example.com",
    "age": 25
});

let html = renderer.render_template("user.tpl", &context)
    .expect("Failed to render template");
```

**Performance Characteristics**:
- Time Complexity: O(n) - Runtime template parsing and rendering
- Space Complexity: O(n) - Templates cached in memory
- Performance: Flexible runtime rendering with template caching

### Using TeraRenderer

TeraRenderer provides runtime template rendering with full Jinja2 compatibility:

```rust,no_run
# use reinhardt_renderers::TeraRenderer;
# use serde_json::json;
let renderer = TeraRenderer::new();

// Render with context
let context = json!({
    "title": "Welcome",
    "users": vec!["Alice", "Bob", "Charlie"]
});

let html = renderer.render_template("index.tpl", &context)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

See the [TeraRenderer documentation](src/tera_renderer.rs) for detailed usage and examples.

## Dependencies

- `serde_json` - JSON serialization
- `serde_yaml` - YAML support
- `quick-xml` - XML generation
- `csv` - CSV output support
- `utoipa` - OpenAPI schema support
- `bytes` - Efficient byte buffer handling
- `async-trait` - Async trait support
- `tera` - Runtime template rendering engine

## Related Crates

- `reinhardt-browsable-api` - Browsable API interface implementation
- `reinhardt-exception` - Error handling
- `reinhardt-apps` - Application framework
