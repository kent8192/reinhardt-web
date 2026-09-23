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
- [Building a Complete Asset Set](#building-a-complete-asset-set)
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

## Building a Complete Asset Set

`buildstatic` uses the same settings precedence as static serving. `[static]`
and typed `[static_files]` accept `url` and `root`; legacy flat `static_url`,
`static_root`, and `staticfiles_dirs` remain supported. Relative filesystem paths
are resolved against the selected project directory. There is no independent
final output-directory flag.

```toml
staticfiles_dirs = ["assets"]

[static]
url = "/console/static/"
root = "staticfiles"
```

```text
manage buildstatic
manage buildstatic --pages --package my-dashboard --release
manage buildstatic --pages --package my-dashboard --features theme --mode development
manage buildstatic --dry-run
```

Ordinary packaging needs no WASM tools. `--pages` selects the same Cargo package
and features for component CSS extraction and WASM compilation, then captures the
complete wasm-bindgen web output from a private intermediate directory. The JS
entry's actual WASM reference is validated; publication does not guess its name.
`--release` (or `--profile <cargo-profile>`) controls compilation, independently
of `--mode` and its cache contract.

To reuse existing build stages, supply their complete materialized outputs:

```text
manage buildstatic --static-manifest /build/collected/manifest.json \
  --pages-dir /build/wasm-dist --pages-entry dashboard.js \
  --pages-document index.html --pages-style css/site.css
```

`--static-manifest` replaces source collection and component style extraction;
`--pages-dir` replaces Cargo/wasm-bindgen execution and requires `--pages-entry`.
The Pages directory includes snippets, maps, and other companions. Ordinary
styles remain classified under `css/`, including `__reinhardt__/components.css`.
`--pages-document` selects a collected HTML render template; legacy collectstatic's
explicit `index.html` beside its manifest is also supported. Repeat `--pages-style`
to select additional styles in cascade order. Generated component CSS is included
once when available. Template stylesheet order is preserved during rendering.

Dry-run performs no compilation, vendor download, output writes, or activation.
It lists known assignments, collisions, and checks requiring generated content;
it deliberately does not claim a final build ID. Physical discovery rejects
symlinks/special files and skips generation directories, manifests, locks, and
private staging. Materialize symlinked sources inside the configured input root.

Legacy version 1, `paths`, `files`, and flat manifests use explicit import adapters.
Their physical names map back to the original logical namespace before rewriting.
Version 2 imports validate the full source generation and retain its entrypoints.
The framework Pages loader is regenerated when importing a version 2 Pages build.
`ManifestStaticFilesStorage` can read version 2 paths, but its legacy writer rejects
writes after loading a version 2 manifest; use `buildstatic` to publish changes.
Legacy `collectstatic` refuses to clear or overwrite version 2 output. For a root
containing `staticfiles.json`, select a separate `STATIC_ROOT` for migration (or
explicitly archive the old manifest first); `buildstatic` never silently deletes
or renames a competing manifest. Keep manual packaging until the new manifest
serving path and a real browser load have passed for the application.

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

With the native `asset-publication` feature, `AssetPipeline` captures inputs,
discovers their dependencies, assigns all paths, rewrites references, and hashes
the final bytes into one generation. Discovery order, source directory location,
and `STATIC_URL` do not determine that identity. Mode, processor versions/options,
stylesheet order, and every published asset do.

```rust
use reinhardt_utils::staticfiles::publication::{
    AssetInput, AssetMode, AssetPipeline,
};

let mut pipeline = AssetPipeline::new();
pipeline.add_input(AssetInput::bytes("images/logo.svg", b"<svg/>".to_vec()))?;
pipeline.add_input(AssetInput::bytes(
    "theme.css", b".logo { mask: url('images/logo.svg'); }".to_vec(),
))?;
let complete = pipeline.prepare(AssetMode::Production)?;
assert!(complete.manifest().paths["images/logo.svg"].ends_with("/vectors/logo.svg"));
# Ok::<(), reinhardt_utils::staticfiles::publication::AssetBuildError>(())
```

Built-in reference processing supports UTF-8 CSS `url()`/`@import`, literal ESM
imports/exports and dynamic imports, `new URL(literal, import.meta.url)`,
wasm-bindgen snippets, and local source-map directives. HTML processing covers
asset attributes, `srcset`, inline CSS/module scripts, and
`{{ static_url('logical/path') }}` expressions; navigation and form actions retain
their meaning. Query strings, fragments, external URLs, and data/blob URLs are
preserved. Local dependencies must be included. Computed imports, import maps,
CommonJS loading, and HTML `base href` need a custom processor or prior bundling;
the publisher reports these cases instead of guessing filenames.

Custom `AssetProcessor` implementations declare extra inputs during `prepare`,
local dependencies during `analyze`, and final references during `rewrite`.
Rewriting receives generation-relative paths, so import cycles work without
recursive hashes. The pipeline reanalyzes output to reject undeclared references.
`register_byte_processor` also accepts existing asynchronous `Processor`
implementations with an explicit version and options. A transform that changes
mapped code must preserve positions or return an updated source map.

Recognized gzip/Brotli representations must agree with their canonical source.
An encoded-only input creates that source, and enabled variants are regenerated
deterministically after rewriting. Source maps retain original source contents
and remap generated positions through URL edits. Unsupported or unverifiable map
transformations fail rather than publishing stale positions.

`AssetPublisher::new(configured_static_root).publish(complete)` writes a private
staging directory on the destination filesystem, flushes all files, promotes the
complete `builds/<build-id>/` directory, and atomically replaces `manifest.json`
last. The immutable directory contains an identical manifest. An OS file lock
serializes publishers; a killed process releases the lock automatically. Reusing
an existing ID requires that its manifest and every output still match. Atomic
activation currently requires Unix filesystem rename semantics; unsupported
platforms return an error before writing output.

`ManifestSnapshot::load` verifies the manifest, complete inventory, expected mode,
sizes, and digests. `SnapshotOptions::expected_build_id` additionally pins a
deployment's required ID. `ManifestStore` holds the active and retained snapshots;
clone `active()` once per request or select `generation(id)` for an old asset URL.
`reload()` validates a candidate before swapping state, and a failed reload
preserves the last-good snapshot. These readers are available with `staticfiles`
without the packaging parsers.

Retain the asset root across deployments. Activation never deletes old
generations, so a page already using generation A can continue loading its assets
after B activates. Rollback selects A's complete immutable manifest again. Choose
a retention policy that accounts for open pages and deployed entry documents;
replacing the entire asset volume loses that compatibility. Production and
development publications must use different roots. Abandoned `.asset-staging-*`
directories are reported and ignored, while incomplete or modified retained
generations fail validation with a diagnostic.

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
