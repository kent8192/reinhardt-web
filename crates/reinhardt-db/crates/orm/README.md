# reinhardt-orm

Django-inspired ORM with QuerySet API and database abstraction

## Overview

A powerful Object-Relational Mapping system inspired by Django's ORM and SQLAlchemy. Features include QuerySet API for chainable queries, model definitions, field types, validators, relationship management, and support for multiple database backends (PostgreSQL, MySQL, SQLite).

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
- **OneToOneField** - One-to-one relationship
- **ManyToManyField** - Many-to-many relationship with through table support

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
- **create()** - Create new record (requires `django-compat` feature)

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
- **Subquery** - Subquery expressions
- **Exists** - EXISTS clause support
- **OuterRef** - Reference to outer query fields
- **QOperator** - Query operators (And, Or, Not)
- **Field** - Query field representation
- **Lookup** - Field lookup operations (exact, iexact, contains, icontains, in, gt, gte, lt, lte, startswith, istartswith, endswith, iendswith, range, isnull, regex, iregex)
- **LookupType** - Typed lookup operations
- **Comparable** - Type-safe comparison operations
- **StringType, NumericType, DateTimeType** - Type-specific operations

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

### Transactions

- **Transaction** - Database transaction management with SQL generation
- **TransactionScope** - RAII transaction guard with automatic rollback on drop
- **IsolationLevel** - Transaction isolation levels (ReadUncommitted, ReadCommitted, RepeatableRead, Serializable)
- **TransactionState** - Transaction state tracking (NotStarted, Active, Committed, RolledBack)
- **Savepoint** - Nested transaction savepoints with SQL generation
- **atomic()** - Helper function for executing code within a transaction
- **atomic_with_isolation()** - Atomic execution with specific isolation level
- **Atomic** - Atomic transaction context (legacy)

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
use reinhardt_macros::Model;
use serde::{Deserialize, Serialize};

#[derive(Model, Serialize, Deserialize, Clone, Debug)]
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
#[derive(Model, Serialize, Deserialize, Clone, Debug)]
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
#[derive(Model, Serialize, Deserialize, Clone, Debug)]
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
#[derive(Model, Serialize, Deserialize, Clone, Debug)]
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