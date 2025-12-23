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

#### Database Integration

- **ContentTypePersistence** - Database-backed content type storage
  - `new()` - Create persistence backend with database URL
  - `from_pool()` - Create from existing connection pool
  - `create_table()` - Automatic table creation with indexes
  - `get()`, `get_by_id()` - Retrieve content types from database
  - `get_or_create()` - Get existing or create new content type in database
  - `save()`, `delete()` - Persist and remove content types
  - `load_all()` - Load all content types from database
  - `exists()` - Check content type existence
  - Supports PostgreSQL, MySQL, and SQLite via sqlx

- **Multi-Database Support**
  - `MultiDbContentTypeManager` - Manage content types across multiple databases
  - Per-database content type registries with isolated caching
  - Cross-database content type searches
  - Database routing for content type operations
  - `add_database()` - Register new database connections
  - `search_all_databases()` - Find content types across all databases
  - `list_databases()` - Get all registered database names

- **GenericForeignKey Constraints**
  - Database-level validation for generic foreign keys
  - `validate_content_type()` - Verify content type exists in database
  - `get_validated_content_type()` - Retrieve validated content type from database

#### ORM Integration

- **ContentTypeQuery** - ORM-style query builder for content types
  - `new()` - Create query builder from connection pool
  - `filter_app_label()`, `filter_model()`, `filter_id()` - Filter by fields
  - `order_by_app_label()`, `order_by_model()`, `order_by_id()` - Sorting
  - `order_by_*_desc()` - Descending order variants
  - `limit()`, `offset()` - Pagination support
  - `all()` - Execute query and get all results
  - `first()` - Get first result
  - `count()` - Count matching records
  - `exists()` - Check if any records match
  - Django-inspired QuerySet API with method chaining

- **ContentTypeTransaction** - Transaction-aware content type operations
  - `new()` - Create transaction context
  - `query()` - Get query builder for transaction
  - `create()` - Create content type within transaction
  - `delete()` - Delete content type within transaction
  - Full ACID transaction support for content type operations
