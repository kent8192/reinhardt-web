# reinhardt-renderers

Response renderers for the Reinhardt framework, inspired by Django REST Framework.

## Overview

Renderers for converting response data to different formats. Includes JSONRenderer, BrowsableAPIRenderer for HTML interface, and support for custom renderers. Handles content negotiation based on Accept headers.

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

### Core Traits

#### Renderer Trait
Base trait for all renderers with async support.

**Methods:**
- `media_types()` - Returns supported MIME types
- `render()` - Async rendering of data to bytes
- `format()` - Optional format identifier

#### RendererContext
Context information passed to renderers during rendering.

## Planned

### Additional Renderers
- **CSVRenderer** - CSV format output for tabular data (partially implemented, not exported)
- **YAMLRenderer** - YAML format output (partially implemented, not exported)
- **OpenAPIRenderer** - Generate OpenAPI 3.0 specifications (module exists, not exported)
- **TemplateRenderer** - Template-based HTML rendering with template engine integration

### Content Negotiation
- Automatic renderer selection based on Accept headers
- Renderer registry for managing multiple renderers
- Quality value (q-factor) support
- Format suffix handling (e.g., `/api/users.json`)

### Advanced Features
- Custom renderer middleware
- Renderer chaining
- Response caching
- Streaming support for large responses
- Compression support (gzip, brotli)

## Dependencies

- `serde_json` - JSON serialization
- `serde_yaml` - YAML support
- `quick-xml` - XML generation
- `csv` - CSV output support
- `utoipa` - OpenAPI schema support
- `bytes` - Efficient byte buffer handling
- `async-trait` - Async trait support

## Related Crates

- `reinhardt-browsable-api` - Browsable API interface implementation
- `reinhardt-exception` - Error handling
- `reinhardt-apps` - Application framework

