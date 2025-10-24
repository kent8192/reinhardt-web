# reinhardt-filters

Type-safe filtering and ordering capabilities for Reinhardt framework

## Overview

Powerful filtering and ordering system that provides compile-time type-safe filtering using reinhardt-orm's Field system. Build complex queries with field lookups, multi-term search, and ordering with full type safety and zero runtime overhead.

## Implemented ✓

### Core Filter System

- **FilterBackend trait** - Async filtering interface for queryset manipulation
- **FilterError** - Comprehensive error handling for invalid parameters and queries
- **Type-safe filtering** - Compile-time checked field access using reinhardt-orm's Field<M, T>

### Query Filtering

- **QueryFilter<M>** - Type-safe query filter combining lookups and ordering
  - Add lookup conditions with `.add()` and `.add_all()`
  - Multiple ordering fields with `.order_by()` and `.order_by_all()`
  - OR group support with `.add_or_group()` for complex queries
  - Multi-term search integration with `.add_multi_term()`
  - Automatic SQL WHERE and ORDER BY clause compilation
  - All conditions combined with AND by default

### Field Ordering

- **OrderingField<M>** - Type-safe field ordering with direction
- **OrderDirection** - Ascending (Asc) or Descending (Desc) ordering
- **FieldOrderingExt** - Extension trait adding `.asc()` and `.desc()` to Field<M, T>
- **SQL generation** - Automatic ORDER BY clause generation with `.to_sql()`
- **Nested field support** - Handle complex field paths (e.g., "author.username")

### Multi-term Search

- **MultiTermSearch** - Search multiple terms across multiple fields
  - `.search_terms()` - Case-insensitive contains search (ICONTAINS)
  - `.exact_terms()` - Case-insensitive exact match (IEXACT)
  - `.prefix_terms()` - Prefix match search (STARTSWITH)
  - `.parse_search_terms()` - Parse comma-separated search strings with quote support
  - `.compile_to_sql()` - Generate SQL WHERE clauses for multi-term searches
- **Query logic** - Terms combined with AND, fields within each term combined with OR

### Searchable Model System

- **SearchableModel trait** - Define searchable fields and default ordering for models
  - `.searchable_fields()` - Specify which string fields support text search
  - `.default_ordering()` - Define default sort order for model queries
  - `.searchable_field_names()` - Helper to extract field names as strings

## Planned

### Advanced Filtering

- Range filters for date and numeric fields
- Geographic/spatial filtering
- Full-text search integration
- Custom filter backends for specialized use cases

### Query Optimization

- Query result caching
- Intelligent index usage
- Query plan optimization hints

### Enhanced Search

- Fuzzy search support
- Relevance scoring
- Synonym handling
- Search result highlighting

## Usage Examples

### Basic Query Filtering

```rust
use reinhardt_filters::QueryFilter;
use reinhardt_orm::Field;

// Create a filter with lookups and ordering
let filter = QueryFilter::<Post>::new()
    .add(Field::new(vec!["title"]).icontains("rust"))
    .add(Field::new(vec!["created_at"]).year().gte(2024))
    .order_by(Field::new(vec!["title"]).asc());
```

### Multi-term Search

```rust
use reinhardt_filters::MultiTermSearch;

// Search for posts containing "rust" AND "programming"
let terms = vec!["rust", "programming"];
let lookups = MultiTermSearch::search_terms::<Post>(terms);

// Generates: (title ICONTAINS 'rust' OR content ICONTAINS 'rust')
//        AND (title ICONTAINS 'programming' OR content ICONTAINS 'programming')
```

### Searchable Model

```rust
use reinhardt_filters::{SearchableModel, FieldOrderingExt};
use reinhardt_orm::{Model, Field};

impl SearchableModel for Post {
    fn searchable_fields() -> Vec<Field<Self, String>> {
        vec![
            Field::new(vec!["title"]),
            Field::new(vec!["content"]),
        ]
    }

    fn default_ordering() -> Vec<OrderingField<Self>> {
        vec![Field::new(vec!["created_at"]).desc()]
    }
}
```

## Integration

Works seamlessly with:

- **reinhardt-orm** - Type-safe Field<M, T> system and QuerySet
- **reinhardt-viewsets** - Automatic filtering in ViewSet responses
- **reinhardt-rest** - Query parameter parsing and validation
