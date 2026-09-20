+++
title = "Serving Static Files"
weight = 10
+++

# Serving Static Files

Guide to serving static files (CSS, JavaScript, images, etc.).

## Table of Contents

- [Basic Setup](#basic-setup)
- [Storage Backends](#storage-backends)
- [Unified Asset Classification](#unified-asset-classification)
- [Development vs Production](#development-vs-production)
- [Path Resolution](#path-resolution)
- [Cache Strategies](#cache-strategies)

---

## Basic Setup

### StaticFilesMiddleware

Use `StaticFilesMiddleware` for serving static files.

```rust
use reinhardt_utils::staticfiles::middleware::StaticFilesMiddleware;
use reinhardt_utils::staticfiles::storage::StaticFilesConfig;

let config = StaticFilesConfig {
    static_root: "static".into(),
    static_url: "/static/".to_string(),
    staticfiles_dirs: vec![
        "app/static".into(),
        "vendor/static".into(),
    ],
    media_url: Some("/media/".to_string()),
};

let middleware = StaticFilesMiddleware::new(config);
```

> **Note**: There are two `StaticFilesConfig` types in `reinhardt::staticfiles`:
> - `storage::StaticFilesConfig` - For storage configuration (`static_root`, `static_url`, `staticfiles_dirs`, `media_url`)
> - `middleware::StaticFilesConfig` - For middleware configuration (`root_dir`, `url_prefix`, `spa_mode`, etc.)
>
> This example uses the storage version. The storage config is also re-exported at `reinhardt_utils::staticfiles::StaticFilesConfig`.

---

## Storage Backends

### FileSystemStorage

Saves files to local filesystem.

```rust
use reinhardt_utils::staticfiles::storage::FileSystemStorage;

let storage = FileSystemStorage::new("/var/www/static", "https://cdn.example.com/static");

// Save file
let url = storage.save("css/style.css", b"body { margin: 0; }").await?;

// Check if file exists
if storage.exists("css/style.css") {
    // Open file
    let content = storage.open("css/style.css").await?;
}

// Get URL
let url = storage.url("css/style.css");
// Returns: "https://cdn.example.com/static/css/style.css"

// Delete file
storage.delete("css/style.css").await?;
```

### MemoryStorage

In-memory storage (for testing).

```rust
use reinhardt_utils::staticfiles::storage::MemoryStorage;

let storage = MemoryStorage::new("/static/");

// Save file
let url = storage.save("test.txt", b"Hello, World!").await?;

// Get file
let content = storage.open("test.txt").await?;
```

### S3Storage

Saves files to AWS S3 (requires `s3` feature).

```rust
use reinhardt_utils::staticfiles::storage::{S3Storage, S3Config};

async fn example() -> Result<(), Box<dyn std::error::Error>> {
let config = S3Config {
    bucket: "my-bucket".to_string(),
    region: "us-east-1".to_string(),
    key_prefix: Some("static/".to_string()),
    cdn_domain: Some("https://cdn.example.com".to_string()),
};

let storage = S3Storage::new(config);

// Save file
let url = storage.save("images/logo.png", image_bytes).await?;
// Returns: "https://cdn.example.com/static/images/logo.png"
Ok(())
}
```

### AzureBlobStorage

Saves files to Azure Blob Storage (requires `azure` feature).

```rust
use reinhardt_utils::staticfiles::storage::{AzureBlobStorage, AzureBlobConfig};

async fn example() -> Result<(), Box<dyn std::error::Error>> {
let config = AzureBlobConfig::new(
    "mystorageaccount".to_string(),
    "static-files".to_string(),
)
.with_account_key("ACCOUNT_KEY".to_string());

let storage = AzureBlobStorage::new(config).await?;
Ok(())
}
```

Blob names may contain nested paths such as `images/logo.png`, but `.` and `..`
path segments are rejected so storage operations remain inside the configured
container and prefix. Generated URLs percent-encode ambiguous characters such as
percent signs and backslashes, and return an empty string for rejected names.

### GcsStorage

Saves files to Google Cloud Storage (requires `gcs` feature).

```rust
use reinhardt_utils::staticfiles::storage::{GcsStorage, GcsConfig};

async fn example() -> Result<(), Box<dyn std::error::Error>> {
let config = GcsConfig::new(
    "my-bucket".to_string(),
    "my-project-id".to_string(),
)
.with_prefix("static".to_string());

let storage = GcsStorage::new(config).await?;
Ok(())
}
```

---

## Unified Asset Classification

`reinhardt_utils::staticfiles::publication` provides one versioned manifest reader
for collected assets and Pages outputs. Version 2 describes a complete generation;
version 1, unversioned `paths`, legacy `files`, and flat mappings retain their
legacy capabilities. Invalid values and duplicate names are errors. When both
`manifest.json` and `staticfiles.json` exist, select a manifest explicitly before
migrating; neither file silently takes precedence.

The classifier preserves each logical name and assigns a publication path from
its producer and extension. Pages-generated JavaScript, WASM, snippets, and maps
stay together under `pages/`. Ordinary assets use the following categories:

| Directory | Formats |
|---|---|
| `videos/` | MP4, WebM, MOV, M4V, OGV, AVI, MKV, MPEG, MPG |
| `vectors/` | SVG, SVGZ, EPS, AI |
| `images/` | PNG, JPEG, GIF, WebP, AVIF, APNG, BMP, TIFF, ICO, HEIC, HEIF, JXL |
| `css/` | CSS |
| `js/` | JS, MJS, CJS |
| `fonts/` | WOFF, WOFF2, TTF, OTF, EOT |
| `audio/` | MP3, WAV, OGG, OGA, FLAC, M4A, AAC, OPUS, AIFF, AIF |
| `other/` | Unknown and extensionless files, ordinary JSON/PDF, and non-Pages WASM |

Matching ignores extension case; filenames keep their original case. Recognized
compressed assets and source maps inherit the underlying category: `app.js.gz`
belongs to JS and `theme.css.map.br` to CSS. Download archives such as
`backup.tar.gz` remain opaque `other/` files. Classification does not imply browser
support or convert CommonJS to ESM.

```rust
use reinhardt_utils::staticfiles::publication::{
    AssetCategory, AssetClassifier, AssetProducer,
};

let mut classifier = AssetClassifier::default();
classifier.register_extension("lottie", AssetCategory::Vectors)?;
let (_, path) = classifier.classify("images/brand/logo.svg", AssetProducer::Static)?;
assert_eq!(path, "vectors/brand/logo.svg");
let (_, glue) = classifier.classify("dashboard.js", AssetProducer::Pages)?;
assert_eq!(glue, "pages/dashboard.js");
# Ok::<(), reinhardt_utils::staticfiles::publication::AssetBuildError>(())
```

At most one leading ordinary category directory is removed before adding the
selected category. Nested application/vendor namespaces are preserved. Duplicate
logical names or case-insensitive output collisions fail with both source names;
choose distinct logical namespaces to resolve them. Custom mappings cannot claim
the reserved `pages` category. Generated component CSS retains the logical name
`__reinhardt__/components.css` and is assigned to `css/__reinhardt__/components.css`.

Generation-bound URL resolution uses
`reinhardt_core::types::static_assets::AssetUrlSnapshot`, identically on native and
WASM. Its mapping contains `builds/<build-id>/...` paths relative to the configured
static root. `STATIC_URL` supplies only the public URL prefix, including nested
prefixes or an HTTP(S) origin. Unknown logical names return errors. Existing
prefix-only resolvers remain available for legacy pipelines.

## Development vs Production

### Development

Use `StaticFileHandler` to serve local files directly.

```rust
use reinhardt_utils::staticfiles::handler::StaticFileHandler;

let handler = StaticFileHandler::new("/path/to/static");
```

### Production

Use `HashedFileStorage` for cache busting.

```rust
use reinhardt_utils::staticfiles::storage::HashedFileStorage;

let storage = HashedFileStorage::new("/var/www/static", "https://cdn.example.com/static");

// Hash is automatically added on save
// "style.css" -> "style.a1b2c3d4e5f6.css"
let url = storage.save("style.css", css_bytes).await?;
```

### ManifestStaticFilesStorage

Versioned files using manifest (similar to Django's `ManifestStaticFilesStorage`).

```rust
use reinhardt_utils::staticfiles::storage::{ManifestStaticFilesStorage, Manifest};

let manifest = Manifest {
    version: "v1.0.0".to_string(),
    files: vec![
        ("css/style.css".to_string(), "css/style.v1.css".to_string()),
    ],
};

let storage = ManifestStaticFilesStorage::new(
    "/var/www/static",
    "https://cdn.example.com/static",
    manifest,
);

// Get versioned path from manifest
let url = storage.url("css/style.css");
// Returns: "https://cdn.example.com/static/css/style.v1.css"
```

---

## Path Resolution

### PathResolver

Resolves paths for static files.

```rust
use reinhardt_utils::staticfiles::PathResolver;

let resolver = PathResolver::new(vec![
    "/app/static".into(),
    "/vendor/static".into(),
]);

// Resolve path
if let Some(path) = resolver.resolve("css/style.css") {
    println!("Found at: {}", path.display());
}
```

### StaticFilesFinder

Finds static files across multiple directories.

```rust
use reinhardt_utils::staticfiles::StaticFilesFinder;
use std::path::PathBuf;

let finder = StaticFilesFinder::new(vec![
    PathBuf::from("static"),
    PathBuf::from("assets"),
]);

// Find specific file
if let Ok(path) = finder.find("css/style.css") {
    println!("Found at: {}", path.display());
}

// Get all files
let all_files = finder.find_all();
// Returns: ["css/style.css", "js/app.js", "images/logo.png", ...]
```

---

## Cache Strategies

### CacheControlMiddleware

Controls caching for static files.

```rust
use reinhardt_utils::staticfiles::{CacheControlMiddleware, CacheControlConfig};

let config = CacheControlConfig {
    public: true,
    max_age: 3600,        // 1 hour
    s_maxage: Some(86400), // 1 day on CDN
    immutable: true,       // Files never change
};

let middleware = CacheControlMiddleware::new(config);
```

### Cache Directives

```rust
use reinhardt_utils::staticfiles::CacheDirective;

// Typical static files (cache for 1 hour)
let directive = CacheDirective::public()
    .with_max_age(3600);

// Versioned files (cache for 1 year)
let immutable = CacheDirective::public()
    .with_max_age(31536000)
    .with_immutable(true);

// HTML files (don't cache)
let no_cache = CacheDirective::no_cache();
```

---

## Router Integration

```rust
use reinhardt::ServerRouter;
use reinhardt_utils::staticfiles::middleware::StaticFilesMiddleware;
use reinhardt_utils::staticfiles::storage::StaticFilesConfig;

let router = ServerRouter::new()
    .with_middleware(StaticFilesMiddleware::new(
        StaticFilesConfig {
            static_root: "static".into(),
            static_url: "/static/".to_string(),
            ..Default::default()
        }
    ));
```

---

## See Also

- [CORS Configuration](../cors/)
- [Middleware Creation](../middleware-creation/)
- [Media Files](https://docs.rs/reinhardt-utils/latest/reinhardt_utils/static/index.html)
