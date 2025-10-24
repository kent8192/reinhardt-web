# reinhardt-validators

Django-style data validation utilities for Rust.

## Overview

A comprehensive collection of reusable validators following Django's validator patterns. Provides type-safe validation for common use cases including email addresses, URLs, numeric ranges, string lengths, and custom regex patterns.

## Features

### Implemented ✓

#### Core Validation Framework

- **Validator Trait**: Generic validation interface `Validator<T>` for implementing custom validators
- **OrmValidator Trait**: Extension trait for ORM validators with custom error messages
- **SettingsValidator Trait**: Extension trait for validating configuration settings
- **ValidationError**: Comprehensive error types with descriptive messages
- **ValidationResult<T>**: Type-safe result type for validation operations
- **Prelude Module**: Convenient re-exports of all validators and error types

#### String Validators

- **MinLengthValidator**: Validates minimum string length
  - Works with both `String` and `&str` types
  - Provides detailed error messages with actual and expected lengths
  - Unicode-aware length checking
- **MaxLengthValidator**: Validates maximum string length
  - Works with both `String` and `&str` types
  - Provides detailed error messages with actual and expected lengths
  - Unicode-aware length checking
- **RegexValidator**: Pattern matching with regular expressions
  - Custom error message support via `with_message()`
  - Full regex syntax support
  - Works with both `String` and `&str` types

#### Numeric Validators

- **MinValueValidator**: Validates minimum numeric values
  - Generic over any `PartialOrd + Display + Clone` type
  - Supports integers (i8, i16, i32, i64, isize, u8, u16, u32, u64, usize)
  - Supports floating-point numbers (f32, f64)
  - Provides detailed error messages with actual and expected values
- **MaxValueValidator**: Validates maximum numeric values
  - Generic over any `PartialOrd + Display + Clone` type
  - Supports all integer and floating-point types
  - Provides detailed error messages with actual and expected values
- **RangeValidator**: Validates values within a range (inclusive)
  - Generic over any `PartialOrd + Display + Clone` type
  - Supports all numeric types
  - Reports whether value is too small or too large

#### Email Validator

- **EmailValidator**: RFC 5322 compliant email validation
  - Case-insensitive validation
  - Local part validation (max 64 characters)
    - Allows alphanumeric characters, dots, underscores, percent signs, plus and minus signs
    - Prevents consecutive dots
    - Prevents leading/trailing dots
  - Domain part validation (max 255 characters)
    - Supports subdomains
    - Each label max 63 characters
    - TLD minimum 2 characters
    - Prevents leading/trailing hyphens in domain labels
  - Total length limit (max 320 characters)
  - Works with both `String` and `&str` types

#### URL Validator

- **UrlValidator**: HTTP/HTTPS URL validation
  - Scheme validation (http, https)
  - Port number support (1-5 digits)
  - Path validation
  - Query string support
  - Fragment identifier support
  - Subdomain support
  - Hyphen support in domain names (not at start/end of labels)
  - Works with both `String` and `&str` types

#### Error Types

- `InvalidEmail(String)`: Invalid email address format
- `InvalidUrl(String)`: Invalid URL format
- `TooSmall { value: String, min: String }`: Value below minimum
- `TooLarge { value: String, max: String }`: Value above maximum
- `TooShort { length: usize, min: usize }`: String shorter than minimum
- `TooLong { length: usize, max: usize }`: String longer than maximum
- `PatternMismatch(String)`: Regex pattern did not match
- `Custom(String)`: Custom validation error

### Planned

#### Additional Validators

- **SlugValidator**: Validate URL-safe slugs
- **UUIDValidator**: Validate UUID formats (v1-v5)
- **IPAddressValidator**: Validate IPv4/IPv6 addresses
- **DateValidator**: Validate date formats
- **TimeValidator**: Validate time formats
- **DateTimeValidator**: Validate datetime formats
- **JSONValidator**: Validate JSON structure and schema
- **FileExtensionValidator**: Validate file extensions
- **FileSizeValidator**: Validate file sizes
- **ImageDimensionValidator**: Validate image width/height
- **ColorValidator**: Validate color codes (hex, rgb, rgba, etc.)
- **PhoneNumberValidator**: Validate phone numbers (E.164 format)
- **CreditCardValidator**: Validate credit card numbers (Luhn algorithm)
- **PostalCodeValidator**: Validate postal codes (country-specific)

#### Enhanced Features

- **Validator Composition**: Combine multiple validators with AND/OR logic
- **Conditional Validation**: Apply validators based on conditions
- **Async Validators**: Support for async validation operations
- **Custom Error Messages**: Per-validator custom error message templates
- **Internationalization (i18n)**: Multi-language error messages
- **Serialization Support**: Serialize/deserialize validators for storage
- **Schema Validation**: JSON Schema and other schema format support

#### Performance Optimizations

- **Lazy Regex Compilation**: Compile regex patterns only when needed
- **Validator Caching**: Cache compiled validators for reuse
- **Parallel Validation**: Run independent validators concurrently

## Usage Examples

### Basic String Validation

```rust
use reinhardt_validators::{MinLengthValidator, MaxLengthValidator, Validator};

let min_validator = MinLengthValidator::new(5);
let max_validator = MaxLengthValidator::new(10);

assert!(min_validator.validate("hello").is_ok());
assert!(min_validator.validate("hi").is_err());

assert!(max_validator.validate("hello").is_ok());
assert!(max_validator.validate("hello world").is_err());
```

### Numeric Range Validation

```rust
use reinhardt_validators::{RangeValidator, Validator};

let validator = RangeValidator::new(10, 20);
assert!(validator.validate(&15).is_ok());
assert!(validator.validate(&5).is_err());
assert!(validator.validate(&25).is_err());
```

### Email Validation

```rust
use reinhardt_validators::{EmailValidator, Validator};

let validator = EmailValidator::new();
assert!(validator.validate("user@example.com").is_ok());
assert!(validator.validate("invalid@").is_err());
```

### URL Validation

```rust
use reinhardt_validators::{UrlValidator, Validator};

let validator = UrlValidator::new();
assert!(validator.validate("http://example.com").is_ok());
assert!(validator.validate("https://example.com:8080/path?query=value#section").is_ok());
assert!(validator.validate("not-a-url").is_err());
```

### Regex Pattern Validation

```rust
use reinhardt_validators::{RegexValidator, Validator};

let validator = RegexValidator::new(r"^\d{3}-\d{4}$")
    .unwrap()
    .with_message("Phone number must be in format XXX-XXXX");

assert!(validator.validate("123-4567").is_ok());
assert!(validator.validate("invalid").is_err());
```

### Combining Multiple Validators

```rust
use reinhardt_validators::{MinLengthValidator, MaxLengthValidator, Validator};

fn validate_username(username: &str) -> Result<(), String> {
    let min_validator = MinLengthValidator::new(3);
    let max_validator = MaxLengthValidator::new(20);

    min_validator.validate(username).map_err(|e| e.to_string())?;
    max_validator.validate(username).map_err(|e| e.to_string())?;

    Ok(())
}

assert!(validate_username("john").is_ok());
assert!(validate_username("jo").is_err());
assert!(validate_username("verylongusernamethatexceedslimit").is_err());
```

### Using the Prelude

```rust
use reinhardt_validators::prelude::*;

let email = EmailValidator::new();
let url = UrlValidator::new();
let range = RangeValidator::new(0, 100);

assert!(email.validate("test@example.com").is_ok());
assert!(url.validate("http://example.com").is_ok());
assert!(range.validate(&50).is_ok());
```

## Testing

All validators include comprehensive test suites based on Django's validator tests. Run tests with:

```bash
cargo test
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE))
- MIT license ([LICENSE-MIT](../../LICENSE-MIT))

at your option.
