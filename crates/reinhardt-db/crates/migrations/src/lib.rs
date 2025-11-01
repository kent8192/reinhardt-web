//! # Reinhardt Migrations
//!
//! Database migration system for Reinhardt framework.
//!
//! ## Features
//!
//! - **Auto-detection**: Detects model changes and generates migrations
//! - **Migration Graph**: Manages dependencies between migrations
//! - **AST-Based Entry Points**: Generates Rust 2024 Edition-compliant module files
//! - **Zero Downtime**: Support for safe schema changes in production
//!
//! ## AST-Based Entry Point Generation
//!
//! The `makemigrations` command uses Abstract Syntax Tree (AST) parsing to generate
//! and maintain migration entry point files (`migrations/app_name.rs`). This ensures:
//!
//! 1. **Rust 2024 Edition Compliance**: Uses `app_name.rs` instead of deprecated `mod.rs`
//! 2. **Robust Module Detection**: Structurally identifies existing migration modules
//! 3. **Consistent Formatting**: Standardized output via `prettyplease`
//!
//! ### Generated Entry Point Example
//!
//! ```rust,ignore
//! // migrations/myapp.rs (auto-generated)
//! pub mod _0001_initial;
//! pub mod _0002_add_field;
//!
//! pub fn all_migrations() -> Vec<fn() -> Migration> {
//!     vec![_0001_initial::migration, _0002_add_field::migration]
//! }
//! ```
//!
//! This file is automatically updated when new migrations are created.

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
	// Phase 2: Pattern Learning and Inference
	ChangeTracker,
	ConstraintDefinition,
	DetectedChanges,
	FieldState,
	IndexDefinition,
	InferenceEngine,
	InferenceRule,
	InferredIntent,
	InteractiveAutodetector,
	MigrationAutodetector,
	MigrationPrompt,
	ModelState,
	PatternMatcher,
	ProjectState,
	RuleCondition,
	SimilarityConfig,
};
pub use commands::{MakeMigrationsCommand, MakeMigrationsOptions, MigrateCommand, MigrateOptions};
pub use di_support::{MigrationConfig, MigrationService};
pub use executor::{
	DatabaseMigrationExecutor, ExecutionResult, MigrationExecutor, OperationOptimizer,
};
pub use graph::{MigrationGraph, MigrationKey, MigrationNode};
pub use loader::MigrationLoader;
pub use migration::Migration;
pub use model_registry::{FieldMetadata, ModelMetadata, ModelRegistry, global_registry};
pub use operations::{
	AddColumn, AlterColumn, ColumnDefinition, CreateTable, DropColumn, Operation, SqlDialect,
};
pub use plan::{MigrationPlan, TransactionMode};

// New operations from refactored modules
pub use auto_migration::{
	AutoMigrationError, AutoMigrationGenerator, AutoMigrationResult, ValidationResult,
};
pub use operations::{
	AddField, AlterField, CreateCollation, CreateExtension, CreateModel, DeleteModel,
	DropExtension, FieldDefinition, MoveModel, RemoveField, RenameField, RenameModel, RunCode,
	RunSQL, StateOperation, special::DataMigration,
};
pub use recorder::{DatabaseMigrationRecorder, MigrationRecord, MigrationRecorder};
pub use schema_diff::{
	ColumnSchema, ConstraintSchema, DatabaseSchema, IndexSchema, SchemaDiff, SchemaDiffResult,
	TableSchema,
};
pub use squash::{MigrationSquasher, SquashOptions};
pub use visualization::{HistoryEntry, MigrationStats, MigrationVisualizer, OutputFormat};
pub use writer::MigrationWriter;
pub use zero_downtime::{MigrationPhase, Strategy, ZeroDowntimeMigration};

pub use introspection::{
	ColumnInfo, DatabaseIntrospector, ForeignKeyInfo, IndexInfo, TableInfo, UniqueConstraintInfo,
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

	#[error("Unsupported database: {0}")]
	UnsupportedDatabase(String),
}

pub type Result<T> = std::result::Result<T, MigrationError>;
