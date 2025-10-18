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

## Planned

- Database integration for direct QuerySet pagination
- Custom cursor encoding strategies
- Configurable cursor expiry time
- Bi-directional cursor pagination
- Relay-style cursor pagination
- Custom ordering strategies for cursor pagination
- Performance optimizations for very large datasets
