# reinhardt-templates

Template engine for Reinhardt framework using Tera

## Overview

Template engine integration for rendering HTML responses. Provides a flexible template system built on Tera (Jinja2-like template engine for Rust) with Django-inspired features including custom filters, file system loading, static file handling, and internationalization support.

## Features

### Implemented ✓

#### Core Template Features

- **Variable substitution**: `{{ variable }}` syntax for inserting dynamic content
- **Control structures**: `{% if %}`, `{% for %}` tags for conditional rendering and loops
- **Template inheritance**: `{% extends %}` and `{% block %}` for template composition
- **Basic filters**: Built-in Tera filters for data transformation

#### Template Management

- **TemplateLoader**: Runtime template registration and rendering
  - Register templates with `register()` method
  - Render templates by name with `render()`
  - Type-safe template loading with `TemplateId` trait
  - Thread-safe concurrent template access
- **FileSystemTemplateLoader**: Load templates from the file system
  - Security: Directory traversal prevention
  - Template caching with optional cache control
  - Unicode and emoji support in file paths
  - Deep nested directory support
  - Concurrent access support

#### Custom Filters (Django-compatible)

All filters are available in Tera templates for data transformation:

**String Transformation**

- `upper` - Convert to uppercase: `{{ "hello"|upper }}` → `HELLO`
- `lower` - Convert to lowercase: `{{ "HELLO"|lower }}` → `hello`
- `capitalize` - Capitalize first character: `{{ "hello"|capitalize }}` → `Hello`
- `title` - Convert to title case: `{{ "hello world"|title }}` → `Hello World`
- `trim` - Remove whitespace: `{{ "  hello  "|trim }}` → `hello`
- `reverse` - Reverse string: `{{ "hello"|reverse }}` → `olleh`

**String Manipulation**

- `truncate(length)` - Truncate with ellipsis: `{{ "Hello World"|truncate(5) }}` → `Hello...`
- `ljust(width, fill)` - Left-justify with padding: `{{ "42"|ljust(5, "0") }}` → `42000`
- `rjust(width, fill)` - Right-justify with padding: `{{ "42"|rjust(5, "0") }}` → `00042`
- `replace(from, to)` - Replace substring: `{{ "hello world"|replace("world", "rust") }}` → `hello rust`

**String Analysis**

- `length` - Get string length: `{{ "hello"|length }}` → `5`
- `split(separator)` - Split into array: `{{ "a,b,c"|split(",") }}` → `["a", "b", "c"]`

**Array Operations**

- `join(separator)` - Join array elements: `{{ items|join(", ") }}` → `a, b, c`

**Conditional Rendering**

- `default(value)` - Provide default for empty strings: `{{ ""|default("N/A") }}` → `N/A`

**HTML Processing**

- `striptags` - Remove HTML tags: `{{ "<p>Hello</p>"|striptags }}` → `Hello`

#### Static Files Support

- **static_filter**: Generate URLs for static files
  - Basic static URL generation: `{{ "css/style.css"|static }}` → `/static/css/style.css`
  - Manifest support for hashed filenames (cache busting)
  - Configurable static URL prefix
  - Path normalization (removes leading slashes)
- **StaticConfig**: Global static files configuration
  - Custom static URL (default: `/static/`)
  - Optional manifest-based cache busting
  - Thread-safe configuration management
- **static_path_join**: Join path components for dynamic path construction

#### Internationalization (i18n) Filters

Basic i18n support with placeholder implementations:

- `get_current_language()` - Get current language code
- `trans(message)` - Translate a string
- `trans_with_context(context, message)` - Translate with context
- `blocktrans(message)` - Block translation
- `blocktrans_plural(singular, plural, count)` - Plural-aware translation
- `localize_date_filter(date)` - Localize date formatting
- `localize_number_filter(number)` - Localize number formatting

#### Advanced Template Features

- **Context processors**: Global context variables for all templates
- **Template tags**: Custom template tags beyond filters
- **Auto-escaping**: Automatic HTML escaping for security
- **Include templates**: `{% include %}` tag for template composition

#### Enhanced i18n

- Full integration with reinhardt-i18n crate
- Actual translation lookup (currently returns input as-is)
- Locale-specific date and number formatting
- Plural forms handling for multiple languages
- Translation context support

#### Performance

- Template compilation caching
- Precompiled template bundles
- Lazy template loading
- Memory-efficient template storage

#### Developer Experience

- Template debugging tools
- Better error messages with line numbers
- Template syntax validation
- Hot reload during development

## Usage

### Basic Template Usage

```rust,no_run
# use tera::{Context, Tera};
# fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut tera = Tera::default();
tera.add_raw_template("hello", "Hello {{ name }}!")?;

let mut context = Context::new();
context.insert("name", "World");

let result = tera.render("hello", &context)?;
assert_eq!(result, "Hello World!");
# Ok(())
# }
```

### Template Loader

```rust
use reinhardt_templates::TemplateLoader;

let mut loader = TemplateLoader::new();
loader.register("hello", || "Hello World!".to_string());

let result = loader.render("hello").unwrap();
assert_eq!(result, "Hello World!");
```

### Type-safe Templates

```rust
use reinhardt_templates::{TemplateLoader, TemplateId};

pub struct HomeTemplate;
impl TemplateId for HomeTemplate {
    const NAME: &'static str = "home.html";
}

let mut loader = TemplateLoader::new();
loader.register_typed::<HomeTemplate, _>(|| "<h1>Home Page</h1>".to_string());

let html = loader.render_typed::<HomeTemplate>().unwrap();
```

### File System Template Loader

```rust,no_run
use reinhardt_templates::FileSystemTemplateLoader;
use std::path::Path;

let loader = FileSystemTemplateLoader::new(Path::new("/app/templates"));
let content = loader.load("index.html").unwrap();
```

### Static Files

```rust
use reinhardt_templates::r#static_filters::{StaticConfig, init_static_config};
use std::collections::HashMap;

// Configure static files
init_static_config(StaticConfig {
    static_url: "/static/".to_string(),
    use_manifest: false,
    manifest: HashMap::new(),
});

// In templates: {{ "css/style.css"|static }} → /static/css/style.css
```

## Dependencies

- **tera**: Jinja2-like template engine for Rust
- **serde**: Serialization support for template context
- **thiserror**: Error handling
- **reinhardt-i18n**: Internationalization support
- **reinhardt-exception**: Unified error types

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
