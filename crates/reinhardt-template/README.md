# reinhardt-template

Template system for Reinhardt framework

## Overview

`reinhardt-template` provides a comprehensive template system for Reinhardt applications, including template engines, response renderers, and template macros. It integrates Tera (Jinja2-like template engine) with Django-inspired features for flexible HTML rendering and content negotiation.

This crate serves as a parent crate that integrates multiple template-related sub-crates to provide a unified templating experience.

## Features

### Implemented ✓

This parent crate re-exports functionality from the following sub-crates:

- **Templates** (`reinhardt-templates`): Template engine with Tera integration
  - Variable substitution with `{{ variable }}` syntax
  - Control structures: `{% if %}`, `{% for %}` tags
  - Template inheritance: `{% extends %}` and `{% block %}`
  - TemplateLoader for runtime template registration
  - FileSystemTemplateLoader for loading from disk
  - Security: Directory traversal prevention
  - Template caching with cache control (enabled by default, can be disabled)
  - Custom filters: upper, lower, trim, reverse, truncate, join, default, capitalize, title, length, ljust, rjust, replace, split, striptags
  - Static file filters for generating URLs to static assets
  - i18n filters for translation and localization

- **Templates Macros** (`reinhardt-templates-macros`): Procedural macros for templates
  - `#[derive(Template)]` for template structs
  - Compile-time template validation
  - Type-safe template rendering

- **Renderers** (`reinhardt-renderers`): Response renderers for different formats
  - JSONRenderer for JSON responses
  - BrowsableAPIRenderer for HTML interface
  - XMLRenderer for XML responses
  - YAMLRenderer for YAML responses
  - CSVRenderer for CSV tabular data
  - OpenAPIRenderer for OpenAPI 3.0 specifications
  - AdminRenderer for Django-like admin interface
  - StaticHTMLRenderer for static HTML content
  - DocumentationRenderer for API documentation
  - SchemaJSRenderer for JavaScript schemas
  - TemplateHTMLRenderer for template-based HTML rendering
  - Content negotiation based on Accept headers
  - Custom renderer support
  - Pretty printing and formatting options

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
reinhardt-template = "0.1.0-alpha.1"
```

### Optional Features

Enable specific features based on your needs:

```toml
[dependencies]
reinhardt-template = { version = "0.1.0-alpha.1", features = ["templates", "renderers"] }
```

Available features:

- `templates` (default): Template engine functionality
- `templates-macros` (default): Template macros
- `renderers` (default): Response renderers
- `full`: All features enabled

## Usage

### Template Rendering

```rust,no_run
# use tera::{Context, Tera};
# fn example() -> Result<(), Box<dyn std::error::Error>> {
// Create Tera instance
let mut tera = Tera::default();
tera.add_raw_template("index.html", "Hello {{ user }}!")?;

// Create context
let mut context = Context::new();
context.insert("user", "John");

// Render template
let html = tera.render("index.html", &context)?;
# Ok(())
# }
```

### File System Templates

```rust
use reinhardt_template::FileSystemTemplateLoader;
use std::path::PathBuf;

// Create loader
let loader = FileSystemTemplateLoader::new(PathBuf::from("./templates"));

// Load and render template
let html = loader.load_and_render("index.html", &context)?;
```

### JSON Rendering

```rust
use reinhardt_template::{JSONRenderer, Renderer};
use serde_json::json;

let renderer = JSONRenderer::new()
    .pretty(true)
    .ensure_ascii(false);

let data = json!({
    "message": "Hello, world!",
    "status": "success"
});

let response = renderer.render(&data)?;
```

### Content Negotiation

```rust
use reinhardt_template::{ContentNegotiation, JSONRenderer, BrowsableAPIRenderer};

let negotiation = ContentNegotiation::new()
    .add_renderer(Box::new(JSONRenderer::new()))
    .add_renderer(Box::new(BrowsableAPIRenderer::new()));

let renderer = negotiation.select_renderer(&accept_header)?;
let response = renderer.render(&data)?;
```

## Sub-crates

This parent crate contains the following sub-crates:

```
reinhardt-template/
├── Cargo.toml          # Parent crate definition
├── src/
│   └── lib.rs          # Re-exports from sub-crates
└── crates/
    ├── templates/      # Template engine
    ├── templates-macros/ # Template macros
    └── renderers/      # Response renderers
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
