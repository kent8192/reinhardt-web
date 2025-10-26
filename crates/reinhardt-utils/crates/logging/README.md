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
  - Handler trait for extensible log processing
  - Thread-safe handler management
- **Log Levels**: Support for Debug, Info, Warn, and Error severity levels
- **Log Records**: Structured log record representation

#### Logging Configuration

- **Global Logging Manager**: Singleton-based global logging initialization and management
  - `init_global_logging()` for one-time setup
  - `get_logger()` for retrieving named loggers
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