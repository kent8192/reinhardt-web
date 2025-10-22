// Core modules - always available
pub mod aggregation;
pub mod annotation;
pub mod bulk_update;
pub mod connection;
pub mod constraints;
pub mod expressions;
pub mod fields;
pub mod functions;
pub mod hybrid_dml;
pub mod indexes;
pub mod inspection;
pub mod model;
pub mod query_fields;
pub mod set_operations;
pub mod transaction;
pub mod typed_join;
pub mod validators;
pub mod window;

// New advanced features
pub mod absolute_url_overrides;
pub mod composite_pk;
pub mod composite_synonym;
pub mod cte;
pub mod file_fields;
pub mod filtered_relation;
pub mod generated_field;
pub mod gis;
pub mod lambda_stmt;
pub mod lateral_join;
pub mod order_with_respect_to;
pub mod pool_types;
pub mod postgres_features;
pub mod postgres_fields;
pub mod two_phase_commit;
pub mod type_decorator;

// SQLAlchemy-style modules - default
pub mod async_query;
pub mod database_routing;
pub mod declarative;
pub mod engine;
pub mod events;
pub mod execution;
pub mod instrumentation;
pub mod loading;
pub mod many_to_many;
pub mod polymorphic;
pub mod query_execution;
pub mod query_options;
pub mod reflection;
pub mod registry;
pub mod relationship;
pub mod session;
pub mod sqlalchemy_query;
pub mod types;

// Django ORM compatibility - optional feature
#[cfg(feature = "django-compat")]
pub mod manager;

// Unified query interface facade
pub mod query;

#[cfg(feature = "django-compat")]
pub use manager::{get_connection, init_database};

// Core exports - always available
pub use aggregation::{Aggregate, AggregateFunc, AggregateResult, AggregateValue};
pub use annotation::{Annotation, AnnotationValue, Expression, Value, When};
pub use connection::{DatabaseBackend, DatabaseConnection, DatabaseExecutor, QueryRow};
pub use constraints::{
    CheckConstraint, Constraint, ForeignKeyConstraint, OnDelete, OnUpdate, UniqueConstraint,
};
pub use expressions::{Exists, OuterRef, QOperator, Subquery, F, Q};
pub use functions::{
    Abs, Cast, Ceil, Concat, CurrentDate, CurrentTime, Extract, ExtractComponent, Floor, Greatest,
    Least, Length, Lower, Mod, Now, NullIf, Power, Round, SqlType, Sqrt, Substr, Trim, TrimType,
    Upper,
};
pub use indexes::{BTreeIndex, GinIndex, GistIndex, HashIndex, Index};
pub use model::{Model, SoftDeletable, SoftDelete, Timestamped, Timestamps};
pub use query_fields::{
    Comparable, DateTimeType, Field, Lookup, LookupType, LookupValue, NumericType,
    QueryFieldCompiler, StringType,
};
pub use set_operations::{CombinedQuery, SetOperation, SetOperationBuilder};
pub use transaction::{Atomic, IsolationLevel, Savepoint, Transaction, TransactionState};
pub use validators::{
    EmailValidator, FieldValidators, MaxLengthValidator, MinLengthValidator, ModelValidators,
    RangeValidator, RegexValidator, RequiredValidator, URLValidator, ValidationError, Validator,
};
pub use window::{
    DenseRank, FirstValue, Frame, FrameBoundary, FrameType, Lag, LastValue, Lead, NTile, NthValue,
    Rank, RowNumber, Window, WindowFunction,
};

// PostgreSQL-specific types
pub use postgres_fields::{
    ArrayField, BigIntegerRangeField, CITextField, DateRangeField, DateTimeRangeField, HStoreField,
    IntegerRangeField, JSONBField,
};

// PostgreSQL-specific advanced features
pub use postgres_features::{ArrayAgg, ArrayOverlap, FullTextSearch, JsonbBuildObject};

// File field types
pub use file_fields::{FileField, FileFieldError, ImageField};

pub use database_routing::DatabaseRouter;
pub use events::{
    event_registry, AttributeEvents, EventListener, EventRegistry, EventResult, InstanceEvents,
    MapperEvents, SessionEvents,
};
pub use execution::{ExecutionResult, QueryExecution, SelectExecution};
// Re-export from reinhardt-hybrid
pub use loading::{
    joinedload, lazyload, noload, raiseload, selectinload, subqueryload, LoadContext, LoadOption,
    LoadOptionBuilder, LoadingStrategy,
};
pub use polymorphic::{
    polymorphic_registry, InheritanceType, PolymorphicConfig, PolymorphicIdentity,
    PolymorphicQuery, PolymorphicRegistry, PolymorphicRelation,
};
pub use query_options::{
    CompiledCacheOption, ExecutionOptions, ForUpdateMode, IsolationLevel as QueryIsolationLevel,
    QueryOptions, QueryOptionsBuilder,
};
pub use registry::{registry, ColumnInfo, Mapper, MapperRegistry, TableInfo};
pub use reinhardt_hybrid::{
    Comparator as HybridComparator, HybridMethod, HybridProperty, UpperCaseComparator,
};
pub use relationship::{CascadeOption, Relationship, RelationshipDirection, RelationshipType};
pub use session::{Session, SessionError};
pub use sqlalchemy_query::{column, select, Column as SqlColumn, JoinType, SelectQuery};
pub use typed_join::TypedJoin;
pub use types::{
    ArrayType, DatabaseDialect, HstoreType, InetType, JsonType, SqlTypeDefinition, SqlValue,
    TypeDecorator, TypeError, TypeRegistry, UuidType,
};

// New features - engine, migrations, many-to-many, async queries
pub use async_query::{AsyncQuery, AsyncSession};
pub use engine::{create_engine, create_engine_with_config, Engine, EngineConfig};
pub use many_to_many::{association_table, AssociationTable, ManyToMany};
pub use query_execution::{ExecutableQuery, QueryCompiler};

// Django ORM compatibility layer (optional)
#[cfg(feature = "django-compat")]
pub use manager::Manager;
// Query types are always available (not feature-gated)
pub use query::{Filter, FilterOperator, FilterValue, Query, QuerySet};
