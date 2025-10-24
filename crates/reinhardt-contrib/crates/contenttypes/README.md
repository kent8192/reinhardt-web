# reinhardt-contenttypes

Generic relations and content type framework

## Overview

Framework for working with generic relationships between models. Allows models to reference any other model type dynamically.

Useful for building flexible systems like comments, tags, or activity streams that can relate to multiple model types.

## Features

### Implemented ✓

#### Core Content Type System

- **ContentType Model** - Represents a model type with app label and model name
  - `ContentType::new()` - Create a new content type
  - `ContentType::with_id()` - Set content type ID
  - `natural_key()` - Get (app_label, model) tuple for natural key
  - `qualified_name()` - Get fully qualified name (e.g., "blog.Post")
  - Implements `Serialize`, `Deserialize`, `PartialEq`, `Eq`, `Hash`, `Clone`

#### Content Type Registry (Runtime)

- **ContentTypeRegistry** - Runtime content type management with thread-safe caching
  - `register()` - Register a new content type with automatic ID assignment
  - `get()` - Get content type by app label and model name
  - `get_by_id()` - Get content type by ID
  - `get_or_create()` - Get existing or create new content type
  - `all()` - List all registered content types
  - `clear()` - Clear registry (mainly for testing)
  - Thread-safe with `RwLock` for concurrent access
  - Automatic ID generation for registered types

#### Global Content Type Registry

- **CONTENT_TYPE_REGISTRY** - Global singleton registry instance
  - Available via `once_cell::Lazy` for initialization
  - Shared across the application for consistent content type management

#### Generic Foreign Keys

- **GenericForeignKey** - Field for referencing any model type
  - `new()` - Create empty generic foreign key
  - `set()` - Set content type and object ID
  - `get_content_type()` - Retrieve associated content type
  - `is_set()` - Check if both content type and object ID are set
  - `clear()` - Clear content type and object ID
  - Implements `Default`, `Serialize`, `Deserialize`, `Clone`

#### Type-Safe API (Compile-Time)

- **ModelType Trait** - Compile-time type-safe content type definitions
  - `APP_LABEL` - Associated constant for app label
  - `MODEL_NAME` - Associated constant for model name
  - Type-safe methods for `ContentTypeRegistry`:
    - `get_typed<M: ModelType>()` - Type-safe get
    - `get_or_create_typed<M: ModelType>()` - Type-safe get or create
    - `register_typed<M: ModelType>()` - Type-safe register
  - Type-safe methods for `GenericForeignKey`:
    - `set_typed<M: ModelType>()` - Type-safe set with model type

#### Generic Relation Queries

- **GenericRelatable Trait** - Trait for models that can be targets of generic relations
  - `get_content_type()` - Get content type for the model
  - `get_object_id()` - Get object ID for the instance

- **GenericRelationQuery** - Helper for building generic relation queries
  - `new()` - Create query for specific content type
  - `add_object()` - Add object ID to query
  - `to_sql()` - Generate SQL query for fetching related objects

### Planned

#### Database Integration

- Automatic ContentType table creation and migration
- Persistence of ContentType instances to database
- Database-backed content type lookups
- Multi-database support for content types
- Foreign key constraints for GenericForeignKey fields

#### ORM Integration

- Generic relation fields for models (`GenericRelation`)
- Reverse generic relation queries
- Prefetch support for generic relations
- QuerySet integration for efficient generic relation queries
- Generic relation filtering and ordering

#### Permission System Integration

- Associate permissions with content types
- Permission checking utilities
- Content type-based authorization

#### Advanced Features

- Content type shortcuts (URL resolution for generic objects)
- Content type view mixins
- Admin interface integration for generic relations
- Automatic content type cleanup on model deletion
- Content type renaming and migration support

#### Management Commands

- `dumpdata`/`loaddata` support for content types
- Content type synchronization commands
- Content type inspection utilities
