# reinhardt-exception

Exception handling and error types

## Overview

Provides comprehensive error handling with Django-inspired exception hierarchy. Includes HTTP exceptions, validation errors, database errors, and custom error types with detailed error messages.

## Features

### Implemented ✓

- **Django-style exception hierarchy** - Comprehensive `Error` enum with categorized error types
- **HTTP status code exceptions** - `Http`, `Authentication` (401), `Authorization` (403), `NotFound` (404), `Internal` (500), etc.
- **Validation error handling** - `Validation` variant with field-level error support
- **Database exception types** - `Database` variant for DB-related errors
- **Custom error types** - `ImproperlyConfigured`, `BodyAlreadyConsumed`, `ParseError`, etc.
- **Error serialization** - All errors implement `Display` and can be converted to HTTP responses via `status_code()` method
- **thiserror integration** - Full integration with `thiserror` for derived error impl
- **anyhow integration** - `Other` variant wraps any `anyhow::Error` for compatibility
- **Error categorization** - `ErrorKind` enum for categorical classification
- **Standard conversions** - `From` implementations for `serde_json::Error`, `std::io::Error`, `http::Error`