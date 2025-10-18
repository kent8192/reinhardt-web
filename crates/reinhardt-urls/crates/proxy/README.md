# reinhardt-proxy

SQLAlchemy-style association proxies for transparent attribute access through relationships.

## Overview

Association proxies allow you to access attributes on related objects as if they were attributes on the parent object. This is particularly useful for many-to-many relationships where you want to work with related objects' attributes directly.

## Features

### Implemented ✓

#### Core Association Proxy (`proxy.rs`)
- `AssociationProxy<T, U>` - Main proxy type for relationship traversal
- `ProxyAccessor` trait - Interface for getting/setting proxy targets
- `ProxyTarget` enum - Represents scalar or collection proxy results
- `ScalarValue` enum - Type-safe scalar value representation (String, Integer, Float, Boolean, Null)
- Creator function support for new associations
- Comprehensive type conversion methods (`as_string()`, `as_integer()`, `as_float()`, `as_boolean()`)

#### Scalar Proxies (`scalar.rs`)
- `ScalarProxy` - For one-to-one and many-to-one relationships
- `ScalarComparison` enum - Rich comparison operators (Eq, Ne, Gt, Gte, Lt, Lte, In, NotIn, IsNull, IsNotNull, Like, NotLike)
- Async get/set operations for scalar values
- Builder methods for all comparison types

#### Collection Proxies (`collection.rs`)
- `CollectionProxy` - For one-to-many and many-to-many relationships
- Unique value support with deduplication
- Collection manipulation methods:
  - `get_values()` - Extract all values from related objects
  - `set_values()` - Replace entire collection
  - `append()` - Add single value
  - `remove()` - Remove matching values
  - `contains()` - Check for value existence
  - `count()` - Get collection size
- Advanced filtering:
  - `filter()` - Filter with FilterCondition
  - `filter_by()` - Filter with custom predicate
- `CollectionOperations` - Wrapper for transformation operations (filter, map, sort, distinct)
- `CollectionAggregations` - Wrapper for aggregation operations (sum, avg, min, max)

#### Query Filtering (`query.rs`)
- `FilterOp` enum - Filter operations (Eq, Ne, Lt, Le, Gt, Ge, In, NotIn, Contains, StartsWith, EndsWith)
- `FilterCondition` - Condition with field, operator, and value
- `QueryFilter` - Container for multiple conditions
- `matches()` method for evaluating conditions against ScalarValue

#### Join Operations (`joins.rs`)
- `JoinConfig` - Configuration for eager/lazy loading
- `LoadingStrategy` enum - Eager, Lazy, Select strategies
- `NestedProxy` - Multi-level relationship traversal
- `RelationshipPath` - Path representation for relationships
- Helper functions:
  - `extract_through_path()` - Parse dot-separated paths
  - `filter_through_path()` - Filter path segments
  - `traverse_and_extract()` - Extract from nested proxies
  - `traverse_relationships()` - Navigate relationship paths

#### Builder Pattern (`builder.rs`)
- `ProxyBuilder<T, U>` - Fluent API for proxy construction
- Method chaining for configuration:
  - `relationship()` - Set relationship name
  - `attribute()` - Set attribute name
  - `creator()` - Set creator function
- Safe construction methods:
  - `build()` - Build with panic on missing config
  - `try_build()` - Build returning Option
- `association_proxy()` helper function

#### Reflection System (`reflection.rs`)
- `Reflectable` trait - Core trait for runtime introspection
  - `get_relationship()` / `get_relationship_mut()` - Access relationships
  - `get_attribute()` / `set_attribute()` - Access attributes
  - `get_relationship_attribute()` / `set_relationship_attribute()` - Nested access
  - `has_relationship()` / `has_attribute()` - Existence checks
- `ProxyCollection` trait - Unified collection interface
  - Generic implementation for `Vec<T>`
  - Methods: `items()`, `add()`, `remove()`, `contains()`, `len()`, `clear()`
- `AttributeExtractor` trait - Scalar value extraction interface
- Helper functions:
  - `downcast_relationship()` - Type-safe downcasting
  - `extract_collection_values()` - Bulk value extraction

#### Error Handling
- `ProxyError` enum with comprehensive error types:
  - `RelationshipNotFound` - Missing relationship
  - `AttributeNotFound` - Missing attribute
  - `TypeMismatch` - Type conversion errors
  - `InvalidConfiguration` - Configuration errors
  - `DatabaseError` - Database operation errors
  - `SerializationError` - Serialization errors
- `ProxyResult<T>` type alias for Result with ProxyError

### Planned

Currently all planned features are implemented.

