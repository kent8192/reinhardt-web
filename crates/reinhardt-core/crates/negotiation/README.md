# reinhardt-negotiation

Django REST Framework-style content negotiation system

## Overview

A content negotiation system that determines the optimal response format based on client priorities. Handles Accept header parsing, media type matching, and renderer selection.

## Implemented ✓

### Core Components

#### MediaType

- **MIME Type Representation**: Media types in `type/subtype` format
- **Parameter Support**: Preserves parameters like `charset=utf-8`
- **Quality Values (q-values)**: `q` parameter indicating Accept header priority
- **Parse Capability**: Converts strings to MediaType objects
- **Wildcard Matching**: Supports wildcards like `*/*`, `application/*`
- **Priority Calculation**: Assigns higher priority to more specific media types
- **Full String Representation**: Generates complete strings including parameters

#### AcceptHeader

- **Accept Header Parser**: Parses HTTP Accept headers
- **Sorting by Quality Values**: Automatically sorts by descending q-values
- **Optimal Match Search**: Selects the best match from available media types
- **Empty Accept Header Support**: Handles cases when Accept header is absent

#### ContentNegotiator

- **Renderer Selection**: Selects optimal renderer based on Accept header
- **Default Media Type**: Configurable default value (default: `application/json`)
- **Negotiation**: Matches client requests with available formats
- **Format Parameter Support**: Supports query parameters like `?format=json`
- **Renderer Filtering**: Filters renderers by format name
- **Parameterized Accept Header**: Supports detailed specifications like `application/json; indent=8`
- **Wildcard Processing**: Uses first renderer when `*/*` is specified

### Django REST Framework Compatible Features

- **BaseContentNegotiation trait**: Abstract interface equivalent to DRF's `BaseContentNegotiation`
- **select_renderer method**: Renderer selection logic
- **select_parser method**: Parser selection logic (basic implementation)
- **NegotiationError**: Error type for negotiation failures

### Advanced Features

#### ContentTypeDetector

- **Automatic Content-Type Detection**: Automatically infers media type from request body
- **Multiple Format Support**: Detects JSON, XML, YAML, Form Data
- **Fallback Configuration**: Configurable default media type when detection fails

#### LanguageNegotiator

- **Accept-Language Support**: Parses Accept-Language headers
- **Quality Value Consideration**: Selects preferred language based on q-factor
- **Region Support**: Supports language codes and regions (e.g., en-US, ja-JP)
- **Fallback Language**: Default language setting when no match is found

#### EncodingNegotiator

- **Accept-Encoding Support**: Parses Accept-Encoding headers
- **Multiple Compression Methods**: Supports Gzip, Brotli, Deflate, Identity
- **Priority Configuration**: Customizable server-side compression method priorities
- **Quality Value Negotiation**: Selection based on client quality values

#### NegotiationCache

- **Negotiation Result Cache**: Fast cache based on Accept headers
- **TTL Support**: Cache expiration time configuration
- **Multiple Header Support**: Cache for combinations of Accept, Accept-Language, Accept-Encoding
- **Automatic Eviction**: Cache size limits and automatic removal of old entries

### Features

- **Type Safety**: Safe implementation leveraging Rust's type system
- **Zero-Cost Abstractions**: Design that doesn't sacrifice performance
- **Detailed Documentation**: Documentation with doctests for all public APIs
- **Comprehensive Testing**: Integration tests that replicate DRF behavior

## Usage Examples

### Basic Content Negotiation

```rust
use reinhardt_negotiation::{ContentNegotiator, MediaType};

let negotiator = ContentNegotiator::new();
let available = vec![
    MediaType::new("application", "json"),
    MediaType::new("text", "html"),
];

// Negotiation based on Accept header
let result = negotiator.negotiate("text/html, application/json", &available);
assert_eq!(result.subtype, "html"); // First matching html is selected
```

### Format Parameter Selection

```rust
use reinhardt_negotiation::{ContentNegotiator, MediaType};

let negotiator = ContentNegotiator::new();
let available = vec![
    MediaType::new("application", "json"),
    MediaType::new("text", "html"),
];

// Selection via query parameter like ?format=json
let result = negotiator.select_by_format("json", &available);
assert_eq!(result.unwrap().subtype, "json");
```

### Renderer Selection

```rust
use reinhardt_negotiation::{ContentNegotiator, MediaType};

let negotiator = ContentNegotiator::new();
let renderers = vec![
    MediaType::new("application", "json"),
    MediaType::new("text", "html"),
];

let result = negotiator.select_renderer(
    Some("application/json"),
    &renderers
);
assert!(result.is_ok());
let (media_type, media_type_str) = result.unwrap();
assert_eq!(media_type.subtype, "json");
```

### Content-Type Detection

```rust
use reinhardt_negotiation::detector::ContentTypeDetector;

let detector = ContentTypeDetector::new();

// JSON detection
let json_body = r#"{"name": "John", "age": 30}"#;
let media_type = detector.detect(json_body.as_bytes());
assert_eq!(media_type.subtype, "json");

// XML detection
let xml_body = r#"<?xml version="1.0"?><root><name>John</name></root>"#;
let media_type = detector.detect(xml_body.as_bytes());
assert_eq!(media_type.subtype, "xml");

// YAML detection
let yaml_body = "name: John\nage: 30";
let media_type = detector.detect(yaml_body.as_bytes());
assert_eq!(media_type.subtype, "yaml");
```

### Language Negotiation

```rust
use reinhardt_negotiation::language::{LanguageNegotiator, Language};

let negotiator = LanguageNegotiator::new();
let available = vec![
    Language::new("en"),
    Language::new("fr"),
    Language::new("ja"),
];

// Selection based on Accept-Language header
let result = negotiator.negotiate("fr;q=0.9, en;q=0.8", &available);
assert_eq!(result.code, "fr");

// Support for languages with regions
let result = negotiator.negotiate("en-US", &available);
assert_eq!(result.code, "en");
```

### Encoding Negotiation

```rust
use reinhardt_negotiation::encoding::{EncodingNegotiator, Encoding};

let negotiator = EncodingNegotiator::new();
let available = vec![Encoding::Gzip, Encoding::Brotli, Encoding::Identity];

// Selection based on Accept-Encoding header
let result = negotiator.negotiate("br;q=1.0, gzip;q=0.9", &available);
assert_eq!(result, Encoding::Brotli);

// Negotiation by quality values
let result = negotiator.negotiate("gzip;q=0.5, identity;q=1.0", &available);
assert_eq!(result, Encoding::Identity);
```

### Caching Negotiation Results

```rust
use reinhardt_negotiation::cache::{NegotiationCache, CacheKey};
use reinhardt_negotiation::{ContentNegotiator, MediaType};

let mut cache: NegotiationCache<MediaType> = NegotiationCache::new();
let negotiator = ContentNegotiator::new();
let available = vec![
    MediaType::new("application", "json"),
    MediaType::new("text", "html"),
];

// Cache negotiation results
let key = CacheKey::new("application/json");
let result = cache.get_or_compute(&key, || {
    negotiator.negotiate("application/json", &available)
});

assert_eq!(result.subtype, "json");

// Second time retrieves from cache (skips negotiation processing)
let cached = cache.get(&key);
assert!(cached.is_some());
```

### Complete Request Processing

```rust
use reinhardt_negotiation::prelude::*;

// Setup all negotiators
let content_negotiator = ContentNegotiator::new();
let language_negotiator = LanguageNegotiator::new();
let encoding_negotiator = EncodingNegotiator::new();
let detector = ContentTypeDetector::new();

// Request processing
let request_body = r#"{"user": "john"}"#;
let accept = "application/json";
let accept_language = "fr";
let accept_encoding = "gzip";

// 1. Detect Content-Type of request body
let detected_type = detector.detect(request_body.as_bytes());
assert_eq!(detected_type.subtype, "json");

// 2. Negotiate response Content-Type
let media_types = vec![
    MediaType::new("application", "json"),
    MediaType::new("text", "html"),
];
let response_media = content_negotiator.negotiate(accept, &media_types);

// 3. Negotiate language
let languages = vec![Language::new("en"), Language::new("fr")];
let response_lang = language_negotiator.negotiate(accept_language, &languages);

// 4. Negotiate encoding
let encodings = vec![Encoding::Gzip, Encoding::Identity];
let response_encoding = encoding_negotiator.negotiate(accept_encoding, &encodings);
```

## License

This crate is part of the reinhardt project and is dual-licensed under Apache License 2.0 or MIT License.
