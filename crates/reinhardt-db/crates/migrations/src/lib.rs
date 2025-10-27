//! # Reinhardt Migrations
//!
//! Database migration system for Reinhardt framework.

pub mod auto_migration;
pub mod autodetector;
pub mod commands;
pub mod di_support;
pub mod executor;
pub mod graph;
pub mod introspection;
pub mod loader;
pub mod migration;
pub mod model_registry;
pub mod operations;
pub mod plan;
pub mod recorder;
pub mod schema_diff;
pub mod squash;
pub mod visualization;
pub mod writer;
pub mod zero_downtime;

pub use autodetector::{
    ConstraintDefinition, DetectedChanges, FieldState, IndexDefinition, MigrationAutodetector,
    ModelState, ProjectState, SimilarityConfig,
    // Phase 2: Pattern Learning and Inference
    ChangeTracker, PatternMatcher, InferenceEngine, InferredIntent, InferenceRule, RuleCondition,
    MigrationPrompt, InteractiveAutodetector,
};
pub use commands::{MakeMigrationsCommand, MakeMigrationsOptions, MigrateCommand, MigrateOptions};
pub use di_support::{MigrationConfig, MigrationService};
pub use executor::{
    DatabaseMigrationExecutor, ExecutionResult, MigrationExecutor, OperationOptimizer,
};
pub use graph::{MigrationGraph, MigrationKey, MigrationNode};
pub use loader::MigrationLoader;
pub use migration::Migration;
pub use model_registry::{global_registry, FieldMetadata, ModelMetadata, ModelRegistry};
pub use operations::{
    AddColumn, AlterColumn, ColumnDefinition, CreateTable, DropColumn, Operation, SqlDialect,
};
pub use plan::{MigrationPlan, TransactionMode};

// New operations from refactored modules
pub use operations::{
    special::DataMigration, AddField, AlterField, CreateCollation, CreateExtension, CreateModel,
    DeleteModel, DropExtension, FieldDefinition, MoveModel, RemoveField, RenameField, RenameModel,
    RunCode, RunSQL, StateOperation,
};
pub use recorder::{DatabaseMigrationRecorder, MigrationRecord, MigrationRecorder};
pub use schema_diff::{
    ColumnSchema, ConstraintSchema, DatabaseSchema, IndexSchema, SchemaDiff, SchemaDiffResult,
    TableSchema,
};
pub use auto_migration::{
    AutoMigrationError, AutoMigrationGenerator, AutoMigrationResult, ValidationResult,
};
pub use squash::{MigrationSquasher, SquashOptions};
pub use visualization::{HistoryEntry, MigrationStats, MigrationVisualizer, OutputFormat};
pub use writer::MigrationWriter;
pub use zero_downtime::{MigrationPhase, Strategy, ZeroDowntimeMigration};

pub use introspection::{
    ColumnInfo, DatabaseIntrospector, ForeignKeyInfo, IndexInfo, TableInfo,
    UniqueConstraintInfo,
};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("Migration not found: {0}")]
    NotFound(String),

    #[error("Dependency error: {0}")]
    DependencyError(String),

    #[error("SQL error: {0}")]
    SqlError(#[from] sqlx::Error),

    #[error("Database error: {0}")]
    DatabaseError(#[from] backends::QueryDatabaseError),

    #[error("Invalid migration: {0}")]
    InvalidMigration(String),

    #[error("Irreversible migration: {0}")]
    IrreversibleError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Format error: {0}")]
    FmtError(#[from] std::fmt::Error),

    #[error("Circular dependency detected: {cycle}")]
    CircularDependency { cycle: String },

    #[error("Node not found: {message} - {node}")]
    NodeNotFound { message: String, node: String },

    #[error("Introspection error: {0}")]
    IntrospectionError(String),
}

pub type Result<T> = std::result::Result<T, MigrationError>;
