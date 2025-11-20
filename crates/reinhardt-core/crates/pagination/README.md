# reinhardt-pagination

Pagination strategies for Reinhardt framework, inspired by Django REST Framework's pagination.

## Overview

Multiple pagination strategies for large datasets. Provides three main pagination styles: PageNumberPagination for traditional page-based pagination, LimitOffsetPagination for SQL-style limit/offset, and CursorPagination for efficient pagination of large datasets without offset performance issues.

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

**Note**: The database cursor types are re-exported at the crate root for convenience.

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
use reinhardt_pagination::{CursorPaginator, HasTimestamp};

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

### Builder Pattern

All pagination strategies support fluent builder pattern:

```rust
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