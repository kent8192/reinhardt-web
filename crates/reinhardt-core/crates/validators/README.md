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

#### Additional Validators (Implemented ✓)

- **SlugValidator**: Validate URL-safe slugs
- **UUIDValidator**: Validate UUID formats (v1-v5)
- **IPAddressValidator**: Validate IPv4/IPv6 addresses
- **DateValidator**: Validate date formats
- **TimeValidator**: Validate time formats
- **DateTimeValidator**: Validate datetime formats
- **JSONValidator**: Validate JSON structure
- **ColorValidator**: Validate color codes (hex, rgb, rgba, etc.)
- **PhoneNumberValidator**: Validate phone numbers (E.164 format)
- **CreditCardValidator**: Validate credit card numbers (Luhn algorithm)
- **IBANValidator**: Validate international bank account numbers
- **CustomRegexValidator**: User-defined regex pattern validation

#### File Validators (Implemented ✓)

- **FileTypeValidator**: Comprehensive file type validation
  - **Extension validation**: `FileTypeValidator::with_extensions()`
    - Case-insensitive extension matching
    - Multiple extensions support
    - Whitelist-based filtering
  - **MIME type validation**: `FileTypeValidator::with_mime_types()`
    - Validates file MIME types
    - Multiple MIME types support
  - **Preset validators**:
    - `FileTypeValidator::images_only()`: Supports JPEG, PNG, GIF, WebP, SVG, BMP, TIFF, ICO, AVIF
    - `FileTypeValidator::documents_only()`: Supports PDF, DOC, DOCX, XLS, XLSX, PPT, PPTX, TXT

**Example**:
```rust
use reinhardt_validators::{FileTypeValidator, Validator};

// Extension validation
let validator = FileTypeValidator::with_extensions(vec![
    "jpg".to_string(),
    "png".to_string(),
]);
assert!(validator.validate_filename("photo.jpg").is_ok());
assert!(validator.validate_filename("photo.JPG").is_ok()); // Case-insensitive
assert!(validator.validate_filename("document.pdf").is_err());

// Preset validator
let image_validator = FileTypeValidator::images_only();
assert!(image_validator.validate_filename("photo.png").is_ok());
```

#### Async Validators (Implemented ✓)

- **ExistsValidator**: Asynchronous foreign key existence validation
  - Custom async check function support
  - Database table reference validation
  - Validates that referenced records exist in the database

- **UniqueValidator**: Asynchronous uniqueness constraint validation
  - Prevent duplicate entries
  - Instance exclusion during updates with `exclude_id` parameter
  - Custom async uniqueness check function

**Example**:
```rust
use reinhardt_validators::{ExistsValidator, UniqueValidator, Validator};

// Foreign key existence check
let exists_validator = ExistsValidator::new(
    "user_id",
    "users",
    Box::new(|value| Box::pin(async move {
        // Database check logic here
        // Return true if record exists, false otherwise
        true
    }))
);

// Async validation
let result = exists_validator.validate_async("123").await;
assert!(result.is_ok());

// Uniqueness check with instance exclusion
let unique_validator = UniqueValidator::new(
    "email",
    Box::new(|value, exclude_id| Box::pin(async move {
        // Check if email is unique, excluding the given ID
        true
    }))
);

// Validate new record (no exclusion)
let result = unique_validator.validate_async("new@example.com").await;
assert!(result.is_ok());
```

#### Database Identifier Validators (Implemented ✓)

- **TableName**: Compile-time and runtime validated table names
  - SQL reserved word checking via `is_sql_reserved_word()`
  - Snake_case format validation
  - Length validation (max 63 characters for PostgreSQL compatibility)
  - Compile-time validation with `new_const()` constant function
  - Runtime validation with `new()` method

- **FieldName**: SQL-safe field/column name validation
  - Same validation rules as TableName
  - Prevents SQL injection through identifier validation

- **ConstraintName**: SQL-safe constraint name validation
  - Validates constraint identifiers for CREATE/ALTER statements

**Example**:
```rust
use reinhardt_validators::{TableName, FieldName};

// Runtime validation
let table = TableName::new("users")?;
assert!(TableName::new("select").is_err()); // SQL reserved word
assert!(TableName::new("User-Table").is_err()); // Not snake_case

// Compile-time validation
const VALID_TABLE: TableName = TableName::new_const("users");
const VALID_FIELD: FieldName = FieldName::new_const("email_address");
```

#### Custom Error Messages (Partial Implementation ✓)

Currently supported by:
- **RegexValidator**: `.with_message("Custom message")`
- **CustomRegexValidator**: Built-in custom message support

**Planned Extension**: Extend to all validators (see lib.rs for planned features)

**Example**:
```rust
use reinhardt_validators::{RegexValidator, Validator};

let validator = RegexValidator::new(r"^\d{3}-\d{4}$")
    .unwrap()
    .with_message("Phone number must be in format XXX-XXXX");

match validator.validate("invalid") {
    Err(e) => assert_eq!(e.to_string(), "Phone number must be in format XXX-XXXX"),
    Ok(_) => panic!("Expected validation error"),
}
```

#### File Size Validator (Implemented ✓)

- **FileSizeValidator**: Validate file sizes with minimum, maximum, or range constraints
  - **Min size validation**: `FileSizeValidator::min(bytes)`
  - **Max size validation**: `FileSizeValidator::max(bytes)`
  - **Range validation**: `FileSizeValidator::range(min_bytes, max_bytes)`
  - **Helper methods for unit conversion**:
    - `FileSizeValidator::from_kb(kb)`: Convert KB to bytes
    - `FileSizeValidator::from_mb(mb)`: Convert MB to bytes
    - `FileSizeValidator::from_gb(gb)`: Convert GB to bytes
  - Integrates well with `FileTypeValidator` for comprehensive file validation
  - Generic over `u64` type for file size values

**Example**:
```rust
use reinhardt_validators::{FileSizeValidator, Validator};

// Validate minimum file size
let min_validator = FileSizeValidator::min(FileSizeValidator::from_kb(100)); // 100 KB minimum
assert!(min_validator.validate(&(150 * 1024)).is_ok()); // 150 KB passes
assert!(min_validator.validate(&(50 * 1024)).is_err()); // 50 KB fails

// Validate maximum file size
let max_validator = FileSizeValidator::max(FileSizeValidator::from_mb(5)); // 5 MB maximum
assert!(max_validator.validate(&(3 * 1024 * 1024)).is_ok()); // 3 MB passes
assert!(max_validator.validate(&(10 * 1024 * 1024)).is_err()); // 10 MB fails

// Validate file size range
let range_validator = FileSizeValidator::range(
    FileSizeValidator::from_kb(100),  // 100 KB minimum
    FileSizeValidator::from_mb(10),   // 10 MB maximum
);
assert!(range_validator.validate(&(5 * 1024 * 1024)).is_ok()); // 5 MB passes
```

#### Validator Composition (Implemented ✓)

- **AndValidator**: Combine multiple validators with AND logic
  - All contained validators must pass for validation to succeed
  - Short-circuits on first failure for better performance
  - Supports nested composition (AND within OR, etc.)
  - Generic over any type `T` that validators can validate

- **OrValidator**: Combine multiple validators with OR logic
  - At least one contained validator must pass
  - Optional error collection from all validators when all fail
  - Supports nested composition (OR within AND, etc.)
  - Generic over any type `T` that validators can validate

**Example**:
```rust
use reinhardt_validators::{AndValidator, OrValidator, MinLengthValidator, MaxLengthValidator, EmailValidator, UrlValidator, Validator};

// AND composition - Username must be 3-20 characters
let username_validator = AndValidator::new(vec![
    Box::new(MinLengthValidator::new(3)),
    Box::new(MaxLengthValidator::new(20)),
]);
assert!(username_validator.validate("john").is_ok());
assert!(username_validator.validate("jo").is_err()); // Too short
assert!(username_validator.validate("verylongusernamethatexceedslimit").is_err()); // Too long

// OR composition - Contact must be either email OR URL
let contact_validator = OrValidator::new(vec![
    Box::new(EmailValidator::new()),
    Box::new(UrlValidator::new()),
]);
assert!(contact_validator.validate("user@example.com").is_ok()); // Valid email
assert!(contact_validator.validate("http://example.com").is_ok()); // Valid URL
assert!(contact_validator.validate("invalid").is_err()); // Neither email nor URL

// Nested composition - Complex validation logic
let complex_validator = OrValidator::new(vec![
    Box::new(AndValidator::new(vec![
        Box::new(MinLengthValidator::new(3)),
        Box::new(MaxLengthValidator::new(10)),
    ])),
    Box::new(MinLengthValidator::new(20)), // Or very long string
]);
assert!(complex_validator.validate("hello").is_ok()); // Passes first (3-10 chars)
assert!(complex_validator.validate("verylongusernameexceeds20chars").is_ok()); // Passes second (20+ chars)
assert!(complex_validator.validate("hi").is_err()); // Fails both

// Error collection with OrValidator
let collecting_validator = OrValidator::new(vec![
    Box::new(MinLengthValidator::new(10)),
    Box::new(MinLengthValidator::new(20)),
])
.with_error_collection(true);

match collecting_validator.validate("short") {
    Err(e) => println!("All validators failed: {}", e), // Shows all error messages
    Ok(_) => unreachable!(),
}
```

#### Postal Code Validator (Implemented ✓)

- **PostalCodeValidator**: Country-specific postal code format validation
  - **Supported countries**: US, UK, JP, CA, DE (5 countries)
  - **Country restriction**: `with_countries(vec![Country::US, Country::JP])`
  - **Single country**: `for_country(Country::US)`
  - **Country detection**: `validate_with_country()` returns detected country
  - **Case-insensitive validation**: Automatically handles uppercase/lowercase
  - **Whitespace trimming**: Handles leading/trailing spaces
  - **Priority-based pattern matching**: Resolves ambiguous formats correctly

**Supported Formats**:
- **US**: ZIP (12345) and ZIP+4 (12345-6789) formats
- **UK**: Complex alphanumeric format (SW1A 1AA, M1 1AE, etc.)
- **JP**: 7-digit with hyphen (123-4567)
- **CA**: Alphanumeric format (K1A 0B1, M5W 1E6)
- **DE**: 5-digit format (10115, 80331)

**Example**:
```rust
use reinhardt_validators::{PostalCodeValidator, Country, Validator};

// Validate with country restriction
let validator = PostalCodeValidator::with_countries(vec![
    Country::US,
    Country::JP,
]);
assert!(validator.validate("12345").is_ok()); // US ZIP
assert!(validator.validate("12345-6789").is_ok()); // US ZIP+4
assert!(validator.validate("123-4567").is_ok()); // Japan
assert!(validator.validate("SW1A 1AA").is_err()); // UK not allowed

// Single country validation
let us_validator = PostalCodeValidator::for_country(Country::US);
assert!(us_validator.validate("90210").is_ok());
assert!(us_validator.validate("123-4567").is_err()); // Not US format

// Country detection
let detector = PostalCodeValidator::new(); // Accepts all countries
let country = detector.validate_with_country("12345-6789").unwrap();
assert_eq!(country, Country::US);

let country = detector.validate_with_country("SW1A 1AA").unwrap();
assert_eq!(country, Country::UK);

// Case-insensitive and whitespace handling
assert!(detector.validate("  sw1a 1aa  ").is_ok()); // UK lowercase with spaces
assert!(detector.validate("k1a 0b1").is_ok()); // Canada lowercase
```

#### Image Dimension Validator (Implemented ✓)

- **ImageDimensionValidator**: Validate image width/height dimensions
  - **Min/max width constraints**: `min_width()`, `max_width()`
  - **Min/max height constraints**: `min_height()`, `max_height()`
  - **Aspect ratio validation**: `aspect_ratio()` with configurable tolerance
  - **File validation**: `validate_file()` for file paths
  - **Bytes validation**: `validate_bytes()` for in-memory images
  - **Supported formats**: JPEG, PNG, GIF, WebP, BMP, TIFF, ICO, and more via `image` crate

**Example**:
```rust
use reinhardt_validators::ImageDimensionValidator;

// Basic dimension constraints
let validator = ImageDimensionValidator::new()
    .min_width(100)
    .max_width(1920)
    .min_height(100)
    .max_height(1080);

// With aspect ratio validation (16:9 with 1% tolerance)
let hd_validator = ImageDimensionValidator::new()
    .min_width(1280)
    .min_height(720)
    .aspect_ratio(16, 9)
    .aspect_ratio_tolerance(0.01);

// Validate from file path
let result = validator.validate_file("image.jpg");

// Validate from bytes
let image_bytes: Vec<u8> = std::fs::read("image.png")?;
let result = validator.validate_bytes(&image_bytes);
```

#### Conditional Validation (Implemented ✓)

- **ConditionalValidator**: Apply validators based on runtime conditions
  - **`when` condition**: Apply validator only when condition is true
  - **`unless` condition**: Apply validator only when condition is false
  - **Closure-based conditions**: Use custom logic for condition evaluation
  - **Chainable API**: Combine with other validators

**Example**:
```rust
use reinhardt_validators::{ConditionalValidator, MinLengthValidator, Validator};

// Apply validation only when condition is true
let validator = ConditionalValidator::when(
    MinLengthValidator::new(10),
    || some_condition(), // Closure returns bool
);

// Apply validation unless condition is true
let validator = ConditionalValidator::unless(
    MinLengthValidator::new(5),
    || skip_condition(),
);
```

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
