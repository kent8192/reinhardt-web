# reinhardt-logging

Logging configuration and utilities for the Reinhardt framework

## Overview

Structured logging system with support for multiple log outputs, log levels, and custom formatters.

Includes request logging, error logging, and integration with external logging services.

## Features

### Implemented ✓

#### Core Logging Infrastructure

- **Logger System**: Core logger implementation with handler attachment support
  - `Logger` struct with warning level logging
  - `LoggerHandle`: Wrapper around `Arc<Logger>` for thread-safe logger access
  - LogHandler trait for extensible log processing
  - Thread-safe handler management
- **Log Levels**: Support for Debug, Info, Warn, and Error severity levels
- **Log Records**: Structured log record representation

#### Logging Configuration

- **Global Logging Manager**: Singleton-based global logging initialization and management
  - `init_global_logging()` for one-time setup
  - `get_logger(name)` returns `LoggerHandle` for retrieving named loggers
  - Thread-safe access via `once_cell`
- **Configuration Structures**:
  - `LoggingConfig`: Main configuration container
  - `HandlerConfig`: Handler-specific settings
  - `LoggerConfig`: Logger-specific settings

#### Log Handlers

- **Console Handler**: Output logs to standard output/error
- **File Handler**: Write logs to file system
- **JSON Handler**: Structured JSON log output
- **Memory Handler**: In-memory log storage for testing and debugging
  - Level-based filtering
  - Cloneable for test assertions

#### Formatters

- **Formatter Trait**: Extensible log formatting interface
- **Standard Formatter**: Default human-readable format
- **Server Formatter**: Server-optimized log format
- **Control Character Escaping**: `escape_control_chars()` utility for safe log output

#### Filters

- **Filter Trait**: Generic log filtering interface
- **Callback Filter**: Custom filter implementation support
- **Debug-based Filters**:
  - `RequireDebugTrue`: Only log when debug mode is enabled
  - `RequireDebugFalse`: Only log when debug mode is disabled

#### Parameter Representation Utilities

- **Smart Parameter Truncation**: Prevent log overflow with large data structures
  - `repr_params()`: Truncate arrays, objects, and nested structures
  - `truncate_param()`: Truncate individual string values
  - `ReprParamsConfig`: Configurable truncation behavior
- **Multi-batch Parameter Display**: Show first/last items for large parameter sets
- **Character-level Truncation**: Middle truncation preserving start/end context
- **SQLAlchemy-inspired Design**: Based on proven parameter representation patterns

#### Convenience APIs

- **Global Logging Functions**:
  - `emit_warning()`: Quick warning emission
  - `attach_memory_handler()`: Easy test handler attachment
- **Type Safety**: Strongly-typed configuration and log levels
- **Async Support**: Built on Tokio for async runtime compatibility

#### Security Logging

- **SecurityLogger**: Dedicated logger for security-related events
  - Authentication events (success/failure)
  - Authorization violations
  - CSRF violations
  - Rate limit exceeded events
  - Suspicious file operations
  - Disallowed host access
- **SecurityError**: Enum for categorizing security events
  - `AuthenticationFailed`, `AuthorizationFailed`, `InvalidToken`
  - `RateLimitExceeded`, `SuspiciousActivity`, `CsrfViolation`
  - `InvalidInput`, `AccessDenied`, `DisallowedHost`

**Usage Example**:

```rust
use reinhardt_logging::security::{SecurityLogger, SecurityError};

let logger = SecurityLogger::new();

// Log authentication events
logger.log_auth_event(true, "user@example.com");  // INFO level
logger.log_auth_event(false, "attacker@evil.com"); // WARNING level

// Log security errors
logger.log_security_error(&SecurityError::CsrfViolation);  // ERROR level

// Log CSRF violation with details
logger.log_csrf_violation("http://evil.com");

// Log rate limit exceeded
logger.log_rate_limit_exceeded("192.168.1.1", 100);

// Log suspicious file operations
logger.log_suspicious_file_operation("delete", Path::new("/etc/passwd"));

// Log disallowed host access
logger.log_disallowed_host("malicious.com");
```

**Log Level Mapping**:

| Event | Log Level |
|-------|-----------|
| Authentication success | INFO |
| Authentication failure | WARNING |
| CSRF violation | ERROR |
| Rate limit exceeded | WARNING |
| Authorization failure | WARNING |
| Suspicious activity | ERROR |