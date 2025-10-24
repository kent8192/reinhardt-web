# reinhardt-parsers

Request body parsing for the Reinhardt framework, inspired by Django REST Framework parsers.

## Overview

Provides a comprehensive set of parsers for handling different request content types in web applications. Each parser implements the `Parser` trait and can handle specific MIME types with automatic content-type negotiation through the `ParserRegistry`.

## Implemented ✓

### Core Parser System

#### `Parser` Trait

- **Async trait** for parsing request bodies with content-type negotiation
- `media_types()` - Returns list of supported MIME types
- `parse()` - Asynchronous parsing of request body into `ParsedData`
- `can_parse()` - Automatic content-type matching with wildcard support

#### `ParserRegistry`

- **Central registry** for managing multiple parsers
- Automatic parser selection based on request Content-Type header
- Builder pattern for registering parsers
- Support for custom parser implementations

#### `MediaType`

- **Content-Type parsing and manipulation**
- Support for MIME type parameters (e.g., `charset=utf-8`)
- Wildcard matching (`application/*`, `*/json`, `*/*`)
- RFC-compliant parsing from Content-Type header strings

#### `ParsedData` Enum

Unified representation of parsed request data:

- `Json(Value)` - JSON data parsed with `serde_json`
- `Form(HashMap<String, String>)` - URL-encoded form data
- `MultiPart { fields, files }` - Multipart form data with file uploads
- `File(UploadedFile)` - Raw file upload

#### `UploadedFile`

File upload representation with:

- Field name and optional filename
- Content-Type detection
- File size tracking
- Binary data storage using `Bytes`

### JSON Parser (`JSONParser`)

#### Basic Features

- **Content-Type**: `application/json`, `application/*+json`
- Parse JSON request bodies using `serde_json`
- Returns `ParsedData::Json(Value)` for flexible data handling

#### Advanced Options

- **Empty body handling** - Configurable via `allow_empty()`
  - Default: Reject empty bodies
  - Optional: Return `null` for empty requests
- **Strict mode** - Configurable via `strict()`
  - Default: Enabled (Django REST Framework behavior)
  - Rejects non-finite floats (`Infinity`, `-Infinity`, `NaN`)
  - Can be disabled for lenient parsing

#### Validation

- Recursive validation for nested structures (objects and arrays)
- Detailed error messages for malformed JSON

### Form Parser (`FormParser`)

#### Basic Features

- **Content-Type**: `application/x-www-form-urlencoded`
- Parse HTML form data using `serde_urlencoded`
- Returns `ParsedData::Form(HashMap<String, String>)`

#### URL Encoding Support

- Automatic percent-decoding of form values
- Handles special characters and spaces correctly
- Empty body returns empty HashMap (not an error)

### MultiPart Parser (`MultiPartParser`)

#### Basic Features

- **Content-Type**: `multipart/form-data`
- Handle file uploads and form fields in single request
- Returns `ParsedData::MultiPart { fields, files }`
- Built on `multer` crate for robust parsing

#### File Upload Features

- **Multiple file uploads** in single request
- Separate handling of form fields vs. file fields
- Content-Type detection per file
- Original filename preservation

#### Size Limits

- **Per-file size limit** - `max_file_size()`
- **Total upload size limit** - `max_total_size()`
- Detailed error messages when limits exceeded

#### Boundary Parsing

- Automatic extraction from Content-Type header
- RFC-compliant multipart boundary handling

### File Upload Parser (`FileUploadParser`)

#### Basic Features

- **Content-Type**: `application/octet-stream`, `*/*`
- Raw file upload without multipart overhead
- Returns `ParsedData::File(UploadedFile)`
- Configurable field name

#### Filename Extraction

- **Standard filename** - Parse from `Content-Disposition` header
- **RFC2231 encoded filenames** - Support for international characters
  - Format: `filename*=utf-8''%encoded_name`
  - Language tag support
  - Precedence over standard filename
- Automatic URL decoding of encoded filenames

#### Size Control

- **Maximum file size** - Configurable via `max_file_size()`
- Detailed error reporting when limit exceeded

### Error Handling

- Unified error types using `reinhardt_exception::Error`
- Type aliases: `ParseError` and `ParseResult<T>`
- Detailed error messages for debugging
- Integration with framework exception system

## Planned

### Additional Parsers

- **XML Parser** - For `application/xml` and `text/xml`
- **YAML Parser** - For `application/x-yaml`
- **MessagePack Parser** - For binary message format
- **Protobuf Parser** - For Protocol Buffers

### Enhanced Features

- **Streaming parsing** - For large file uploads without loading entire body into memory
- **Content negotiation** - Automatic parser selection based on Accept header
- **Custom validators** - Per-parser validation hooks
- **Schema validation** - JSON Schema, XML Schema support
- **Compression support** - Gzip, Brotli, Deflate decompression

### Performance Optimizations

- **Zero-copy parsing** - Where possible with current parser implementations
- **Parallel multipart processing** - Parse multiple files concurrently
- **Memory pooling** - Reuse buffers for repeated parsing operations

## Usage Example

```rust
use bytes::Bytes;
use reinhardt_parsers::{
    JSONParser, FormParser, MultiPartParser, FileUploadParser,
    ParserRegistry,
};

// Create a registry with all parsers
let registry = ParserRegistry::new()
    .register(JSONParser::new())
    .register(FormParser::new())
    .register(MultiPartParser::new().max_file_size(10 * 1024 * 1024))
    .register(FileUploadParser::new("upload"));

// Parse a JSON request
let json_body = Bytes::from(r#"{"name": "test"}"#);
let parsed = registry
    .parse(Some("application/json"), json_body)
    .await?;

// Parse a form request
let form_body = Bytes::from("name=test&value=123");
let parsed = registry
    .parse(Some("application/x-www-form-urlencoded"), form_body)
    .await?;
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE))
- MIT license ([LICENSE-MIT](../../LICENSE-MIT))

at your option.
