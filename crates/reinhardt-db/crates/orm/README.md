# reinhardt-orm

Django-inspired ORM with QuerySet API and database abstraction

## Overview

A powerful Object-Relational Mapping system inspired by Django's ORM and SQLAlchemy. Features include QuerySet API for chainable queries, model definitions, field types, validators, relationship management, and support for multiple database backends (PostgreSQL, MySQL, SQLite).

## Documentation

- **[README.md](README.md)** - Feature list and API reference (this file)
- **[USAGE_GUIDE.md](USAGE_GUIDE.md)** - Comprehensive usage guide with examples and best practices

**Quick Links:**
- [Basic Model Definition](USAGE_GUIDE.md#1-basic-model-definition)
- [CRUD Operations](USAGE_GUIDE.md#2-crud-operations)
- [Query Building](USAGE_GUIDE.md#3-query-building)
- [Transaction Management](USAGE_GUIDE.md#5-transaction-management)
- [Best Practices](USAGE_GUIDE.md#7-best-practices)

## Installation

Add `reinhardt` to your `Cargo.toml`:

```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["db-orm"] }

# Or use a preset:
# reinhardt = { version = "0.1.0-alpha.1", features = ["standard"] }  # Recommended
# reinhardt = { version = "0.1.0-alpha.1", features = ["full"] }      # All features
```

Then import ORM features:

```rust
use reinhardt::db::orm::{Model, QuerySet, DatabaseConnection};
use reinhardt::db::orm::fields::{AutoField, CharField, IntegerField, DateTimeField};
use reinhardt::db::orm::transaction::transaction;
use reinhardt::core::macros::model;  // For #[model(...)] attribute macro
```

**Note:** ORM features are included in the `standard` and `full` feature presets.

## Implemented ✓

### Core Model System

- **Model trait** - Core trait for database models with composition-based design
- **Timestamped trait** - Automatic created_at/updated_at timestamp management
- **SoftDeletable trait** - Soft delete functionality with deleted_at timestamps
- **Timestamps struct** - Composable timestamp fields (created_at, updated_at)
- **SoftDelete struct** - Composable soft delete field with restore capability

### Field Types

- **AutoField** - Auto-incrementing integer primary key
- **BigIntegerField** - 64-bit integer field
- **BooleanField** - Boolean field with default value support
- **CharField** - Text field with max_length, null/blank options, and choices
- **IntegerField** - Standard integer field with choices support
- **DateField** - Date field with auto_now and auto_now_add options
- **DateTimeField** - DateTime field with auto_now and auto_now_add options
- **DecimalField** - Decimal field with precision settings (max_digits, decimal_places)
- **EmailField** - Email field with validation and customizable max_length
- **FloatField** - Floating-point number field
- **TextField** - Large text field
- **TimeField** - Time field with auto_now options
- **URLField** - URL field with validation
- **BinaryField** - Raw binary data field (non-editable by default)
- **SlugField** - URL-friendly string field with db_index
- **SmallIntegerField** - Small integer field (-32768 to 32767)
- **PositiveIntegerField** - Positive integer field (0 to 2147483647)
- **PositiveSmallIntegerField** - Small positive integer field (0 to 32767)
- **PositiveBigIntegerField** - Large positive integer field
- **GenericIPAddressField** - IPv4/IPv6 address field with protocol filtering
- **FilePathField** - Filesystem path selection field with pattern matching

### PostgreSQL-Specific Fields

- **ArrayField** - PostgreSQL array type support
- **JSONBField** - PostgreSQL JSONB type support
- **HStoreField** - PostgreSQL key-value store field
- **CITextField** - Case-insensitive text field
- **IntegerRangeField** - Integer range field
- **BigIntegerRangeField** - Big integer range field
- **DateRangeField** - Date range field
- **DateTimeRangeField** - DateTime range field

### Relationship Fields

- **ForeignKey** - Many-to-one relationship with on_delete options
  - `tweet.user(&db).await` - Async accessor to load related instance
- **OneToOneField** - One-to-one relationship
  - `profile.user(&db).await` - Async accessor to load related instance
- **ManyToManyField** - Many-to-many relationship with through table support
  - `user.groups_accessor(db)` - Accessor for managing many-to-many relationships
- **ForeignKeyAccessor** - Type-safe accessor for reverse FK relationships
  - `Tweet::user_accessor().reverse(&user, db)` - Recommended reverse accessor creation
- **ReverseAccessor** - Helper for accessing reverse FK relationships
  - `ReverseAccessor::new(&user, "user_id", db)` - Manual reverse accessor creation

### Field Configuration

- **BaseField** - Common field attributes (null, blank, default, db_default, db_column, db_tablespace, primary_key, unique, editable, choices)
- **Field deconstruction** - Serializable field representation for migrations

### Validators

- **RequiredValidator** - Required field validation
- **MinLengthValidator** - Minimum length validation
- **MaxLengthValidator** - Maximum length validation
- **RangeValidator** - Numeric range validation
- **RegexValidator** - Regular expression pattern validation
- **EmailValidator** - Email address format validation
- **URLValidator** - URL format validation
- **FieldValidators** - Field-level validation container
- **ModelValidators** - Model-level validation container

### Query System

- **QuerySet** - Chainable query interface with filtering
- **Filter** - Query filtering with operators (Eq, Ne, Gt, Gte, Lt, Lte, In, NotIn, Contains, StartsWith, EndsWith)
- **Query** - Query building and execution
- **FilterOperator** - Comparison operators for filtering
- **FilterValue** - Type-safe filter value handling (String, Integer, Float, Boolean, Null)
- **select_related** - Eagerly load related objects using JOIN queries
- **prefetch_related** - Eagerly load related objects using separate queries
- **create()** - Create new record

### Database Manager (Django-compatible)

- **Manager** - Django-style model manager for database operations
- **all()** - Get all records as QuerySet
- **filter()** - Filter records by field and operator
- **get()** - Get single record by primary key
- **create()** - Create new record
- **update()** - Update existing record
- **delete()** - Delete record by primary key
- **count()** - Count records
- **bulk_create()** - Efficiently create multiple records in batches with conflict handling
- **bulk_update()** - Efficiently update multiple records in batches
- **get_or_create()** - Get existing or create new record with atomic operation
- **Global database connection** - init_database() and get_connection() for connection management

### Expressions & Query Fields

- **Q** - Complex query expressions with AND/OR logic
- **F** - Field reference expressions
- **FieldRef<M, T>** - Type-safe field references with compile-time type checking
- **Subquery** - Subquery expressions
- **Exists** - EXISTS clause support
- **OuterRef** - Reference to outer query fields
- **QOperator** - Query operators (And, Or, Not)
- **Field** - Query field representation
- **Lookup** - Field lookup operations (exact, iexact, contains, icontains, in, gt, gte, lt, lte, startswith, istartswith, endswith, iendswith, range, isnull, regex, iregex)
- **LookupType** - Typed lookup operations
- **Comparable** - Type-safe comparison operations
- **StringType, NumericType, DateTimeType** - Type-specific operations

#### Type-Safe Field References

The `#[model(...)]` attribute macro automatically generates type-safe field accessor methods that return `FieldRef<M, T>`:

> **Note**: The `#[model(...)]` attribute automatically applies `#[derive(Model)]`, so you should use only `#[model(...)]` without explicitly adding `#[derive(Model)]`.

```rust
use reinhardt::core::macros::Model;
use serde::{Deserialize, Serialize};

#[model(app_label = "users", table_name = "users")]
struct User {
    #[field(primary_key = true)]
    id: i64,

    #[field(max_length = 100)]
    username: String,

    #[field(max_length = 255)]
    email: String,
}

// Auto-generated by macro:
// impl User {
//     pub const fn field_id() -> FieldRef<User, i64> { ... }
//     pub const fn field_username() -> FieldRef<User, String> { ... }
//     pub const fn field_email() -> FieldRef<User, String> { ... }
// }

// Usage: Type-safe field references
let id_ref = User::field_id();          // FieldRef<User, i64>
let username_ref = User::field_username(); // FieldRef<User, String>

// Convert to F expression for queries
let f: F = User::field_email().into();
```

**Benefits:**
- ✅ Compile-time type checking (prevents typos in field names)
- ✅ IDE autocomplete support for all model fields
- ✅ Seamless conversion to F expressions via `Into<F>` trait
- ✅ Const evaluation for zero runtime overhead

**Migration from string-based field references:**
```rust
// Before (string-based, error-prone)
let query = User::objects().filter("username__icontains", "alice");

// After (type-safe, compiler-checked)
let query = User::objects().filter(User::field_username(), "alice");
```

### Functions

- **Aggregate functions** - Abs, Ceil, Floor, Round, Power, Sqrt, Mod
- **String functions** - Concat, Length, Lower, Upper, Substr, Trim (with TrimType)
- **Date/Time functions** - CurrentDate, CurrentTime, Now, Extract (with ExtractComponent)
- **Utility functions** - Cast (with SqlType), Coalesce, NullIf, Greatest, Least

### Window Functions

- **Window** - Window function support
- **Frame** - Frame specification (FrameType, FrameBoundary)
- **Ranking functions** - RowNumber, Rank, DenseRank, NTile
- **Value functions** - FirstValue, LastValue, NthValue, Lead, Lag

### Annotations & Aggregation

- **Annotation** - Query annotations
- **Expression** - Value expressions
- **Value** - Literal values in queries
- **When** - Conditional expressions

### Set Operations

- **SetOperation** - UNION, INTERSECT, EXCEPT operations
- **CombinedQuery** - Combined query results
- **SetOperationBuilder** - Fluent API for set operations

### Transaction Management

**Recommended API (Closure-based):**
- `transaction()` - Execute closure with automatic commit/rollback ✅ **Recommended**
- `transaction_with_isolation()` - Transaction with specific isolation level ✅ **Recommended**
- `TransactionScope::execute()` - Execute closure on existing transaction scope

**Low-level API:**
- `TransactionScope` - RAII transaction guard with automatic rollback on drop
- `TransactionScope::begin()` - Start a new transaction
- `TransactionScope::begin_with_isolation()` - Start transaction with isolation level
- `TransactionScope::begin_nested()` - Start nested transaction (savepoint)
- `TransactionScope::commit()` - Explicitly commit a transaction
- `TransactionScope::rollback()` - Explicitly rollback a transaction

**Alternative API (Legacy):**
- `atomic()` - Helper function for executing code within a transaction (use `transaction()` instead)
- `atomic_with_isolation()` - Atomic execution with specific isolation level (use `transaction_with_isolation()` instead)
- `Atomic` - Atomic transaction context (deprecated)

**Supporting Types:**
- `Transaction` - Database transaction management with SQL generation
- `Savepoint` - Nested transaction savepoints with SQL generation
- `IsolationLevel` - Transaction isolation level specification

### Transaction Usage Examples

#### Basic Transaction
```rust
use reinhardt::db::orm::transaction::transaction;
use reinhardt::db::orm::connection::DatabaseConnection;

async fn create_user(conn: &DatabaseConnection, name: &str) -> Result<i64, anyhow::Error> {
    transaction(conn, |_tx| async move {
        let id = insert_user(name).await?;
        update_user_count().await?;
        Ok(id)  // Auto-commit on success
    }).await  // Auto-rollback on error
}
```

#### Transaction with Isolation Level
```rust
use reinhardt::db::orm::transaction::{transaction_with_isolation, IsolationLevel};

async fn update_inventory(conn: &DatabaseConnection) -> Result<(), anyhow::Error> {
    transaction_with_isolation(conn, IsolationLevel::Serializable, |_tx| async move {
        let stock = get_current_stock().await?;
        if stock > 0 {
            decrement_stock().await?;
        }
        Ok(())
    }).await
}
```

#### Error Handling
```rust
use reinhardt::db::orm::transaction::transaction;

async fn transfer_money(
    conn: &DatabaseConnection,
    from: &str,
    to: &str,
    amount: i64,
) -> Result<(), anyhow::Error> {
    transaction(conn, |_tx| async move {
        debit_account(from, amount).await?;   // Error → auto-rollback
        credit_account(to, amount).await?;    // Error → auto-rollback
        Ok(())  // Success → auto-commit
    }).await
}
```

#### Using TransactionScope directly (advanced)
```rust
use reinhardt::db::orm::transaction::TransactionScope;

async fn complex_operation(conn: &DatabaseConnection) -> Result<(), anyhow::Error> {
    let tx = TransactionScope::begin(conn).await?;

    // Perform operations
    insert_record().await?;

    // Conditionally commit or rollback
    if some_condition {
        tx.commit().await?;
    } else {
        tx.rollback().await?;
    }

    Ok(())
}
```

### Database Connection

- **DatabaseConnection** - Connection abstraction with transaction support
  - `begin_transaction()` - Begin a transaction
  - `begin_transaction_with_isolation()` - Begin with specific isolation level
  - `commit_transaction()` - Commit the current transaction
  - `rollback_transaction()` - Rollback the current transaction
  - `savepoint()` - Create a savepoint for nested transactions
  - `release_savepoint()` - Release a savepoint
  - `rollback_to_savepoint()` - Rollback to a savepoint
- **DatabaseExecutor** - Query execution trait
- **DatabaseBackend** - Multiple database support (PostgreSQL, MySQL, SQLite)
- **QueryRow** - Query result row representation

### Indexes

- **Index** - Base index support
- **BTreeIndex** - B-tree index for ordered data
- **HashIndex** - Hash index for exact matches
- **GinIndex** - PostgreSQL GIN index for full-text search
- **GistIndex** - PostgreSQL GiST index for geometric data

### Constraints

- **Constraint** - Base constraint trait
- **UniqueConstraint** - Unique field constraints
- **CheckConstraint** - Check constraints with conditions
- **ForeignKeyConstraint** - Foreign key constraints
- **OnDelete** - Cascade delete behavior (Cascade, SetNull, SetDefault, Restrict, NoAction)
- **OnUpdate** - Cascade update behavior (Cascade, SetNull, SetDefault, Restrict, NoAction)

### Relationships (SQLAlchemy-inspired)

- **Relationship** - Relationship configuration
- **RelationshipType** - OneToOne, OneToMany, ManyToOne, ManyToMany
- **RelationshipDirection** - Bidirectional relationship support
- **CascadeOption** - Cascade operations (All, Delete, SaveUpdate, Merge, Expunge, DeleteOrphan, Refresh)

### Relationship Accessor Usage Examples

#### ForeignKey Accessor (Forward Relationship)
```rust
use reinhardt::db::orm::Model;

// Automatically generated by #[model(...)] macro
// Access related instance via FK field
let tweet = Tweet::objects().filter(...)...await?;
let user = tweet.user(&db).await?;  // Load related User instance
```

#### ManyToMany Accessor
```rust
// Access many-to-many relationship
let user = User::objects().filter(...)...await?;
let groups_accessor = user.groups_accessor(db.clone());

// Add relationship
groups_accessor.add(&group).await?;

// Get all related records
let groups = groups_accessor.all().await?;

// Count related records
let count = groups_accessor.count().await?;

// Pagination
let page1 = groups_accessor.paginate(1, 10).all().await?;
```

#### Reverse Accessor (Reverse ForeignKey Relationship)

**ForeignKeyAccessor method (recommended):**
```rust
// Tweet model has: #[rel(foreign_key)] user: ForeignKeyField<User>

let user = User::objects().filter(...)...await?;

// Use ForeignKeyAccessor for type-safe reverse access
let tweets_accessor = Tweet::user_accessor().reverse(&user, db.clone());

// Get all related tweets
let tweets = tweets_accessor.all().await?;

// Count related tweets
let tweet_count = tweets_accessor.count().await?;

// Pagination
let recent_tweets = tweets_accessor.paginate(1, 10).all().await?;
```

**Manual creation (for advanced use cases):**
```rust
use reinhardt::db::orm::ReverseAccessor;

// Manual reverse accessor creation for user → tweets relationship
let user = User::objects().filter(...)...await?;
let tweets_accessor = ReverseAccessor::<User, Tweet>::new(&user, "user_id", db.clone());

// Get all related tweets
let tweets = tweets_accessor.all().await?;

// Count related tweets
let tweet_count = tweets_accessor.count().await?;

// Pagination
let recent_tweets = tweets_accessor.limit(10).all().await?;
```

### Loading Strategies

- **LoadingStrategy** - Eager vs lazy loading
- **LoadOption** - Loading option configuration
- **LoadOptionBuilder** - Fluent API for loading options
- **LoadContext** - Loading context management
- **selectinload** - Load relationships with separate SELECT
- **joinedload** - Load relationships with JOIN
- **subqueryload** - Load relationships with subquery
- **lazyload** - Lazy load relationships
- **noload** - Do not load relationships
- **raiseload** - Raise error if relationship accessed

### Events System

- **EventRegistry** - Global event registration
- **EventListener** - Event listener trait
- **EventResult** - Event handling results
- **MapperEvents** - Model mapping events
- **SessionEvents** - Session lifecycle events
- **AttributeEvents** - Attribute modification events
- **InstanceEvents** - Instance lifecycle events

### Query Execution

- **QueryExecution** - Query execution interface
- **ExecutionResult** - Execution results
- **SelectExecution** - SELECT query execution
- **QueryCompiler** - Query compilation to SQL
- **ExecutableQuery** - Executable query trait
- **QueryFieldCompiler** - Field-level query compilation

### Type System

- **SqlValue** - SQL value types
- **SqlTypeDefinition** - SQL type definitions
- **TypeRegistry** - Type registration system
- **TypeDecorator** - Custom type decorators
- **DatabaseDialect** - Dialect-specific type handling
- **UuidType** - UUID type support
- **JsonType** - JSON type support
- **ArrayType** - Array type support
- **HstoreType** - PostgreSQL HStore type support
- **InetType** - IP address type support
- **TypeError** - Type conversion errors

### Registry System

- **MapperRegistry** - Model mapper registration
- **Mapper** - Model-to-table mapping
- **TableInfo** - Table metadata
- **ColumnInfo** - Column metadata
- **registry()** - Global registry access

### SQLAlchemy-style Query API

- **SelectQuery** - SQLAlchemy-style SELECT queries
- **select()** - Create SELECT query
- **column()** - Column reference in queries
- **SqlColumn** - Column representation
- **JoinType** - Join types (Inner, Left, Right, Full, Cross)

### Engine & Connection Management

- **Engine** - Database engine
- **EngineConfig** - Engine configuration
- **create_engine()** - Create database engine
- **create_engine_with_config()** - Create engine with config

### Query Options

- **QueryOptions** - Query execution options
- **QueryOptionsBuilder** - Fluent API for query options
- **ExecutionOptions** - Execution-specific options
- **ForUpdateMode** - Row locking modes (NoWait, SkipLocked, Update, KeyShare)
- **CompiledCacheOption** - Query compilation caching

### Async Query Support

- **AsyncQuery** - Asynchronous query execution
- **AsyncSession** - Asynchronous session management

### Many-to-Many Support

- **ManyToMany** - Many-to-many relationship helper
- **AssociationTable** - Junction table representation
- **association_table()** - Create association table
- **Type-safe accessor methods** - Auto-generated `{field_name}_accessor()` methods for ManyToManyField

The `#[model]` macro automatically generates type-safe accessor methods for each `ManyToManyField`:

```rust
use reinhardt_db::orm::associations::ManyToManyField;

#[model(app_label = "users", table_name = "users")]
struct User {
    #[field(primary_key = true)]
    id: Uuid,

    #[field(many_to_many)]
    following: ManyToManyField<User, User>,
}

// Auto-generated accessor method (type-safe, no string literals)
let accessor = user.following_accessor(db);
let followers = accessor.all().await?;

// Old API (still supported for backward compatibility)
let accessor = ManyToManyAccessor::<User, User>::new(&user, "following", db);
```

**Benefits:**
- Compile-time field name validation (no typos)
- Type inference for Source and Target models
- IDE auto-completion support
- Cleaner, more idiomatic API

### Bulk Operations

- **bulk_update** - Efficient bulk updates with field specification

### Typed Joins

- **TypedJoin** - Type-safe join operations

### Composite Primary Keys

- **composite_primary_key()** - Define multiple fields as composite primary key
- **get_composite_pk_values()** - Retrieve composite primary key values as HashMap
- **get_composite()** - Query by composite primary key values

Example:

```rust
use reinhardt::core::macros::Model;
use serde::{Deserialize, Serialize};

#[model(app_label = "test_app", table_name = "post_tags")]
struct PostTag {
    #[field(primary_key = true)]
    post_id: i64,

    #[field(primary_key = true)]
    tag_id: i64,

    #[field(max_length = 200)]
    description: String,
}

// Access composite primary key metadata
let composite_pk = PostTag::composite_primary_key();
assert!(composite_pk.is_some());

// Get composite PK values from instance
let post_tag = PostTag { post_id: 1, tag_id: 5, description: "Tech".to_string() };
let pk_values = post_tag.get_composite_pk_values();
```

### Database Indexes

- **index** - Mark field for database indexing via `#[field(index = true)]`
- **index_metadata()** - Retrieve index information for model fields

Example:

```rust
#[model(app_label = "test_app", table_name = "users")]
struct User {
    #[field(primary_key = true)]
    id: i64,

    #[field(index = true, max_length = 100)]
    email: String,

    #[field(index = true, max_length = 50)]
    username: String,
}

// Access index metadata
let indexes = User::index_metadata();
assert_eq!(indexes.len(), 2);
```

### Check Constraints

- **check** - Define CHECK constraints via `#[field(check = "expression")]`
- **constraint_metadata()** - Retrieve constraint information for model fields
- **ConstraintType** - Constraint types (Check, ForeignKey, Unique)

Example:

```rust
#[model(app_label = "test_app", table_name = "products")]
struct Product {
    #[field(primary_key = true)]
    id: i64,

    #[field(max_length = 100)]
    name: String,

    #[field(check = "price > 0")]
    price: f64,

    #[field(check = "quantity >= 0")]
    quantity: i32,
}

// Access constraint metadata
let constraints = Product::constraint_metadata();
let price_constraint = constraints.iter()
    .find(|c| c.name == "price_check")
    .expect("price_check constraint should exist");
assert_eq!(price_constraint.definition, "price > 0");
```

### Field Validators

- **email** - Email format validation via `#[field(email = true)]`
- **url** - URL format validation via `#[field(url = true)]`
- **min_length** - Minimum string length via `#[field(min_length = N)]`
- **min_value** - Minimum numeric value via `#[field(min_value = N)]`
- **max_value** - Maximum numeric value via `#[field(max_value = N)]`

Validators are stored in field metadata attributes and can be accessed at runtime.

Example:

```rust
#[model(app_label = "test_app", table_name = "users")]
struct User {
    #[field(primary_key = true)]
    id: i64,

    #[field(max_length = 100, email = true)]
    email: String,

    #[field(max_length = 200, url = true)]
    website: String,

    #[field(max_length = 100, min_length = 3)]
    username: String,

    #[field(min_value = 0, max_value = 120)]
    age: i32,
}

// Access validator metadata via field_metadata()
let fields = User::field_metadata();
let email_field = fields.iter()
    .find(|f| f.name == "email")
    .expect("email field should exist");
assert!(email_field.attributes.contains_key("email"));
```

### Hybrid Properties (via reinhardt-hybrid)

- **HybridProperty** - Properties that work at both instance and class level
- **HybridMethod** - Methods that work at both instance and class level
- **HybridComparator** - Custom comparison logic for hybrid properties

### Common Table Expressions (CTE)

- **CTE** - Common Table Expression (WITH clause) support
- **CTECollection** - Manage multiple CTEs in a query
- **CTEBuilder** - Fluent API for building CTEs
- **CTEPatterns** - Common CTE patterns (recursive, materialized)

### Type-Safe Joins

- **TypedJoin<L, R>** - Type-safe JOIN operations with compile-time type checking
  - Ensures join conditions match table relationships
  - Prevents incorrect JOIN usage at compile time
  - Generic over left (L) and right (R) table types

### Model Inspection

- **ModelInspector<M>** - Runtime model metadata inspection
  - `fields()` - Get all field metadata
  - `relations()` - Get all relationship information
  - `indexes()` - Get index definitions
  - `constraints()` - Get constraint information
- **FieldInspector** - Field-level metadata inspection
- **FieldInfo** - Field metadata (name, type, attributes)
- **RelationInfo** - Relationship metadata (type, target model, foreign key)
- **IndexInfo** - Index metadata (columns, unique, type)
- **ConstraintInfo** - Constraint metadata (type, definition, affected columns)
- **ConstraintType** - Constraint types (Check, ForeignKey, Unique, PrimaryKey)

## Testing

### Prerequisites

ORM tests require **Docker** for TestContainers integration:

```bash
# Verify Docker is running
docker version
docker ps
```

**Note**: Docker Desktop must be installed and running. See [Database Testing Guide](../../README.md#testing) for detailed setup instructions.

### Running ORM Tests

```bash
# Run all ORM tests (requires Docker)
cargo test --package reinhardt-orm --all-features

# Run specific test suite
cargo test --package reinhardt-orm --test orm_integration_tests
```

### TestContainers Usage

ORM tests automatically use TestContainers to provide isolated database instances:

```rust
use reinhardt::test::fixtures::postgres_container;
use rstest::*;

#[rstest]
#[tokio::test]
async fn test_orm_operations(
    #[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
    let (_container, pool, _port, _database_url) = postgres_container.await;

    // Use ORM with the provided connection pool
    let user = User {
        id: 1,
        username: "test_user".to_string(),
        email: "test@example.com".to_string(),
    };

    // Perform ORM operations
    user.save(&pool).await.unwrap();

    // Container automatically cleaned up after test
}
```

For comprehensive testing standards, see:
- [Parent Database Testing Guide](../../README.md#testing)
- [Testing Standards](../../../../docs/TESTING_STANDARDS.md)