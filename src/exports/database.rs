//! Database, ORM, and query builder re-exports.

pub use reinhardt_db::orm::{
    DatabaseBackend, DatabaseConnection, Model, QuerySet, SoftDeletable, SoftDelete, Timestamped,
    Timestamps,
};

// Query expressions (Django-style F/Q objects)
//
// # Examples
//
// ```rust,no_run
// # use reinhardt::{F, Q};
// let price_expr = F::field("price");
// let filter = Q::and(vec![
//     Q::field("status").equals("active"),
//     Q::field("price").gt(100),
// ]);
// ```
pub use reinhardt_db::orm::{
    Exists, F, FieldRef, Filter, FilterOperator, FilterValue, OuterRef, Q, QOperator, Subquery,
};

// Annotations and aggregations
pub use reinhardt_db::orm::{
    Aggregate, AggregateFunc, AggregateValue, Annotation, AnnotationValue,
};

// Transaction management
pub use reinhardt_db::orm::{
    IsolationLevel, QueryValue, Savepoint, Transaction, TransactionExecutor, TransactionScope,
    atomic, atomic_with_isolation,
};

// Database functions
pub use reinhardt_db::orm::{
    Abs, Cast, Ceil, Concat, CurrentDate, CurrentTime, Extract, ExtractComponent, Floor, Greatest,
    Least, Length, Lower, Mod, Now, NullIf, Power, Round, SqlType, Sqrt, Substr, Trim, TrimType,
    Upper,
};

// Window functions
pub use reinhardt_db::orm::{
    DenseRank, FirstValue, Frame, FrameBoundary, FrameType, Lag, LastValue, Lead, NTile, NthValue,
    Rank, RowNumber, Window, WindowFunction,
};

// Constraints and indexes
pub use reinhardt_db::orm::{
    BTreeIndex, CheckConstraint, Constraint, ForeignKeyConstraint, GinIndex, GistIndex, HashIndex,
    Index, OnDelete, OnUpdate, UniqueConstraint,
};

// Query builder types from reinhardt-query (via reinhardt-db)
pub use reinhardt_db::orm::{IntoValue, Order, QueryBuilderValue};

// Connection pool
pub use reinhardt_db::pool::{ConnectionPool, PoolConfig, PoolError};

// Content types
pub use reinhardt_db::contenttypes::{
    CONTENT_TYPE_REGISTRY, ContentType, ContentTypeRegistry, GenericForeignKey, GenericRelatable,
    GenericRelationQuery, ModelType,
};

// Migrations
pub use reinhardt_db::migrations::{
    FieldState, Migration, MigrationAutodetector, MigrationError, MigrationPlan, MigrationRecorder,
    ModelState, ProjectState,
};
