# reinhardt-forms

Django-inspired form handling and validation for Rust

## Overview

`reinhardt-forms` provides a comprehensive form system for HTML form handling, validation, and rendering. Inspired by Django's forms framework, it offers both automatic form generation from models and manual form definitions with extensive validation capabilities.

## Features Status

### Core Form System

#### Implemented ✓

- **Form Base (`Form`)**: Complete form data structure with binding, validation, and rendering
  - Form creation with initial data and field prefix support
  - Data binding and validation lifecycle
  - Custom clean functions for form-level and field-level validation
  - Multiple rendering formats: `as_table()`, `as_p()`, `as_ul()`
  - Field access and manipulation (add, remove, get)
  - Initial data and change detection
  - Error handling and reporting

- **BoundField**: Field bound to form data for rendering
  - Field data and error binding
  - HTML rendering with proper escaping
  - Widget integration
  - Label and help text support

- **CSRF Protection (`CsrfToken`)**: Basic CSRF token implementation
  - Token generation and storage
  - Hidden input rendering
  - Form integration via `enable_csrf()`

- **Media Management (`Media`)**: CSS and JavaScript asset management
  - Media definition structure
  - Widget media integration (via `MediaDefiningWidget` trait)

### Field Types

#### Implemented ✓

**Basic Fields:**

- `CharField`: Text input with min/max length, stripping, null character validation
- `IntegerField`: Integer input with min/max value constraints, string parsing
- `BooleanField`: Boolean/checkbox input with flexible type coercion
- `EmailField`: Email validation with regex, length constraints

**Advanced Fields:**

- `FloatField`: Floating-point number validation with min/max constraints
- `DecimalField`: Precise decimal number handling with scale and precision
- `DateField`: Date input with multiple format support and locale handling
- `TimeField`: Time input with format parsing
- `DateTimeField`: Combined date and time validation
- `URLField`: URL validation with scheme and max length checks
- `JSONField`: JSON data validation and parsing
- `FileField`: File upload handling with size validation
- `ImageField`: Image file validation with dimension checks
- `ChoiceField`: Selection from predefined choices
- `MultipleChoiceField`: Multiple selection support
- `RegexField`: Pattern-based validation with custom regex
- `SlugField`: URL slug validation
- `GenericIPAddressField`: IPv4/IPv6 address validation
- `UUIDField`: UUID format validation
- `DurationField`: Time duration parsing
- `ComboField`: Multiple field validation combination
- `MultiValueField`: Composite field handling (base for split fields)
- `SplitDateTimeField`: Separate date and time inputs

**Model-Related Fields:**

- `ModelChoiceField`: Foreign key selection with queryset support
- `ModelMultipleChoiceField`: Many-to-many selection

### Model Integration

#### Implemented ✓

- **ModelForm (`ModelForm<T>`)**: Automatic form generation from models
  - `FormModel` trait for model integration
  - Field type inference from model metadata
  - Field inclusion/exclusion configuration
  - Custom field override support
  - Model instance population from form data
  - Save functionality with validation

- **ModelFormBuilder**: Fluent API for ModelForm configuration
  - Field selection (include/exclude)
  - Widget customization
  - Label customization
  - Help text customization

- **ModelFormConfig**: Configuration structure for ModelForm behavior
  - Field mapping configuration
  - Validation rules
  - Save behavior customization

### Formsets

#### Implemented ✓

- **FormSet**: Managing multiple forms together
  - Form collection management
  - Validation across multiple forms
  - Extra form generation
  - Min/max form count constraints
  - Deletion and ordering support
  - Management form handling
  - Non-form error tracking

- **ModelFormSet**: Formset for model instances
  - Queryset integration
  - Instance creation, update, and deletion
  - Inline formset support
  - Configuration via `ModelFormSetConfig`
  - Builder pattern API via `ModelFormSetBuilder`

### Advanced Features

#### Implemented ✓

- **Form Wizard (`FormWizard`)**: Multi-step form flow
  - Step definition and management (`WizardStep`)
  - Conditional step availability
  - Session data storage across steps
  - Step navigation (next, previous, jump)
  - Final data compilation
  - Progress tracking

- **Widgets System**: HTML rendering for form fields
  - Base `Widget` trait
  - `WidgetType` enumeration
  - Built-in widgets:
    - Text inputs (text, password, email, number)
    - Date/time inputs (date, time, datetime)
    - Textarea
    - Select (single and multiple)
    - Checkbox and radio inputs
    - File input
    - Hidden input
    - Split datetime
  - Custom attribute support
  - Choice rendering for select widgets

### Validation

#### Implemented ✓

- **Field Validation**: Individual field cleaning and validation
  - Required field checking
  - Type conversion and coercion
  - Length constraints (CharField)
  - Value range constraints (IntegerField, FloatField, DecimalField)
  - Format validation (EmailField, URLField, DateField, etc.)
  - Pattern matching (RegexField)
  - Custom validators

- **Form Validation**: Multi-field validation
  - Custom clean methods (`add_clean_function`)
  - Field-specific clean methods (`add_field_clean_function`)
  - Cross-field validation
  - Error aggregation
  - Non-field errors

- **Error Handling**: Comprehensive error reporting
  - `FieldError` types (Required, Invalid, Validation)
  - `FormError` types (Field, Validation)
  - Custom error messages
  - Error message internationalization support

#### Implemented ✓

- **Security Features**:
  - Rate limiting integration (`RateLimiter`)
  - Honeypot fields (`HoneypotField`)
  - Form security middleware (`FormSecurityMiddleware`)

- **File Handling**:
  - Temporary file cleanup via `Drop` implementation
  - File upload handler with size and extension validation
  - Memory-based file uploads
  - Disk-based temporary files with auto-deletion

- **Internationalization** (Partial):
  - Locale-aware date/time formatting (`DateField` with `localize` support)
  - Number format localization (`DecimalField` with `thousands_separator`)
  - Locale configuration per field

- **Form Templating** (Partial):
  - Bootstrap 5 integration (`BootstrapRenderer`)
  - Tailwind CSS integration (`TailwindRenderer`)
  - CSS framework renderers for text inputs, selects, and checkboxes

### Planned Features

- **Advanced CSRF Protection**:
  - Cryptographic token generation
  - Token rotation
  - Same-site cookie support
  - Origin validation

- **File Handling Enhancements**:
  - Chunked upload support
  - Progress tracking
  - File validation rules engine

- **Security Features**:
  - Advanced XSS protection
  - Input sanitization rules

- **Additional Field Types**:
  - `PasswordField` with strength validation
  - `ColorField` for color picker
  - `RangeField` for numeric ranges
  - `ArrayField` for array data
  - `GeometryField` for spatial data

- **Internationalization**:
  - Multi-language error messages
  - RTL language support
  - Complete i18n message catalog

- **Form Templating**:
  - Template-based rendering engine
  - Custom form layouts
  - Accessible form markup generation (ARIA attributes)
  - Additional CSS framework support

- **Advanced Widgets**:
  - Rich text editor
  - Date picker with calendar
  - Autocomplete input
  - Multi-select with search
  - File drag-and-drop

- **Testing Utilities**:
  - Form test helpers
  - Mock data generation
  - Validation test fixtures
  - Integration test support

## Usage Examples

### Basic Form

```rust
use reinhardt_forms::{Form, CharField, IntegerField, FormField};
use std::collections::HashMap;
use serde_json::json;

let mut form = Form::new();
form.add_field(Box::new(CharField::new("name".to_string())));
form.add_field(Box::new(IntegerField::new("age".to_string())));

let mut data = HashMap::new();
data.insert("name".to_string(), json!("John"));
data.insert("age".to_string(), json!(30));

form.bind(data);
assert!(form.is_valid());
```

### ModelForm

```rust
use reinhardt_forms::{ModelForm, ModelFormBuilder};

let form = ModelFormBuilder::<User>::new()
    .include_fields(vec!["name", "email"])
    .build();
```

### Custom Validation

```rust
use reinhardt_forms::{Form, FormError};

let mut form = Form::new();
form.add_clean_function(|data| {
    if data.get("password") != data.get("confirm_password") {
        Err(FormError::Validation("Passwords do not match".to_string()))
    } else {
        Ok(())
    }
});
```

## Architecture

- **Field Layer**: Individual field types with validation logic
- **Form Layer**: Form structure, binding, and validation
- **Widget Layer**: HTML rendering and browser interaction
- **Model Layer**: ORM integration and automatic form generation
- **Formset Layer**: Multiple form management
- **Wizard Layer**: Multi-step form flows

## Design Philosophy

This crate follows Django's forms philosophy:

- Declarative field definitions
- Separation of validation logic
- Automatic HTML rendering
- Model integration
- Extensible and customizable

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
