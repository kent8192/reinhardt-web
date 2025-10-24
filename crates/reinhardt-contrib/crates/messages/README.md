# reinhardt-messages

Flash messages and user notifications for Reinhardt framework

## Overview

Django-inspired messaging framework for displaying one-time notification messages to users. Provides a complete message system with multiple storage backends and flexible configuration options.

## Features

### Implemented ✓

#### Core Message System

- **Message Levels**: 5 predefined levels (Debug, Info, Success, Warning, Error) with numeric priority values (10, 20, 25, 30, 40)
- **Custom Levels**: Support for user-defined message levels with custom numeric values
- **Message Tags**: Level-based tags and extra custom tags for styling and filtering
- **Message Creation**: Convenience methods for creating messages (`Message::debug()`, `Message::info()`, etc.)
- **Message Configuration**: `MessageConfig` for customizing level tags globally

#### Storage Backends

- **MemoryStorage**: In-memory storage using thread-safe `Arc<Mutex<VecDeque>>` for testing and temporary messages
- **SessionStorage**: Session-based persistent storage with JSON serialization
  - Customizable session key (default: `"_messages"`)
  - Session availability validation
  - Serialization/deserialization for session integration
- **CookieStorage**: Cookie-based storage with automatic size management
  - Configurable cookie name and size limit (default: 4KB)
  - Automatic message truncation using binary search when exceeding size limits
  - Drops oldest messages first when size limit is exceeded
- **FallbackStorage**: Intelligent fallback between Cookie and Session storage
  - Attempts cookie storage first for better performance
  - Automatically falls back to session storage when cookie size is exceeded
  - Tracks which storage backend(s) were used
  - Supports flushing messages from both backends

#### Utilities

- **Binary Search Algorithms**: Efficient size-limited message management
  - `bisect_keep_left()`: Keep maximum messages from the beginning within size limit
  - `bisect_keep_right()`: Keep maximum messages from the end within size limit
- **SafeData**: HTML-safe string wrapper for rendering pre-sanitized HTML content
  - Prevents double-escaping of HTML in messages
  - Serializable with serde support

#### Storage Trait

- **MessageStorage Trait**: Unified interface for all storage backends
  - `add()`: Add a message to storage
  - `get_all()`: Retrieve and clear all messages
  - `peek()`: View messages without clearing
  - `clear()`: Remove all messages

### Planned

#### Middleware Integration

- Request/response middleware for automatic message handling
- Automatic message retrieval and storage during request lifecycle
- Context processor for template integration

#### Advanced Features

- Message filtering by level
- Message persistence control (sticky messages)
- Message expiry and TTL support
- Async storage backend support
- Custom serialization formats (MessagePack, CBOR)
- Message encryption for sensitive data
- Rate limiting for message creation

#### Template Integration

- Template tags for message rendering
- Default message templates with Bootstrap/Tailwind CSS styling
- JavaScript integration for client-side message display
- Toast notification support
- Message dismissal tracking

#### Testing Utilities

- Mock storage backends for testing
- Message assertion helpers
- Test fixtures for common scenarios

## Usage

### Basic Message Creation

```rust
use reinhardt_messages::{Message, Level};

// Using level constructor
let msg = Message::new(Level::Info, "Operation completed");

// Using convenience methods
let debug_msg = Message::debug("Debug information");
let info_msg = Message::info("User logged in");
let success_msg = Message::success("Profile updated successfully");
let warning_msg = Message::warning("Disk space is low");
let error_msg = Message::error("Failed to connect to database");

// With custom tags
let tagged_msg = Message::info("Important notification")
    .with_tags(vec!["urgent".to_string(), "user-action".to_string()]);
```

### Storage Backends

```rust
use reinhardt_messages::storage::{
    MessageStorage, MemoryStorage, SessionStorage,
    CookieStorage, FallbackStorage
};

// Memory storage (for testing)
let mut memory = MemoryStorage::new();
memory.add(Message::info("Test message"));
let messages = memory.get_all();

// Session storage
let mut session = SessionStorage::new()
    .with_session_key("custom_messages");
session.add(Message::success("Saved to session"));

// Cookie storage with size limit
let mut cookie = CookieStorage::new()
    .with_cookie_name("flash_messages")
    .with_max_size(2048);
cookie.add(Message::warning("Stored in cookie"));

// Fallback storage (Cookie → Session)
let mut fallback = FallbackStorage::new()
    .with_max_cookie_size(4096);
fallback.add(Message::info("Automatically handled"));
fallback.store().unwrap(); // Triggers fallback if needed
```

### Custom Message Levels

```rust
use reinhardt_messages::{Level, MessageConfig};

// Create custom level
let custom_level = Level::Custom(35);
let msg = Message::new(custom_level, "Custom priority message");

// Configure custom level tags
let mut config = MessageConfig::new();
config.set_tag(35, "urgent".to_string());
assert_eq!(config.get_tag(Level::Custom(35)), Some("urgent"));
```

### SafeData for HTML Content

```rust
use reinhardt_messages::SafeData;

// Mark HTML content as safe
let safe_html = SafeData::new("<b>Bold text</b>");
println!("{}", safe_html); // Renders: <b>Bold text</b>

// Convert back to String
let html_string = safe_html.into_string();
```

## Architecture

### Message Levels

- Numeric priority system allows custom levels between standard ones
- Level ordering: Debug (10) < Info (20) < Success (25) < Warning (30) < Error (40)
- Custom levels can have any i32 value for fine-grained control

### Storage Strategy

- All storage backends implement `MessageStorage` trait for consistency
- Cookie storage uses binary search to efficiently fit maximum messages within size limits
- Fallback storage intelligently routes messages based on size constraints
- Session storage validates middleware availability before operations

### Size Management

- Binary search algorithms (`bisect_keep_left`/`bisect_keep_right`) optimize message truncation
- Efficient serialization size calculation without full re-serialization
- Automatic oldest-first removal when size limits are exceeded

## Testing

Comprehensive test coverage based on Django's message framework tests:

- Message creation and manipulation
- Level comparison and ordering
- All storage backend operations
- Size limit handling and truncation
- Serialization/deserialization
- Binary search algorithms

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT license

at your option.
