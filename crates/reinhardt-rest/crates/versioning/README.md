# reinhardt-versioning

API versioning strategies for Reinhardt framework, inspired by Django REST Framework.

## Status

✅ **Complete Implementation** - All features implemented and tested

## Features

### ✅ Implemented

#### Core Versioning Strategies

1. **URLPathVersioning** - Version detection from URL path
   - Extracts version from path segments (e.g., `/v1/users/`, `/v2.0/api/`)
   - Customizable regex patterns for flexible version extraction
   - Pattern configuration with `with_pattern()` method
   - Default fallback version support
   - Allowed versions validation
   - Examples: `/v1/`, `/v2.0/`, `/api/v3/`

2. **AcceptHeaderVersioning** - Version detection from Accept header
   - Parses Accept header media type parameters (e.g., `Accept: application/json; version=2.0`)
   - Configurable version parameter name
   - Strict version validation
   - Default version fallback
   - Compatible with standard media type negotiation
   - Example: `Accept: application/json; version=1.0`

3. **QueryParameterVersioning** - Version detection from query parameters
   - Extracts version from query string (e.g., `?version=1.0`, `?v=2.0`)
   - Customizable parameter name with `with_version_param()`
   - Multiple parameter support
   - Default version fallback
   - Examples: `?version=1.0`, `?v=2.0`

4. **HostNameVersioning** - Version detection from subdomain
   - Extracts version from hostname subdomain (e.g., `v1.api.example.com`)
   - Customizable regex patterns for hostname parsing
   - Host format configuration with `with_host_format()`
   - Hostname pattern mapping with `with_hostname_pattern()`
   - Port handling support
   - Examples: `v1.api.example.com`, `api-v2.example.com`

5. **NamespaceVersioning** - Version detection from URL namespace
   - Router namespace integration for version extraction
   - Configurable namespace patterns (e.g., `/v{version}/`)
   - Namespace prefix support with `with_namespace_prefix()`
   - Router integration methods:
     - `extract_version_from_router()` - Extract version from router path
     - `get_available_versions_from_router()` - Get all registered versions
   - Pattern-based version extraction
   - Examples: `/v1/users/`, `/api/v2/posts/`

#### Middleware System

6. **VersioningMiddleware** - Automatic version detection and injection
   - Integrates with any `BaseVersioning` strategy
   - Automatic version extraction from requests
   - Stores version in request extensions
   - Error handling for invalid versions
   - Clone support for middleware composition
   - Zero-cost abstraction over versioning strategies

7. **RequestVersionExt** - Type-safe version access from requests
   - `version()` - Get version as `Option<String>`
   - `version_or()` - Get version with fallback default
   - Seamless integration with request extensions
   - Type-safe version retrieval

8. **ApiVersion** - Version data type
   - `as_str()` - Get version as string slice
   - `to_string()` - Get version as owned String
   - `new()` - Create new version instance
   - Clone and Debug support

#### Handler Integration

9. **VersionedHandler** - Trait for version-aware handlers
   - `handle_versioned()` - Handle request with version context
   - `supported_versions()` - Get list of supported versions
   - `supports_version()` - Check version support

10. **VersionedHandlerWrapper** - Handler trait adapter
    - Makes `VersionedHandler` compatible with standard `Handler` trait
    - Automatic version determination
    - Version validation before handling
    - Error handling for unsupported versions

11. **SimpleVersionedHandler** - Simple version-to-response mapper
    - Map versions to static responses
    - `with_version_response()` - Add version-specific response
    - `with_default_response()` - Set fallback response
    - HashMap-based response lookup

12. **ConfigurableVersionedHandler** - Advanced handler configuration
    - Map versions to different handler implementations
    - `with_version_handler()` - Add version-specific handler
    - `with_default_handler()` - Set fallback handler
    - Dynamic handler dispatch based on version

13. **VersionedHandlerBuilder** - Builder pattern for handlers
    - Fluent API for handler construction
    - Version-handler mapping
    - Default handler configuration
    - Automatic wrapper integration

14. **VersionResponseBuilder** - Response builder with version metadata
    - `with_data()` - Add response data
    - `with_field()` - Add individual fields
    - `with_version_info()` - Add version metadata
    - `version()` - Get current version
    - JSON serialization support

15. **versioned_handler!** - Macro for easy handler creation
    - Declarative syntax for version mapping
    - Optional default handler
    - Compile-time version checking

#### Configuration System

16. **VersioningConfig** - Global configuration
    - Centralized versioning settings
    - Strategy configuration
    - Default and allowed versions
    - Strict mode enforcement
    - Version parameter customization
    - Hostname pattern mapping
    - Builder pattern API
    - Serialization/deserialization support

17. **VersioningStrategy** - Strategy enumeration
    - Five strategy variants:
      - `AcceptHeader` - Accept header versioning
      - `URLPath { pattern }` - URL path with custom pattern
      - `QueryParameter { param_name }` - Query parameter with custom name
      - `HostName { patterns }` - Hostname with pattern mapping
      - `Namespace { pattern }` - Namespace with custom pattern
    - Serde support for configuration files
    - JSON/YAML compatible

18. **VersioningManager** - Configuration management
    - Create versioning instances from configuration
    - Dynamic configuration updates
    - `config()` - Get current configuration
    - `versioning()` - Get versioning instance
    - `update_config()` - Update configuration at runtime
    - Environment variable support with `from_env()`

19. **Environment Configuration** - Env var support
    - `REINHARDT_VERSIONING_DEFAULT_VERSION` - Default version
    - `REINHARDT_VERSIONING_ALLOWED_VERSIONS` - Comma-separated allowed versions
    - `REINHARDT_VERSIONING_STRATEGY` - Strategy type
    - `REINHARDT_VERSIONING_STRICT_MODE` - Enable/disable strict mode

#### URL Reverse System

20. **VersionedUrlBuilder** - Versioned URL construction
    - Build URLs with version in appropriate location
    - Strategy-aware URL generation
    - `build()` - Build URL with default version
    - `build_with_version()` - Build URL with specific version
    - `build_all_versions()` - Build URLs for all allowed versions
    - Support for all five versioning strategies

21. **UrlReverseManager** - Multiple builder management
    - Named builder registration
    - Default builder support
    - `add_builder()` - Register named builder
    - `with_default_builder()` - Set default builder
    - `build_url()` - Build URL with named builder
    - `build_default_url()` - Build URL with default builder
    - `build_all_urls()` - Build URLs with all builders

22. **ApiDocUrlBuilder** - API documentation URL builder
    - OpenAPI schema URLs
    - Swagger UI URLs
    - ReDoc URLs
    - Custom format support
    - Version-specific documentation paths
    - Examples:
      - `/v1.0/openapi.json`
      - `/v2.0/swagger-ui/`
      - `/v1.0/redoc/`

23. **ApiDocFormat** - Documentation format enum
    - `OpenApi` - OpenAPI 3.0 JSON
    - `Swagger` - Swagger UI
    - `ReDoc` - ReDoc documentation
    - `Custom(String)` - Custom format

24. **versioned_url!** - Macro for URL building
    - Simple syntax for URL construction
    - Version override support
    - Type-safe URL generation

#### Testing & Quality

25. **Comprehensive Test Coverage**
    - 29+ unit tests across all modules
    - 11+ integration tests
    - All versioning strategies tested
    - Middleware integration tests
    - Handler system tests
    - URL building tests
    - Configuration serialization tests

26. **Test Utilities** - `test_utils` module
    - `create_test_request()` - Create mock requests for testing
    - Header customization
    - URI customization
    - Reusable across test suites

27. **Full Documentation**
    - Comprehensive rustdoc comments
    - Code examples for all public APIs
    - Usage examples in each module
    - Integration examples

#### Error Handling

28. **VersioningError** - Comprehensive error types
    - `InvalidAcceptHeader` - Malformed Accept header
    - `InvalidURLPath` - Invalid URL path format
    - `InvalidNamespace` - Invalid namespace format
    - `InvalidHostname` - Invalid hostname format
    - `InvalidQueryParameter` - Invalid query parameter
    - `VersionNotAllowed` - Version not in allowed list
    - Integration with `reinhardt_apps::Error`

#### Traits & Abstractions

29. **BaseVersioning** - Core versioning trait
    - `determine_version()` - Extract version from request
    - `default_version()` - Get default version
    - `allowed_versions()` - Get allowed versions
    - `is_allowed_version()` - Check version validity
    - `version_param()` - Get version parameter name
    - Async trait for async version detection
    - Send + Sync for thread safety

### Planned

なし - すべての機能が実装済みです

## Quick Start

## Basic Usage

```rustuse reinhardt_versioning::{URLPathVersioning, VersioningMiddleware, RequestVersionExt};

// Create versioning strategylet versioning = URLPathVersioning::new()
    .with_default_version("1.0")
    .with_allowed_versions(vec!["1.0", "2.0"]);

// Use as middlewarelet middleware = VersioningMiddleware::new(versioning);

// Access in handlerasync fn handler(request: Request) -> Result<Response> {
    let version = request.version().unwrap_or_else(|| "1.0".to_string());
    // Version-specific logic here
}
```

## Global Configuration

```rustuse reinhardt_versioning::{VersioningConfig, VersioningManager, VersioningStrategy};

// Configure versioning globallylet config = VersioningConfig {
    default_version: "1.0".to_string(),
    allowed_versions: vec!["1.0".to_string(), "2.0".to_string()],
    strategy: VersioningStrategy::URLPath {
        default_version: Some("1.0".to_string()),
        allowed_versions: Some(vec!["1.0".to_string(), "2.0".to_string()]),
        pattern: Some("/v{version}/".to_string()),
    },
};

let manager = VersioningManager::new(config);
```

## Handler Integration

```rustuse reinhardt_versioning::{VersionedHandlerBuilder, SimpleVersionedHandler};

// Create versioned handlerslet v1_handler = Arc::new(
    SimpleVersionedHandler::new()
        .with_version_response("1.0", r#"{"version": "1.0"}"#)
);

let v2_handler = Arc::new(
    SimpleVersionedHandler::new()
        .with_version_response("2.0", r#"{"version": "2.0"}"#)
);

// Build versioned handlerlet handler = VersionedHandlerBuilder::new(versioning)
    .with_version_handler("1.0", v1_handler)
    .with_version_handler("2.0", v2_handler)
    .build();
```

## URL Reverse Support

```rustuse reinhardt_versioning::{VersionedUrlBuilder, VersioningStrategy};

// Create URL builderlet url_builder = VersionedUrlBuilder::with_strategy(
    versioning,
    "https://api.example.com",
    VersioningStrategy::URLPath,
);

// Generate versioned URLslet v1_url = url_builder.with_version("1.0").build("/users");
// Result: "https://api.example.com/v1.0/users"

let v2_url = url_builder.with_version("2.0").build("/users");
// Result: "https://api.example.com/v2.0/users"
```

## Documentation

See inline documentation for detailed API usage.

## License

MIT OR Apache-2.0