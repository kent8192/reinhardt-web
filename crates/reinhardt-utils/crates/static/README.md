# reinhardt-static

Static file serving and production utilities for Reinhardt

## Overview

Static file handling for serving CSS, JavaScript, images, and other static assets. Includes file collection, URL generation, storage backends, health checks, and metrics collection for production deployments.

## Features

### Core Functionality

#### ✓ Implemented

- **Static File Configuration** (`StaticFilesConfig`)
  - Configurable static root directory for collected files
  - Static URL path configuration with validation
  - Multiple source directories support via `STATICFILES_DIRS`
  - Media URL configuration and conflict detection

- **Storage Backends** (`Storage` trait)
  - `FileSystemStorage` - Local file system storage
  - `MemoryStorage` - In-memory storage for testing
  - Extensible storage backend system

- **Static File Finder** (`StaticFilesFinder`)
  - Locate files across multiple static directories
  - Support for collecting files from various sources
  - `find_all()` - Recursively discover all static files across configured directories
  - Efficient directory tree traversal with proper error handling

- **Hashed File Storage** (`HashedFileStorage`)
  - File hashing for cache busting
  - Configurable hashing algorithms (MD5, SHA-256)
  - Automatic hash calculation and filename generation
  - Integration with manifest system

- **Manifest System** (`ManifestStaticFilesStorage`)
  - JSON manifest for mapping original filenames to hashed versions
  - Versioned manifest format (currently V1)
  - Enables efficient static file lookup in production
  - Supports deployment workflows with pre-collected assets

- **Media Asset Management** (`Media`, `HasMedia`)
  - CSS and JavaScript dependency declaration for forms and widgets
  - Media type organization (e.g., "all", "screen", "print")
  - HTML rendering for `<link>` and `<script>` tags
  - Dependency merging with duplicate prevention
  - Trait-based system for components to declare their assets

- **Static File Handler** (`StaticFileHandler`)
  - HTTP request handling for static files
  - MIME type detection via `mime_guess`
  - Error handling with `StaticError` and `StaticResult` types
  - File serving with proper content types
  - Directory serving with automatic index file detection
  - Configurable index files (default: `["index.html"]`) via `with_index_files()`
  - Serves index.html when accessing directories directly
  - **ETag Support**: Content-based ETag generation for conditional requests
    - Automatic ETag generation using hash of file content
    - Support for `If-None-Match` headers
    - 304 Not Modified responses for cached resources
    - Implemented in `handler.rs` (`StaticFile::etag()` method)

- **Configuration Validation** (`checks` module)
  - Django-style system checks for static files configuration
  - Multiple check levels: Debug, Info, Warning, Error, Critical
  - Comprehensive validation rules:
    - `static.E001` - STATIC_ROOT not set
    - `static.E002` - STATIC_ROOT in STATICFILES_DIRS
    - `static.E003` - STATIC_URL is empty
    - `static.E004` - STATICFILES_DIRS entry is not a directory
    - `static.W001` - STATIC_ROOT is subdirectory of STATICFILES_DIRS
    - `static.W002` - STATIC_URL doesn't start with '/'
    - `static.W003` - STATIC_URL doesn't end with '/'
    - `static.W004` - STATICFILES_DIRS is empty
    - `static.W005` - Directory doesn't exist
    - `static.W006` - Duplicate STATICFILES_DIRS entries
    - `static.W007` - MEDIA_URL doesn't start with '/'
    - `static.W008` - MEDIA_URL doesn't end with '/'
    - `static.W009` - MEDIA_URL prefix conflict with STATIC_URL
  - Helpful hints for fixing configuration issues

- **Health Check System** (`health` module)
  - Health status monitoring (Healthy, Degraded, Unhealthy)
  - Async health check trait with `async_trait`
  - Health check manager for centralized monitoring
  - Detailed health reports with metadata support
  - Marker traits for specialized checks:
    - `CacheHealthCheck` - Cache-related health checks
    - `DatabaseHealthCheck` - Database-related health checks
  - Component-level health status tracking
  - Production-ready monitoring integration

- **Metrics Collection** (`metrics` module)
  - Performance metrics tracking
  - Request timing and profiling (`RequestTimer`)
  - Request-specific metrics (`RequestMetrics`)
  - Centralized metrics collection (`MetricsCollector`)
  - Generic metric types for custom measurements

- **Middleware** (`StaticFilesMiddleware`)
  - Request/response processing for static files
  - Integration with HTTP pipeline
  - Automatic static file serving in development

- **Dependency Resolution** (`DependencyGraph`)
  - Track dependencies between static assets
  - Resolve asset loading order
  - Support for complex asset dependency chains

#### Implemented in Related Crates

- **collectstatic Command** (implemented in `reinhardt-commands`)
  - ✓ CLI command for collecting static files from all sources
  - ✓ Copy files to STATIC_ROOT with optional processing
  - ✓ Integration with deployment workflows
  - ✓ Progress reporting and verbose output
  - See [reinhardt-commands](../../commands/README.md) for details

- **GZip Compression** (implemented in `reinhardt-middleware`)
  - ✓ Response compression for bandwidth optimization
  - ✓ Configurable compression level (0-9)
  - ✓ Minimum size threshold configuration
  - ✓ Content-type filtering (text/\*, application/json, etc.)
  - ✓ Automatic Accept-Encoding detection
  - ✓ Compression only when beneficial (size check)
  - See [reinhardt-middleware](../../../reinhardt-middleware/README.md) for details

- **Brotli Compression** (implemented in `reinhardt-middleware`)
  - ✓ Advanced compression with better ratios than gzip
  - ✓ Configurable quality levels (Fast, Balanced, Best)
  - ✓ Window size configuration (10-24)
  - ✓ Content-type filtering
  - ✓ Automatic Accept-Encoding: br detection
  - ✓ Intelligent compression (only when beneficial)
  - See [reinhardt-middleware](../../../reinhardt-middleware/README.md) for details

- **Cache-Control Header Management**
  - ✓ Configurable cache policies per file type
  - ✓ Long-term caching for static assets (CSS, JS, fonts, images)
  - ✓ Short-term caching for HTML files
  - ✓ Flexible cache directives (public, private, no-cache, immutable, etc.)
  - ✓ max-age and s-maxage configuration
  - ✓ Vary header support
  - ✓ Pattern-based cache policies

- **CDN Integration**
  - ✓ Multi-provider support (CloudFront, Fastly, Cloudflare, Custom)
  - ✓ CDN URL generation with path prefixes
  - ✓ Versioned URL generation
  - ✓ HTTPS/HTTP configuration
  - ✓ Custom header support
  - ✓ Cache invalidation request helpers
  - ✓ Wildcard purge support

- **Advanced Storage Backends** (`storage` module)
  - `S3Storage` - S3-compatible storage backend (AWS S3, MinIO, LocalStack)
    - Configurable credentials (access key, secret key)
    - Custom endpoint support for S3-compatible services
    - Path-style addressing configuration
    - Path prefix support within buckets
  - `AzureBlobStorage` - Azure Blob Storage backend
    - Shared key and SAS token authentication
    - Custom endpoint support for Azure emulator
    - Container and path prefix configuration
  - `GcsStorage` - Google Cloud Storage backend
    - Service account credentials (JSON or file path)
    - Custom endpoint support for GCS emulator
    - Project ID and bucket configuration
  - `StorageRegistry` - Custom storage backend registration system
    - Dynamic registration of storage backends
    - Factory pattern for creating storage instances
    - Backend lifecycle management (register, unregister, clear)

- **Template Integration** (`template_integration` module)
  - Integration with `reinhardt-templates` for static file URLs in templates
  - `TemplateStaticConfig` - Configuration for template static file generation
  - `init_template_static_config()` - Initialize from `StaticFilesConfig`
  - `init_template_static_config_with_manifest()` - Initialize with manifest support
  - Automatic hashed filename resolution via manifest
  - Works with Tera's `{{ "path/to/file.css"|static }}` filter syntax
  - Supports custom static URLs (CDN, etc.)
  - Feature flag: `templates-integration` (optional)

- **File Processing Pipeline** (`processing` module)
  - CSS/JavaScript minification (basic whitespace and comment removal)
  - Asset bundling with dependency resolution
  - Processing pipeline manager
  - Configurable optimization levels
  - Feature flag: `processing` (default: disabled)

- **Development Server Features** (`dev_server` module)
  - File system watching with `notify` crate
  - Auto-reload notification system using broadcast channels
  - Development error pages with detailed debugging information
  - WebSocket-based reload notifications (port 35729 by default)
  - Smart reload strategies:
    - CSS files: Reload without full page refresh
    - Other files: Full page reload
  - Multiple path watching support
  - Client connection tracking
  - Feature flag: `dev-server` (default: disabled)

- **Advanced File Processing**
  - Image optimization (PNG, JPEG, WebP) - Feature flag: `image-optimization`
  - Source map generation - Feature flag: `source-maps`
  - Asset compression (gzip, brotli) - Feature flag: `compression`
  - Minification for CSS and JavaScript
  - Asset bundling with dependency resolution

- **Advanced Minification** (OXC-powered)
  - Variable renaming (mangling) - Feature flag: `advanced-minification`
  - Dead code elimination
  - Production-grade compression
  - Console.log removal option
  - Debugger statement removal

## Architecture

### Storage System

The storage system is built around the `Storage` trait, allowing multiple backend implementations:

**Local Storage**:

- **FileSystemStorage**: Default storage using local filesystem
- **MemoryStorage**: In-memory storage for testing
- **HashedFileStorage**: Wraps other storage backends to add content-based hashing
- **ManifestStaticFilesStorage**: Production storage with manifest for efficient lookups

**Cloud Storage** (optional, feature-gated):

- **S3Storage**: Amazon S3 and S3-compatible services (MinIO, LocalStack)
- **AzureBlobStorage**: Microsoft Azure Blob Storage
- **GcsStorage**: Google Cloud Storage

**Extensibility**:

- **StorageRegistry**: Register and manage custom storage backends dynamically

### Health Checks

The health check system provides:

- Async health check execution
- Component-level status tracking
- Aggregated health reports
- Extensible check registration
- Integration with monitoring systems

### Metrics

The metrics system enables:

- Request-level timing
- Custom metric collection
- Performance profiling
- Production monitoring integration

## Usage Examples

### Basic Configuration

```rust
use reinhardt_static::StaticFilesConfig;
use std::path::PathBuf;

let config = StaticFilesConfig {
    static_root: PathBuf::from("/var/www/static"),
    static_url: "/static/".to_string(),
    staticfiles_dirs: vec![
        PathBuf::from("app/static"),
        PathBuf::from("vendor/static"),
    ],
    media_url: Some("/media/".to_string()),
};
```

### Configuration Validation

```rust
use reinhardt_static::checks::check_static_files_config;

let messages = check_static_files_config(&config);
for message in messages {
    println!("[{}] {}", message.id, message.message);
    if let Some(hint) = message.hint {
        println!("  Hint: {}", hint);
    }
}
```

### Finding All Static Files

```rust
use reinhardt_static::StaticFilesFinder;
use std::path::PathBuf;

let mut finder = StaticFilesFinder::new();
finder.add_directory(PathBuf::from("app/static"));
finder.add_directory(PathBuf::from("vendor/static"));

// Recursively find all static files
let all_files = finder.find_all();
for file in all_files {
    println!("Found: {}", file);
}
```

### Directory Serving with Index Files

```rust
use reinhardt_static::StaticFileHandler;
use std::path::PathBuf;

let handler = StaticFileHandler::new(PathBuf::from("/var/www/static"))
    .with_index_files(vec![
        "index.html".to_string(),
        "index.htm".to_string(),
        "default.html".to_string(),
    ]);

// Accessing /docs/ will serve /docs/index.html if it exists
```

### Media Assets for Forms

```rust
use reinhardt_static::media::{Media, HasMedia};

let mut media = Media::new();
media.add_css("all", "/static/css/forms.css");
media.add_js("/static/js/widgets.js");

// Render in templates
let css_html = media.render_css();
let js_html = media.render_js();
```

### Health Checks

```rust
use reinhardt_static::health::{HealthCheckManager, HealthCheck, HealthCheckResult};
use async_trait::async_trait;
use std::sync::Arc;

struct StaticFilesHealthCheck;

#[async_trait]
impl HealthCheck for StaticFilesHealthCheck {
    async fn check(&self) -> HealthCheckResult {
        // Check if static files are accessible
        HealthCheckResult::healthy("static_files")
            .with_metadata("static_root_exists", "true")
    }
}

let mut manager = HealthCheckManager::new();
manager.register("static", Arc::new(StaticFilesHealthCheck));

let report = manager.run_checks().await;
if report.is_healthy() {
    println!("All systems operational");
}
```

### Template Integration

**Feature flag**: `templates-integration`

```toml
[dependencies]
reinhardt-static = { version = "0.1.0-alpha.1", features = ["templates-integration"] }
reinhardt-templates = "0.1.0-alpha.1"
```

#### Basic Template Integration

```rust
use reinhardt_static::{StaticFilesConfig, init_template_static_config};
use std::path::PathBuf;

// Initialize static files configuration
let config = StaticFilesConfig {
    static_root: PathBuf::from("/var/www/static"),
    static_url: "/static/".to_string(),
    staticfiles_dirs: vec![],
    media_url: None,
};

// Initialize template static config
init_template_static_config(&config);
```

Now in your Tera templates, you can use the `static` filter:

```html
<!DOCTYPE html>
<html>
<head>
    <link rel="stylesheet" href="{{ "css/style.css"|static }}">
    <script src="{{ "js/app.js"|static }}"></script>
</head>
<body>
    <img src="{{ "images/logo.png"|static }}" alt="Logo">
</body>
</html>
```

This will generate:

```html
<!DOCTYPE html>
<html>
  <head>
    <link rel="stylesheet" href="/static/css/style.css" />
    <script src="/static/js/app.js"></script>
  </head>
  <body>
    <img src="/static/images/logo.png" alt="Logo" />
  </body>
</html>
```

#### Template Integration with Manifest (Hashed Filenames)

```rust
use reinhardt_static::{ManifestStaticFilesStorage, init_template_static_config_with_manifest};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create manifest storage
    let storage = ManifestStaticFilesStorage::new(
        PathBuf::from("/var/www/static"),
        "/static/"
    );

    // Initialize template config with manifest support
    init_template_static_config_with_manifest(&storage).await?;

    Ok(())
}
```

With a manifest file (`staticfiles.json`):

```json
{
  "css/style.css": "css/style.abc123def.css",
  "js/app.js": "js/app.456789abc.js",
  "images/logo.png": "images/logo.xyz987uvw.png"
}
```

The same template will now generate hashed URLs for cache busting:

```html
<!DOCTYPE html>
<html>
  <head>
    <link rel="stylesheet" href="/static/css/style.abc123def.css" />
    <script src="/static/js/app.456789abc.js"></script>
  </head>
  <body>
    <img src="/static/images/logo.xyz987uvw.png" alt="Logo" />
  </body>
</html>
```

#### CDN Integration

```rust
use reinhardt_static::TemplateStaticConfig;
use std::collections::HashMap;

// Configure with CDN URL
let config = TemplateStaticConfig::new(
    "https://cdn.example.com/assets/".to_string()
);

// Convert to reinhardt_templates::StaticConfig
let static_config = reinhardt_templates::StaticConfig::from(config);
reinhardt_templates::init_static_config(static_config);
```

Templates will now generate CDN URLs:

```html
<link rel="stylesheet" href="https://cdn.example.com/assets/css/style.css" />
```

#### Advanced: Custom Manifest Loading

```rust
use reinhardt_static::TemplateStaticConfig;
use std::collections::HashMap;

// Create custom manifest mapping
let mut manifest = HashMap::new();
manifest.insert("app.js".to_string(), "app.v1.2.3.js".to_string());
manifest.insert("main.css".to_string(), "main.v1.2.3.css".to_string());

// Configure with custom manifest
let config = TemplateStaticConfig::new("/static/".to_string())
    .with_manifest(manifest);

let static_config = reinhardt_templates::StaticConfig::from(config);
reinhardt_templates::init_static_config(static_config);
```

### File Processing Pipeline

**Feature flag**: `processing`

```toml
[dependencies]
reinhardt-static = { version = "0.1.0-alpha.1", features = ["processing"] }
```

#### Basic CSS/JS Minification

```rust
use reinhardt_static::processing::{ProcessingPipeline, ProcessingConfig};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create processing configuration
    let config = ProcessingConfig::new(PathBuf::from("dist"))
        .with_minification(true)
        .with_image_optimization(false);

    // Create pipeline
    let pipeline = ProcessingPipeline::new(config);

    // Process a CSS file
    let css_content = b"body { color: red; }";
    let minified = pipeline
        .process_file(css_content, &PathBuf::from("style.css"))
        .await?;

    // Process a JavaScript file
    let js_content = b"const x = 1; // comment";
    let minified_js = pipeline
        .process_file(js_content, &PathBuf::from("app.js"))
        .await?;

    Ok(())
}
```

#### Asset Bundling

```rust
use reinhardt_static::processing::bundle::{AssetBundler, BundleConfig};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create bundler
    let mut bundler = AssetBundler::new();

    // Add files
    bundler.add_file(
        PathBuf::from("utils.js"),
        b"export const add = (a, b) => a + b;".to_vec(),
    );
    bundler.add_file(
        PathBuf::from("main.js"),
        b"import { add } from './utils.js'; console.log(add(1, 2));".to_vec(),
    );

    // Define dependency (main depends on utils)
    bundler.add_dependency(
        PathBuf::from("main.js"),
        PathBuf::from("utils.js"),
    );

    // Bundle in dependency order
    let bundle = bundler.bundle()?;

    // utils.js will be included before main.js
    println!("{}", String::from_utf8_lossy(&bundle));

    Ok(())
}
```

#### Dependency Graph

```rust
use reinhardt_static::DependencyGraph;

let mut graph = DependencyGraph::new();

// Add files and dependencies
graph.add_dependency("app.js".to_string(), "config.js".to_string());
graph.add_dependency("app.js".to_string(), "utils.js".to_string());
graph.add_dependency("config.js".to_string(), "constants.js".to_string());

// Resolve processing order
let order = graph.resolve_order();
// Result: ["constants.js", "utils.js", "config.js", "app.js"]
// (dependencies come first)
```

#### Custom Bundle Configuration

```rust
use reinhardt_static::processing::bundle::{AssetBundler, BundleConfig};
use std::path::PathBuf;

let mut bundler = AssetBundler::new();
bundler.add_file(PathBuf::from("a.js"), b"const a = 1;".to_vec());
bundler.add_file(PathBuf::from("b.js"), b"const b = 2;".to_vec());
bundler.add_file(PathBuf::from("c.js"), b"const c = 3;".to_vec());

// Bundle in custom order (ignoring dependencies)
let bundle = bundler.bundle_files(&[
    PathBuf::from("c.js"),
    PathBuf::from("a.js"),
    PathBuf::from("b.js"),
])?;
```

#### Processing with Storage Integration

```rust,no_run
# use reinhardt_static::processing::{ProcessingPipeline, ProcessingConfig};
# use reinhardt_static::ManifestStaticFilesStorage;
# use std::path::PathBuf;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create pipeline
    let config = ProcessingConfig::new(PathBuf::from("dist"))
        .with_minification(true);
    let pipeline = ProcessingPipeline::new(config);

    // Create storage
    let storage = ManifestStaticFilesStorage::new(
        PathBuf::from("dist"),
        "/static/"
    );

    // Process and save files
    let css_content = tokio::fs::read("src/style.css").await?;
    let minified = pipeline
        .process_file(&css_content, &PathBuf::from("style.css"))
        .await?;

    // Save with hashed filename
    storage.save("style.css", &minified).await?;

    Ok(())
}
```

#### Compression (Gzip and Brotli)

**Feature flag**: `compression`

```toml
[dependencies]
reinhardt-static = { version = "0.1.0-alpha.1", features = ["compression"] }
```

```rust
use reinhardt_static::processing::compress::{GzipCompressor, BrotliCompressor, CompressionConfig};
use reinhardt_static::processing::Processor;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Gzip compression
    let gzip = GzipCompressor::with_level(9);
    let input = b"Hello, World! This is test data.";
    let compressed = gzip.process(input, &PathBuf::from("test.txt")).await?;
    println!("Original: {} bytes, Compressed: {} bytes", input.len(), compressed.len());

    // Brotli compression (better compression ratio)
    let brotli = BrotliCompressor::new();
    let compressed_br = brotli.process(input, &PathBuf::from("test.txt")).await?;

    // Compression configuration
    let config = CompressionConfig::new()
        .with_gzip(true)
        .with_brotli(true)
        .with_min_size(1024)  // Only compress files > 1KB
        .add_extension("txt".to_string());

    // Check if file should be compressed
    if config.should_compress(&PathBuf::from("large.js"), 5000) {
        println!("File will be compressed");
    }

    Ok(())
}
```

#### Source Maps

**Feature flag**: `source-maps`

```toml
[dependencies]
reinhardt-static = { version = "0.1.0-alpha.1", features = ["source-maps"] }
```

```rust
use reinhardt_static::processing::sourcemap::{SourceMap, SourceMapGenerator, SourceMapMerger};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate source map
    let generator = SourceMapGenerator::new()
        .with_inline_sources(true)
        .with_source_root("/src".to_string());

    let map = generator.generate_for_file(
        &PathBuf::from("dist/app.min.js"),
        &PathBuf::from("src/app.js"),
        "const x = 1; const y = 2;"
    );

    // Save as JSON
    let map_json = map.to_json_pretty()?;
    tokio::fs::write("dist/app.min.js.map", map_json).await?;

    // Generate source map comment
    let comment = generator.generate_comment("app.min.js.map");
    println!("Add to minified file: {}", comment);

    // Merge multiple source maps
    let mut merger = SourceMapMerger::new();

    let mut map1 = SourceMap::new("file1.min.js".to_string());
    map1.add_source("src/file1.js".to_string());
    merger.add_map(map1);

    let mut map2 = SourceMap::new("file2.min.js".to_string());
    map2.add_source("src/file2.js".to_string());
    merger.add_map(map2);

    let merged = merger.merge("bundle.min.js".to_string());
    println!("Merged map has {} sources", merged.sources.len());

    Ok(())
}
```

#### Image Optimization

**Feature flag**: `image-optimization`

```toml
[dependencies]
reinhardt-static = { version = "0.1.0-alpha.1", features = ["image-optimization"] }
```

```rust
use reinhardt_static::processing::images::ImageOptimizer;
use reinhardt_static::processing::Processor;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create optimizer with quality setting (1-100)
    let optimizer = ImageOptimizer::new(85);

    // Optimize PNG
    let png_data = tokio::fs::read("image.png").await?;
    let optimized = optimizer.process(&png_data, &PathBuf::from("image.png")).await?;
    tokio::fs::write("image.optimized.png", optimized).await?;

    // Optimize JPEG
    let jpg_data = tokio::fs::read("photo.jpg").await?;
    let optimized_jpg = optimizer.process(&jpg_data, &PathBuf::from("photo.jpg")).await?;

    // Custom settings
    let optimizer_lossless = ImageOptimizer::with_settings(100, false);

    Ok(())
}
```

#### Advanced Minification (Production-Grade)

**Feature flag**: `advanced-minification`

```toml
[dependencies]
reinhardt-static = { version = "0.1.0-alpha.1", features = ["advanced-minification"] }
```

Advanced minification using OXC provides production-grade optimization including variable renaming, dead code elimination, and advanced compression.

```rust
use reinhardt_static::processing::advanced_minify::{AdvancedJsMinifier, AdvancedMinifyConfig};
use reinhardt_static::processing::Processor;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Production minifier (recommended for production builds)
    let minifier = AdvancedJsMinifier::production();

    let input = br#"
        function calculateSum(a, b) {
            console.log('Calculating sum');
            debugger;
            const result = a + b;
            return result;
        }

        const unusedVariable = 42;
    "#;

    let minified = minifier.process(input, &PathBuf::from("app.js")).await?;
    let output = String::from_utf8(minified)?;

    // Output: Minified with variable renaming, console.log removed, debugger removed
    println!("Minified: {}", output);

    Ok(())
}
```

**Custom Configuration**:

```rust
use reinhardt_static::processing::advanced_minify::{AdvancedJsMinifier, AdvancedMinifyConfig};

// Custom configuration
let config = AdvancedMinifyConfig::new()
    .with_mangle(true)              // Enable variable renaming
    .with_compress(true)            // Enable compression
    .with_drop_console(true)        // Remove console.* calls
    .with_drop_debugger(true)       // Remove debugger statements
    .with_toplevel(false)           // Don't mangle top-level vars
    .with_keep_fnames(false)        // Rename function names
    .with_keep_classnames(false);   // Rename class names

let minifier = AdvancedJsMinifier::with_config(config);
```

**Development Mode** (minimal minification):

```rust
use reinhardt_static::processing::advanced_minify::AdvancedJsMinifier;

// Development minifier (preserves readability)
let minifier = AdvancedJsMinifier::development();
```

**Configuration Presets**:

| Preset            | Mangle | Compress | Drop Console | Drop Debugger | Use Case           |
|-------------------|--------|----------|--------------|---------------|--------------------|
| `production()`    | ✓      | ✓        | ✓            | ✓             | Production builds  |
| `development()`   | ✗      | ✗        | ✗            | ✗             | Development builds |
| `new()` (default) | ✓      | ✓        | ✗            | ✓             | General use        |

**Performance Benefits**:

- **File size reduction**: 40-60% compared to basic minification
- **Variable renaming**: `myLongVariableName` → `a`
- **Dead code removal**: Eliminates unreachable code
- **Console removal**: Strips debugging statements
- **AST-based**: Safer than regex-based minification

**Integration with Processing Pipeline**:

```rust
use reinhardt_static::processing::{ProcessingPipeline, ProcessingConfig};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Pipeline with advanced minification
    let config = ProcessingConfig::new(PathBuf::from("dist"))
        .with_minification(true)        // Basic minification
        .with_advanced_minification(true); // Advanced minification (requires feature)

    let pipeline = ProcessingPipeline::new(config);

    let js_content = tokio::fs::read("src/app.js").await?;
    let optimized = pipeline.process_file(&js_content, &PathBuf::from("app.js")).await?;

    tokio::fs::write("dist/app.min.js", optimized).await?;

    Ok(())
}
```

### Development Server Features

**Feature flag**: `dev-server`

```toml
[dependencies]
reinhardt-static = { version = "0.1.0-alpha.1", features = ["dev-server"] }
```

#### File Watching and Auto-Reload

```rust,no_run
use reinhardt_static::{DevServerConfig, FileWatcher, AutoReload};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create development server configuration
    let config = DevServerConfig::new()
        .with_watch_path(PathBuf::from("./static"))
        .with_watch_path(PathBuf::from("./templates"))
        .with_auto_reload(true)
        .with_reload_port(35729);

    // Create file watcher
    let paths = vec![
        PathBuf::from("./static"),
        PathBuf::from("./templates"),
    ];
    let mut watcher = FileWatcher::new(&paths)?;

    // Create auto-reload system
    let reload = AutoReload::new();

    // Listen for file changes
    loop {
        if let Some(event) = watcher.next_event().await {
            println!("File change detected: {:?}", event);
            reload.handle_watch_event(event);
        }
    }
}
```

#### Auto-Reload with Broadcast Channels

```rust
use reinhardt_static::{AutoReload, ReloadEvent};

#[tokio::main]
async fn main() {
    let reload = AutoReload::new();

    // Client subscribes to reload events
    let mut rx = reload.subscribe();

    // Simulate file change
    reload.trigger_reload();

    // Receive event
    if let Ok(event) = rx.try_recv() {
        match event {
            ReloadEvent::Reload => println!("Full page reload"),
            ReloadEvent::ReloadFile(path) => println!("Reload file: {}", path),
            ReloadEvent::ClearCache => println!("Clear cache"),
        }
    }
}
```

#### Smart CSS Reloading

```rust
use reinhardt_static::{AutoReload, WatchEvent};
use std::path::PathBuf;

let reload = AutoReload::new();

// CSS file changed - no full page reload needed
let event = WatchEvent::Modified(PathBuf::from("./static/css/main.css"));
reload.handle_watch_event(event);
// Sends ReloadEvent::ReloadFile("/static/css/main.css")

// JavaScript file changed - full page reload
let event = WatchEvent::Modified(PathBuf::from("./static/js/app.js"));
reload.handle_watch_event(event);
// Sends ReloadEvent::Reload
```

#### Development Error Pages

```rust
use reinhardt_static::DevelopmentErrorHandler;
use std::io;

let handler = DevelopmentErrorHandler::new()
    .with_stack_trace(true)
    .with_source_context(true)
    .with_context_lines(5);

let error = io::Error::new(io::ErrorKind::NotFound, "File not found");

// Generate HTML error page
let html = handler.format_error(&error);
// Returns detailed error page with:
// - Error message
// - Stack trace
// - Error chain
// - Helpful styling

// Or generate plain text
let text = handler.format_error_text(&error);
```

#### Client Connection Tracking

```rust
use reinhardt_static::AutoReload;

#[tokio::main]
async fn main() {
    let reload = AutoReload::new();

    // When a client connects
    reload.add_client().await;
    println!("Connected clients: {}", reload.client_count().await);

    // When a client disconnects
    reload.remove_client().await;
    println!("Connected clients: {}", reload.client_count().await);
}
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.