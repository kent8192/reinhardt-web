# reinhardt-http

HTTP request and response handling

## Overview

Core HTTP abstractions for the Reinhardt framework. Provides request and response types, header handling, cookie management, and content negotiation with support for JSON, Form, and multipart data.

## Features

### Implemented ✓

- **Request type** - Complete HTTP request representation
  - Method, URI, version, headers, body
  - Path parameters and query string parsing
  - HTTPS detection (`is_secure`)
  - Type-safe extensions (`Extensions`)
- **Response type** - Flexible HTTP response creation
  - Status code helpers (`ok()`, `created()`, `not_found()`, etc.)
  - Builder pattern (`with_body()`, `with_header()`, `with_json()`)
  - Redirect responses (301, 302, 307)
  - JSON serialization support
- **StreamingResponse** - Streaming response support
  - Media type configuration
  - Custom header support
- **Extensions** - Type-safe request extensions
  - Store/retrieve arbitrary typed data
  - Thread-safe with Arc/Mutex
- **Error integration** - Re-exports `reinhardt_exception::Error` and `Result`

### Planned

Currently all planned features are implemented.