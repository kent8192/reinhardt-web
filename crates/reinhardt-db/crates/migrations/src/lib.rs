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

pub mod ast_parser;
pub mod auto_migration;
pub mod autodetector;
pub mod di_support;
pub mod executor;
pub mod fields;
pub mod graph;
pub mod introspection;
pub mod migration;
pub mod migration_namer;
pub mod migration_numbering;
pub mod model_registry;
pub mod operation_trait;
pub mod operations;
pub mod plan;
pub mod recorder;
pub mod registry;
pub mod repository;
pub mod schema_diff;
pub mod service;
pub mod source;
pub mod squash;
pub mod state_loader;
pub mod visualization;
pub mod zero_downtime;

pub use autodetector::{
	// Pattern Learning and Inference
	ChangeTracker,
	ConstraintDefinition,
	DetectedChanges,
	FieldState,
	ForeignKeyAction,
	ForeignKeyConstraintInfo,
	ForeignKeyInfo,
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
	to_snake_case,
};
pub use di_support::{MigrationConfig, MigrationService as DIMigrationService};
pub use executor::{
	DatabaseMigrationExecutor, ExecutionResult, MigrationExecutor, OperationOptimizer,
};
pub use fields::FieldType;
pub use graph::{MigrationGraph, MigrationKey, MigrationNode};
pub use migration::Migration;
pub use migration_namer::MigrationNamer;
pub use migration_numbering::MigrationNumbering;
pub use model_registry::{
	FieldMetadata, ManyToManyMetadata, ModelMetadata, ModelRegistry, RelationshipMetadata,
	global_registry,
};
pub use operation_trait::MigrationOperation;
pub use operations::{
	AddColumn, AlterColumn, ColumnDefinition, Constraint, CreateTable, DropColumn, Operation,
	SqlDialect,
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
pub use repository::{MigrationRepository, filesystem::FilesystemRepository};
pub use schema_diff::{
	ColumnSchema, ConstraintSchema, DatabaseSchema, IndexSchema, SchemaDiff, SchemaDiffResult,
	TableSchema,
};
pub use service::MigrationService;
pub use source::{
	MigrationSource, composite::CompositeSource, filesystem::FilesystemSource,
	registry::RegistrySource,
};
pub use squash::{MigrationSquasher, SquashOptions};
pub use state_loader::MigrationStateLoader;
pub use visualization::{HistoryEntry, MigrationStats, MigrationVisualizer, OutputFormat};
pub use zero_downtime::{MigrationPhase, Strategy, ZeroDowntimeMigration};

pub use introspection::{
	ColumnInfo, DatabaseIntrospector, ForeignKeyInfo as IntrospectionForeignKeyInfo, IndexInfo,
	TableInfo, UniqueConstraintInfo,
};

// Re-export types from reinhardt-backends for convenience
pub use reinhardt_backends::{DatabaseConnection, DatabaseType};

use thiserror::Error;

/// Trait for types that provide migrations.
///
/// This trait enables compile-time migration collection, which is necessary
/// because Rust cannot dynamically load code at runtime like Python's Django.
///
/// # Example
///
/// ```rust,ignore
/// use reinhardt_migrations::{Migration, MigrationProvider};
///
/// pub struct PollsMigrations;
///
/// impl MigrationProvider for PollsMigrations {
///     fn migrations() -> Vec<Migration> {
///         vec![
///             super::_0001_initial::migration(),
///             super::_0002_add_published::migration(),
///         ]
///     }
/// }
///
/// // Use with test fixtures
/// let (container, db) = postgres_with_migrations_from::<PollsMigrations>().await;
/// ```
pub trait MigrationProvider {
	/// Returns all migrations provided by this type.
	///
	/// Migrations should be returned in dependency order (base migrations first).
	fn migrations() -> Vec<Migration>;
}

#[derive(Debug, Error)]
pub enum MigrationError {
	#[error("Migration not found: {0}")]
	NotFound(String),

	#[error("Dependency error: {0}")]
	DependencyError(String),

	#[error("SQL error: {0}")]
	SqlError(#[from] sqlx::Error),

	#[error("Database error: {0}")]
	DatabaseError(#[from] reinhardt_backends::QueryDatabaseError),

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

// Prelude for migrations
pub mod prelude {
	pub use crate::fields::prelude::*;
	pub use crate::{ColumnDefinition, Constraint, ForeignKeyAction, Migration, Operation};
}
