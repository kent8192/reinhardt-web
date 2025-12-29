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
- **Standard conversions** - `From` implementations for `serde_json::Error`, `std::io::Error`, `http::Error`, `String`, `&str`, `validator::ValidationErrors`
- **Parameter validation context** - `ParamErrorContext` struct with detailed parameter extraction error information
- **Parameter type enumeration** - `ParamType` enum (`Json`, `Query`, `Path`, `Form`, `Header`, `Cookie`, `Body`)
- **Additional error types** - `TemplateNotFound` (404), `MissingContentType` (400), `MethodNotAllowed` (405), `Conflict` (409)
- **Pagination error types** - `InvalidPage`, `InvalidCursor`, `InvalidLimit` variants for pagination validation
- **URL parameter errors** - `MissingParameter` variant for URL reverse operations
- **Helper utilities** - `extract_field_from_serde_error` and `extract_field_from_urlencoded_error` functions
- **Error kind classification** - `kind()` method returns `ErrorKind` for categorical error analysis

## Installation

Add `reinhardt` to your `Cargo.toml`:

```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["core"] }

# Or use a preset:
# reinhardt = { version = "0.1.0-alpha.1", features = ["standard"] }  # Recommended
# reinhardt = { version = "0.1.0-alpha.1", features = ["full"] }      # All features
```

**Note:** The `core` feature (included in `standard` and `full`) is required to use the exception types from this crate.

## Usage

### Basic Error Creation

```rust
use reinhardt::core::exception::Error;

// Create HTTP errors
let http_err = Error::Http("Invalid request format".to_string());
assert_eq!(http_err.status_code(), 400);

// Create authentication errors
let auth_err = Error::Authentication("Invalid token".to_string());
assert_eq!(auth_err.status_code(), 401);

// Create database errors
let db_err = Error::Database("Connection timeout".to_string());
assert_eq!(db_err.status_code(), 500);
```

### Using the Result Type Alias

```rust
use reinhardt::core::exception::{Error, Result};

fn validate_email(email: &str) -> Result<()> {
	if email.contains('@') {
		Ok(())
	} else {
		Err(Error::Validation("Email must contain @".to_string()))
	}
}

// Successful validation
assert!(validate_email("user@example.com").is_ok());

// Failed validation
let result = validate_email("invalid-email");
assert!(result.is_err());
```

### Parameter Validation Errors

```rust
use reinhardt::core::exception::{Error, ParamErrorContext, ParamType};

let ctx = ParamErrorContext::new(ParamType::Json, "missing field 'email'")
	.with_field("email")
	.with_expected_type::<String>();
let error = Error::ParamValidation(Box::new(ctx));
assert_eq!(error.status_code(), 400);
```

### Error Conversions

```rust
use reinhardt::core::exception::Error;

// Automatic conversion from serde_json::Error
let json_error = serde_json::from_str::<i32>("invalid").unwrap_err();
let error: Error = json_error.into();
assert_eq!(error.status_code(), 400);

// Conversion from String
let error: Error = "Something went wrong".into();
assert_eq!(error.status_code(), 500);
```