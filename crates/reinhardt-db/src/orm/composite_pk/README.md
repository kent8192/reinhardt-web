# Composite Primary Key Support

## Overview

This module provides full support for composite primary keys in Reinhardt ORM, addressing one of Django's well-known limitations.

**Django Limitation**: Django only supports single-column primary keys.

**Reinhardt Solution**: Full composite primary key support with type-safe validation and SQL generation.

## Features

- ✅ Multiple field composite keys
- ✅ Automatic SQL generation (`PRIMARY KEY (field1, field2, ...)`)
- ✅ WHERE clause generation for queries
- ✅ Type-safe value validation
- ✅ Custom constraint naming
- ✅ Integration with Reinhardt's constraint system

## Quick Start

### Defining a Composite Primary Key

```rust
use reinhardt_orm::composite_pk::CompositePrimaryKey;

// Create a composite key with two fields
let pk = CompositePrimaryKey::new(vec![
    "user_id".to_string(),
    "role_id".to_string(),
])?;

// Or with a custom constraint name
let pk = CompositePrimaryKey::with_name(
    vec!["user_id".to_string(), "role_id".to_string()],
    "user_role_pk"
)?;
```

### Using in Model Definitions

```rust
use reinhardt_orm::{Model, composite_pk::CompositePrimaryKey};

#[model(
    table_name = "user_roles",
    composite_pk = ["user_id", "role_id"]
)]
pub struct UserRole {
    pub user_id: i64,
    pub role_id: i64,
    pub assigned_at: DateTime<Utc>,
}
```

### Generating SQL

```rust
let pk = CompositePrimaryKey::new(vec![
    "user_id".to_string(),
    "order_id".to_string(),
])?;

// Generates: PRIMARY KEY (user_id, order_id)
let sql = pk.to_sql();
```

### Querying by Composite Key

```rust
use reinhardt_orm::composite_pk::{CompositePrimaryKey, PkValue};
use std::collections::HashMap;

let pk = CompositePrimaryKey::new(vec![
    "user_id".to_string(),
    "role_id".to_string(),
])?;

let mut values = HashMap::new();
values.insert("user_id".to_string(), PkValue::Int(100));
values.insert("role_id".to_string(), PkValue::Int(5));

// Generates: user_id = 100 AND role_id = 5
let where_clause = pk.to_where_clause(&values)?;
```

## Supported Value Types

The `PkValue` enum supports the following types:

- `String` - Text values
- `Int(i64)` - Signed integers
- `Uint(u64)` - Unsigned integers
- `Bool` - Boolean values

### Automatic Conversions

```rust
use reinhardt_orm::composite_pk::PkValue;

// From various Rust types
let v1: PkValue = 42i32.into();        // Int(42)
let v2: PkValue = 100u64.into();       // Uint(100)
let v3: PkValue = "test".into();       // String("test")
let v4: PkValue = true.into();         // Bool(true)
```

## Advanced Usage

### Validation

```rust
let pk = CompositePrimaryKey::new(vec![
    "id".to_string(),
    "type".to_string(),
])?;

let mut values = HashMap::new();
values.insert("id".to_string(), PkValue::Int(1));
values.insert("type".to_string(), PkValue::String("admin".to_string()));

// Validate all required fields are present
pk.validate(&values)?;
```

### Field Inspection

```rust
let pk = CompositePrimaryKey::new(vec![
    "a".to_string(),
    "b".to_string(),
    "c".to_string(),
])?;

// Get all fields
assert_eq!(pk.fields(), &["a", "b", "c"]);

// Check field count
assert_eq!(pk.field_count(), 3);

// Check if field exists
assert!(pk.contains_field("a"));
assert!(!pk.contains_field("d"));
```

### SQL String Escaping

Values are automatically escaped for SQL safety:

```rust
let value = PkValue::String("O'Brien".to_string());
assert_eq!(value.to_sql_string(), "'O''Brien'");  // Single quotes escaped
```

## Common Use Cases

### 1. Many-to-Many Join Tables

```rust
#[model(
    table_name = "user_groups",
    composite_pk = ["user_id", "group_id"]
)]
pub struct UserGroup {
    pub user_id: i64,
    pub group_id: i64,
    pub joined_at: DateTime<Utc>,
}
```

### 2. Multi-Tenant Systems

```rust
#[model(
    table_name = "tenant_resources",
    composite_pk = ["tenant_id", "resource_id"]
)]
pub struct TenantResource {
    pub tenant_id: i64,
    pub resource_id: i64,
    pub quota: i32,
}
```

### 3. Time-Series Data

```rust

#[model(
    table_name = "metrics",
    composite_pk = ["device_id", "timestamp"]
)]
pub struct Metric {
    pub device_id: String,
    pub timestamp: i64,
    pub value: f64,
}
```

### 4. Geographical Data

```rust

#[model(
    table_name = "locations",
    composite_pk = ["country_code", "city_code", "postal_code"]
)]
pub struct Location {
    pub country_code: String,
    pub city_code: String,
    pub postal_code: String,
    pub latitude: f64,
    pub longitude: f64,
}
```

## Error Handling

```rust
use reinhardt_orm::composite_pk::{CompositePrimaryKey, CompositePkError};

// Empty fields error
let result = CompositePrimaryKey::new(vec![]);
assert!(matches!(result, Err(CompositePkError::EmptyFields)));

// Duplicate field error
let result = CompositePrimaryKey::new(vec![
    "id".to_string(),
    "id".to_string(),  // Duplicate!
]);
assert!(matches!(result, Err(CompositePkError::DuplicateField(_))));

// Missing field error during validation
let pk = CompositePrimaryKey::new(vec![
    "user_id".to_string(),
    "role_id".to_string(),
])?;

let mut values = HashMap::new();
values.insert("user_id".to_string(), PkValue::Int(1));
// Missing role_id!

let result = pk.validate(&values);
assert!(matches!(result, Err(CompositePkError::MissingField(_))));
```

## Comparison with Django

| Feature | Django | Reinhardt |
|---------|--------|-----------|
| Single PK | ✅ Yes | ✅ Yes |
| Composite PK | ❌ No | ✅ Yes |
| Type Safety | ⚠️ Runtime | ✅ Compile-time |
| Custom Names | ✅ Yes | ✅ Yes |
| SQL Generation | ✅ Yes | ✅ Yes |
| Validation | ⚠️ Runtime | ✅ Compile-time + Runtime |

## Performance Considerations

- Composite keys are validated at compile-time when possible
- SQL generation is optimized with pre-allocated strings
- No runtime overhead for type conversions
- Zero-cost abstractions via Rust's type system

## Integration with ORM

Composite primary keys integrate seamlessly with Reinhardt's ORM:

```rust
use reinhardt_orm::{QuerySet, Model};

// Find by composite key
let user_role = UserRole::objects()
    .filter(user_id__eq(100))
    .filter(role_id__eq(5))
    .first()
    .await?;

// Update with composite key
UserRole::objects()
    .filter(user_id__eq(100))
    .filter(role_id__eq(5))
    .update(assigned_at=now())
    .await?;

// Delete with composite key
UserRole::objects()
    .filter(user_id__eq(100))
    .filter(role_id__eq(5))
    .delete()
    .await?;
```

## Testing

The module includes comprehensive tests covering:

- Valid composite key creation
- Error cases (empty fields, duplicates)
- SQL generation
- WHERE clause generation
- Value type conversions
- Field inspection
- Constraint trait implementation

Run tests with:

```bash
cargo test --package reinhardt-orm --lib composite_pk
```

## See Also

- [Constraints Module](../constraints.rs) - General constraint system
- [Model Macro](../../reinhardt-macros) - Model definition macros
- [Django Documentation](https://docs.djangoproject.com/en/stable/topics/db/models/#composite-primary-keys) - Django's limitations

## Contributing

Composite primary key support is actively maintained. If you encounter issues or have suggestions:

1. Check existing issues on GitHub
2. Review the test suite for examples
3. Submit a PR with tests for new features

---

**Status**: ✅ Fully Implemented (Since v0.1.0)

**Django Parity**: ✅ Exceeds Django capabilities
