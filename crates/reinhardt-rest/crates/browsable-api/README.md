# reinhardt-browsable-api

HTML browsable API interface

## Overview

Web-based interface for exploring and testing API endpoints. Provides human-friendly HTML rendering of API responses, interactive forms for testing endpoints, and authentication handling.

## Installation

Add `reinhardt` to your `Cargo.toml`:

```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["rest-browsable-api"] }

# Or use a preset:
# reinhardt = { version = "0.1.0-alpha.1", features = ["standard"] }  # Recommended
# reinhardt = { version = "0.1.0-alpha.1", features = ["full"] }      # All features
```

Then import browsable API features:

```rust
use reinhardt::rest::browsable_api::{BrowsableApiRenderer, ApiContext};
use reinhardt::rest::browsable_api::{BrowsableResponse, FormContext, FormField};
```

**Note:** Browsable API features are included in the `standard` and `full` feature presets.

## Features

### Implemented ✓

#### Core Rendering

- **BrowsableApiRenderer**: Handlebars-based HTML template renderer
  - Default DRF-inspired template with gradient header design
  - Customizable template registration support
  - JSON response pretty-printing and syntax highlighting
  - Responsive design with modern CSS styling
- **ApiContext**: Complete context structure for API rendering
  - Title, description, endpoint, and HTTP method display
  - Response data with status code
  - Allowed HTTP methods visualization
  - Request headers display in table format
  - Optional form context integration

#### Response Handling

- **BrowsableResponse**: Structured API response type
  - Data payload with serde_json::Value support
  - ResponseMetadata with status, method, path, and headers
  - Convenience constructors (new, success)
  - Full serialization/deserialization support

#### Form Generation

- **FormContext**: Interactive request form rendering
  - Dynamic form field generation
  - Support for multiple input types (text, textarea, etc.)
  - Required field indicators
  - Help text for field guidance
  - Initial value support for form fields
- **FormField**: Individual form field configuration
  - Field name, label, and type specification
  - Required/optional field handling
  - Help text and initial value support

#### Template System

- **ApiTemplate**: Basic HTML template utilities
  - Simple API response rendering
  - Error page generation with status codes
  - Fallback templates for simple use cases

#### Visual Features

- HTTP method badges with color coding (GET, POST, PUT, PATCH, DELETE)
- Monospace endpoint display
- Dark theme code blocks for JSON responses
- Responsive container layout with shadow effects
- Form styling with proper input controls
- Header table display with clean formatting