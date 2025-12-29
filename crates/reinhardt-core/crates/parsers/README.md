# reinhardt-parsers

Request body parsing for the Reinhardt framework, inspired by Django REST Framework parsers.

## Overview

Provides a comprehensive set of parsers for handling different request content types in web applications. Each parser implements the `Parser` trait and can handle specific MIME types with automatic content-type negotiation through the `ParserRegistry`.

## Installation

Add `reinhardt` to your `Cargo.toml`:

```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["parsers"] }

# Or use a preset:
# reinhardt = { version = "0.1.0-alpha.1", features = ["standard"] }  # Recommended
# reinhardt = { version = "0.1.0-alpha.1", features = ["full"] }      # All features
```

Then import parser features:

```rust
use reinhardt::core::parsers::{
    JSONParser, FormParser, MultiPartParser, FileUploadParser,
    XMLParser, YamlParser, MessagePackParser, ProtobufParser,
    ParserRegistry, Parser, ParsedData,
};
```

**Note:** Parser features are included in the `standard` and `full` feature presets.

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
- `Xml(Value)` - XML data parsed with `quick-xml`
- `Yaml(Value)` - YAML data parsed with `serde_yaml`
- `Form(HashMap<String, String>)` - URL-encoded form data
- `MultiPart { fields, files }` - Multipart form data with file uploads
- `File(UploadedFile)` - Raw file upload
- `MessagePack(Value)` - MessagePack binary data parsed with `rmp_serde`
- `Protobuf(Value)` - Protocol Buffers binary data with dynamic schema support

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

### XML Parser (`XMLParser`)

#### Basic Features

- **Content-Type**: `application/xml`, `text/xml`
- Parse XML request bodies using `quick-xml`
- Returns `ParsedData::Xml(Value)` as JSON-like structure

#### Configuration Options

- **Attribute handling** - Configurable via `include_attributes()`
  - Custom attribute prefix (default: `@`)
  - Example: `<tag id="123">` → `{"@id": "123"}`
- **Text content key** - Configurable via `text_key()` (default: `#text`)
- **CDATA preservation** - Configurable via `preserve_cdata()`
- **Whitespace trimming** - Configurable via `trim_text()`
- **Type parsing** - Optional number/boolean parsing via `parse_numbers()`, `parse_booleans()`

#### Advanced Features

- Nested element support with automatic array conversion for repeated elements
- Empty element handling
- Namespace support (implicit via quick-xml)

### YAML Parser (`YamlParser`)

#### Basic Features

- **Content-Type**: `application/yaml`, `application/x-yaml`
- Parse YAML request bodies using `serde_yaml`
- Returns `ParsedData::Yaml(Value)` for flexible data handling

#### Advanced Options

- **Empty body handling** - Configurable via `allow_empty()`
  - Default: Reject empty bodies
  - Optional: Return `null` for empty requests

#### Supported Data Types

- Nested structures (maps and sequences)
- Arrays
- Boolean values (`true`, `false`)
- Number types (integer and float)
- Null values
- Multiline strings (literal and folded)

### Compression Parser (`CompressedParser`)

#### Basic Features

- **Transparent decompression wrapper** for any parser
- Automatic detection via `Content-Encoding` header
- Returns same `ParsedData` type as wrapped parser

#### Supported Algorithms

- **Gzip** - `Content-Encoding: gzip`
- **Brotli** - `Content-Encoding: br`
- **Deflate** - `Content-Encoding: deflate`
- **Identity** - Pass-through for uncompressed data

#### Usage Pattern

Wrap any existing parser to add compression support.

### MessagePack Parser (`MessagePackParser`)

#### Basic Features

- **Content-Type**: `application/msgpack`, `application/x-msgpack`
- Parse MessagePack binary format using `rmp_serde`
- Returns `ParsedData::MessagePack(Value)` as JSON-compatible structure

#### Characteristics

- Efficient binary serialization format
- More compact than JSON while maintaining similar data structures
- Supports all standard MessagePack data types
- Automatic deserialization to JSON-like `Value` type

### Protobuf Parser (`ProtobufParser`)

#### Basic Features

- **Content-Type**: `application/protobuf`, `application/x-protobuf`
- Parse Protocol Buffers binary format
- Returns `ParsedData::Protobuf(Value)` with dynamic schema support

#### Schema Support

- **Dynamic parsing** - Wire format parsing without full schema
- **Schema registry** - Optional type resolution via `with_schema_registry()`
- Infers types from Protobuf wire types

#### Wire Format Support

- Varint encoding (int32, int64, uint32, uint64, sint32, sint64, bool, enum)
- 64-bit fixed (fixed64, sfixed64, double)
- Length-delimited (string, bytes, embedded messages, packed repeated fields)
- 32-bit fixed (fixed32, sfixed32, float)

### Streaming Parser (`StreamingParser`)

#### Basic Features

- **Memory-efficient parsing** for large file uploads
- Incremental processing without loading entire body into memory
- Chunk-based data processing

#### Configuration

- **Chunk size** - Configurable via constructor
  - Example: `StreamingParser::new(1024 * 1024)` for 1MB chunks
- **Maximum size limit** - Configurable via `with_max_size()`
  - Prevents memory exhaustion from extremely large uploads

#### StreamChunk Structure

- `data: Bytes` - Chunk content
- `offset: usize` - Position in overall stream
- `total_size: Option<usize>` - Total size if known

#### Use Cases

- Large file uploads (100MB+)
- Streaming data processing
- Progress tracking for uploads
- Memory-constrained environments

### Validation System

#### `ParserValidator` Trait

- **Custom validation hooks** for parsers
- `before_parse()` - Validate before parsing (content-type, body size, etc.)
- `after_parse()` - Validate after parsing (structure, content)

#### Built-in Validators

**`SizeLimitValidator`**
- Enforces maximum body size limits
- Prevents memory exhaustion attacks
- Example: `SizeLimitValidator::new(1024 * 1024)` for 1MB limit

**`ContentTypeValidator`**
- Validates required content types
- Ensures requests match expected media types
- Useful for API endpoint protection

**`CompositeValidator`**
- Combines multiple validators
- Executes validators in sequence
- Short-circuits on first validation failure

### Error Handling

- Unified error types using `reinhardt_exception::Error`
- Type aliases: `ParseError` and `ParseResult<T>`
- Detailed error messages for debugging
- Integration with framework exception system

## Usage Example

```rust
use bytes::Bytes;
use reinhardt::core::parsers::{
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
