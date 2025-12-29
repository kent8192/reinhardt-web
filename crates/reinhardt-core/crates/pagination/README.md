# reinhardt-pagination

Pagination strategies for Reinhardt framework, inspired by Django REST Framework's pagination.

## Overview

Multiple pagination strategies for large datasets. Provides three main pagination styles: PageNumberPagination for traditional page-based pagination, LimitOffsetPagination for SQL-style limit/offset, and CursorPagination for efficient pagination of large datasets without offset performance issues.

## Installation

Add `reinhardt` to your `Cargo.toml`:

```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["pagination"] }

# Or use a preset:
# reinhardt = { version = "0.1.0-alpha.1", features = ["standard"] }  # Recommended
# reinhardt = { version = "0.1.0-alpha.1", features = ["full"] }      # All features
```

Then import pagination features:

```rust
use reinhardt::core::pagination::{PageNumberPagination, LimitOffsetPagination, CursorPagination};
use reinhardt::core::pagination::{Paginator, AsyncPaginator, PaginatedResponse};

// For database cursor pagination
use reinhardt::core::pagination::{CursorPaginator, HasTimestamp, DatabaseCursor};
```

**Note:** Pagination features are included in the `standard` and `full` feature presets.

## Implemented ✓

### Core Features

- **PaginatedResponse** - Generic paginated response wrapper with count, next/previous links, and results
- **PaginationMetadata** - Pagination metadata structure for API responses
- **Page** - Comprehensive page representation with Django-like API
  - Navigation: `has_next()`, `has_previous()`, `has_other_pages()`
  - Page numbers: `next_page_number()`, `previous_page_number()`, `page_range()`
  - Elided ranges: `get_elided_page_range()` for long page lists with ellipsis
  - Indexing: `start_index()`, `end_index()`, `len()`, `is_empty()`
  - Direct access: `get()`, `get_slice()`, Index trait support, IntoIterator support

#### Utilities

- **PaginatorImpl** - Enum wrapper for Paginator implementations
  - Enables `dyn Paginator` compatibility by wrapping concrete pagination types
  - Variants: `PageNumber`, `LimitOffset`, `Cursor`
  - Implements both `Paginator` and `AsyncPaginator` traits

#### Relay Pagination Types

GraphQL Relay Cursor Connections Specification-compliant type definitions:

- **Edge<T>** - Relay connection edge (item + cursor)
  - `node` - The actual data item
  - `cursor` - Opaque cursor string for this edge
- **PageInfo** - Pagination metadata for Relay connections
  - `has_next_page` - Whether more items exist after this page
  - `has_previous_page` - Whether more items exist before this page
  - `start_cursor` - Cursor of the first edge
  - `end_cursor` - Cursor of the last edge
- **Connection<T>** - Relay connection wrapper
  - `edges` - List of edges with their cursors
  - `page_info` - Pagination information
  - `total_count` - Optional total count of items
- **RelayPagination** - Relay-style paginator with `first`/`after` and `last`/`before` parameters

#### Configuration

- **ErrorMessages** - Customizable error messages for PageNumberPagination
  - `invalid_page` - Error message for invalid page parameter
  - `min_page` - Error message for page numbers less than 1
  - `no_results` - Error message for pages with no results

### Pagination Strategies

#### PageNumberPagination

Traditional page-based pagination with page numbers.

- URL format: `?page=2&page_size=10`
- Configurable page size and maximum page size
- Custom query parameter names
- Orphans support: merge small last pages with previous page
- Last page shortcuts: support "last" keyword
- Empty first page handling
- Lenient `get_page()`: returns valid page even with invalid input
- Custom error messages
- Async support: `aget_page()` and `apaginate()`

#### LimitOffsetPagination

SQL-style limit/offset pagination.

- URL format: `?limit=10&offset=20`
- Configurable default and maximum limit
- Custom query parameter names
- Automatic URL building for next/previous links
- Input validation for limit and offset values
- Async support: `apaginate()`

#### CursorPagination

Cursor-based pagination for consistent results in large, changing datasets.

- URL format: `?cursor=<encoded_cursor>&page_size=10`
- Opaque cursor tokens (base64-encoded with checksum)
- Cursor security: timestamp validation and checksum verification
- Cursor expiry: 24-hour automatic expiration
- Configurable page size with maximum limit
- Custom ordering field support
- Tamper-proof cursor encoding
- Async support: `apaginate()`

##### Database-Integrated Cursor Pagination

The `cursor` module provides efficient cursor-based pagination that integrates directly with database queries, achieving **O(k) performance** instead of the O(n) cost of OFFSET/LIMIT pagination.

**Types:**
- `Direction` - Pagination direction (`Forward` or `Backward`)
- `DatabaseCursor` - Database cursor structure (id + timestamp)
- `CursorPaginator` - O(k) performance paginator
- `DatabaseCursorPaginatedResponse<T>` - Response type with next/previous cursors
- `HasTimestamp` trait - Required for models (provides `id()` and `timestamp()` methods)
- `DatabasePaginationError` - Database cursor pagination errors

**Note**: The database cursor types are re-exported at the crate root with `Database` prefix to avoid naming conflicts:
- `cursor::Cursor` → `DatabaseCursor`
- `cursor::CursorPaginatedResponse` → `DatabaseCursorPaginatedResponse`
- `cursor::PaginationError` → `DatabasePaginationError`

**Performance Characteristics:**

- **Space Complexity**: O(n) → O(1)
  - OFFSET/LIMIT: O(n) - loads data into memory
  - Cursor: O(page_size) - only current page
- **Time Complexity**: O(n+k) → O(k)
  - OFFSET/LIMIT: O(n) - skips data + O(k) fetch
  - Cursor: O(k) - uses index only
- **Real-world Impact**: 1000x+ improvement on deep pages
  - Page 1: Both similar (~5ms)
  - Page 1000: Cursor = ~5ms, OFFSET/LIMIT = ~5000ms

**Advantages:**

1. **Consistency**: No page drift when data is inserted/deleted
2. **Performance**: Leverages database indexes for constant-time seeks
3. **Scalability**: Performance remains constant regardless of page depth

**Limitations:**

1. **No Random Access**: Cannot jump directly to "page 100"
2. **Forward-Only Navigation**: Going backwards requires additional implementation (prev_cursor)
3. **Ordering Constraints**: Requires stable ordering by id + timestamp

**Usage Example:**

```rust
use reinhardt::core::pagination::{CursorPaginator, HasTimestamp};

#[derive(Clone)]
struct User {
    id: i64,
    created_at: i64,
    name: String,
}

impl HasTimestamp for User {
    fn id(&self) -> i64 { self.id }
    fn timestamp(&self) -> i64 { self.created_at }
}

let paginator = CursorPaginator::new(10);
let page1 = paginator.paginate(&users, None).unwrap();

// Navigate to next page
let page2 = paginator.paginate(&users, page1.next_cursor).unwrap();
```

**Database Requirements:**

- Composite index on `(id, timestamp)` columns is required for optimal performance
- Without proper indexing, performance degrades to O(n)

### Traits

- **Paginator** - Synchronous pagination trait for custom implementations
- **AsyncPaginator** - Asynchronous pagination trait with `apaginate()` method
- **SchemaParameter** - OpenAPI/documentation schema generation support
- **CursorEncoder** - Custom cursor encoding strategy trait
  - `encode()` - Encode position to opaque cursor string
  - `decode()` - Decode cursor string to position
  - Default implementation: `Base64CursorEncoder`
- **HasTimestamp** - Trait for models with id and timestamp fields (required for database cursor pagination)
  - `id()` - Get model's unique identifier
  - `timestamp()` - Get model's timestamp for ordering
- **OrderingStrategy** - Custom ordering strategy for cursor pagination
  - Implementations: `CreatedAtOrdering`, `IdOrdering`

### Builder Pattern

All pagination strategies support fluent builder pattern:

```rust
use reinhardt::core::pagination::{PageNumberPagination, LimitOffsetPagination, CursorPagination};

// PageNumberPagination
let paginator = PageNumberPagination::new()
    .page_size(20)
    .max_page_size(100)
    .page_size_query_param("limit")
    .orphans(3)
    .allow_empty_first_page(false);

// LimitOffsetPagination
let paginator = LimitOffsetPagination::new()
    .default_limit(25)
    .max_limit(100);

// CursorPagination
let paginator = CursorPagination::new()
    .page_size(20)
    .max_page_size(50)
    .ordering(vec!["-created_at".to_string(), "id".to_string()]);
```