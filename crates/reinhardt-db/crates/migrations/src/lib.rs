//! # Reinhardt Migrations
//!
//! Database migration system for Reinhardt framework.
//!
//! ## Planned Features
//! TODO: Implement migration squashing to combine multiple migrations
//! TODO: Add built-in support for complex data migrations
//! TODO: Implement zero-downtime migration support
//! TODO: Add automatic operation reordering and optimization
//! TODO: Enhance atomic operations with better transaction handling
//! TODO: Add schema history visualization tools

pub mod autodetector;
pub mod commands;
pub mod di_support;
pub mod executor;
pub mod graph;
pub mod loader;
pub mod migration;
pub mod model_registry;
pub mod operations;
pub mod plan;
pub mod recorder;
pub mod writer;

pub use autodetector::{
    ConstraintDefinition, DetectedChanges, FieldState, IndexDefinition, MigrationAutodetector,
    ModelState, ProjectState,
};
pub use commands::{MakeMigrationsCommand, MakeMigrationsOptions, MigrateCommand, MigrateOptions};
pub use di_support::{MigrationConfig, MigrationService};
pub use executor::{ExecutionResult, MigrationExecutor};
pub use graph::{MigrationGraph, MigrationKey, MigrationNode};
pub use loader::MigrationLoader;
pub use migration::Migration;
pub use model_registry::{global_registry, FieldMetadata, ModelMetadata, ModelRegistry};
pub use plan::MigrationPlan;
pub use operations::{
    AddColumn, AlterColumn, ColumnDefinition, CreateTable, DropColumn, Operation, SqlDialect,
};

// New operations from refactored modules
pub use operations::{
    AddField, AlterField, CreateCollation, CreateExtension, CreateModel, DeleteModel,
    DropExtension, FieldDefinition, RemoveField, RenameField, RenameModel, RunCode, RunSQL,
    StateOperation,
};
pub use recorder::{DatabaseMigrationRecorder, MigrationRecord, MigrationRecorder};
pub use writer::MigrationWriter;

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
}

pub type Result<T> = std::result::Result<T, MigrationError>;
