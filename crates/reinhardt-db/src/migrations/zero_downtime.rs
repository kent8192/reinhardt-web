//! Zero-downtime migration support
//!
//! This module provides strategies for performing database migrations with minimal or no downtime,
//! inspired by patterns from Braintree, GitHub, and Stripe's migration practices.
//!
//! # Example
//!
//! ```rust
//! use reinhardt_db::migrations::zero_downtime::{ZeroDowntimeMigration, Strategy};
//! use reinhardt_db::migrations::Migration;
//!
//! // Create a migration with zero-downtime strategy
//! let migration = Migration::new("0001_add_column", "myapp");
//! let zd_migration = ZeroDowntimeMigration::new(migration, Strategy::ExpandContractPattern);
//!
//! // Get the phases
//! let phases = zd_migration.get_phases().unwrap();
//! assert!(phases.len() > 0);
//! ```

use super::{Migration, Operation, Result};

/// Zero-downtime migration strategy
///
/// # Example
///
/// ```rust
/// use reinhardt_db::migrations::zero_downtime::Strategy;
///
/// let strategy = Strategy::ExpandContractPattern;
/// assert_eq!(strategy.name(), "Expand-Contract Pattern");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
	/// Expand-Contract Pattern: Add new schema, dual-write, migrate data, remove old schema
	ExpandContractPattern,
	/// Blue-Green Deployment: Run two versions simultaneously
	BlueGreenDeployment,
	/// Rolling Deployment: Gradual rollout with backward compatibility
	RollingDeployment,
	/// Shadow Mode: Test new schema alongside old one
	ShadowMode,
}

impl Strategy {
	/// Get strategy name
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::migrations::zero_downtime::Strategy;
	///
	/// let strategy = Strategy::ExpandContractPattern;
	/// assert_eq!(strategy.name(), "Expand-Contract Pattern");
	/// ```
	pub fn name(&self) -> &str {
		match self {
			Strategy::ExpandContractPattern => "Expand-Contract Pattern",
			Strategy::BlueGreenDeployment => "Blue-Green Deployment",
			Strategy::RollingDeployment => "Rolling Deployment",
			Strategy::ShadowMode => "Shadow Mode",
		}
	}

	/// Get strategy description
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::migrations::zero_downtime::Strategy;
	///
	/// let strategy = Strategy::ExpandContractPattern;
	/// let desc = strategy.description();
	/// assert!(desc.contains("Expand"));
	/// ```
	pub fn description(&self) -> &str {
		match self {
			Strategy::ExpandContractPattern => {
				"Expand schema first (add columns/tables), then contract (remove old schema)"
			}
			Strategy::BlueGreenDeployment => {
				"Deploy new version alongside old, switch traffic when ready"
			}
			Strategy::RollingDeployment => {
				"Gradually deploy to servers while maintaining backward compatibility"
			}
			Strategy::ShadowMode => "Test new schema in shadow mode before switching over",
		}
	}
}

/// Migration phase for zero-downtime deployment
///
/// # Example
///
/// ```rust
/// use reinhardt_db::migrations::zero_downtime::MigrationPhase;
/// use reinhardt_db::migrations::Migration;
///
/// let migration = Migration::new("0001_initial", "myapp");
/// let phase = MigrationPhase::new(1, "Expand", migration);
/// assert_eq!(phase.phase_number, 1);
/// ```
#[derive(Debug, Clone)]
pub struct MigrationPhase {
	/// Phase number (1-based)
	pub phase_number: usize,
	/// Phase description
	pub description: String,
	/// Migration for this phase
	pub migration: Migration,
	/// Whether this phase requires deployment
	pub requires_deployment: bool,
}

impl MigrationPhase {
	/// Create a new migration phase
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::migrations::zero_downtime::MigrationPhase;
	/// use reinhardt_db::migrations::Migration;
	///
	/// let migration = Migration::new("0001_expand", "myapp");
	/// let phase = MigrationPhase::new(1, "Expand schema", migration);
	/// ```
	pub fn new(phase_number: usize, description: impl Into<String>, migration: Migration) -> Self {
		Self {
			phase_number,
			description: description.into(),
			migration,
			requires_deployment: false,
		}
	}

	/// Set whether this phase requires code deployment
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::migrations::zero_downtime::MigrationPhase;
	/// use reinhardt_db::migrations::Migration;
	///
	/// let migration = Migration::new("0001_expand", "myapp");
	/// let phase = MigrationPhase::new(1, "Expand", migration)
	///     .requires_deployment(true);
	/// assert!(phase.requires_deployment);
	/// ```
	pub fn requires_deployment(mut self, requires: bool) -> Self {
		self.requires_deployment = requires;
		self
	}
}

/// Zero-downtime migration
///
/// Wraps a migration with a strategy for zero-downtime deployment.
///
/// # Example
///
/// ```rust
/// use reinhardt_db::migrations::zero_downtime::{ZeroDowntimeMigration, Strategy};
/// use reinhardt_db::migrations::Migration;
///
/// let migration = Migration::new("0001_add_field", "myapp");
/// let zd = ZeroDowntimeMigration::new(migration, Strategy::ExpandContractPattern);
/// ```
pub struct ZeroDowntimeMigration {
	migration: Migration,
	strategy: Strategy,
}

impl ZeroDowntimeMigration {
	/// Create a new zero-downtime migration
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::migrations::zero_downtime::{ZeroDowntimeMigration, Strategy};
	/// use reinhardt_db::migrations::Migration;
	///
	/// let migration = Migration::new("0001_rename_column", "myapp");
	/// let zd = ZeroDowntimeMigration::new(migration, Strategy::ExpandContractPattern);
	/// ```
	pub fn new(migration: Migration, strategy: Strategy) -> Self {
		Self {
			migration,
			strategy,
		}
	}

	/// Get the migration phases
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::migrations::zero_downtime::{ZeroDowntimeMigration, Strategy};
	/// use reinhardt_db::migrations::Migration;
	///
	/// let migration = Migration::new("0001_add_column", "myapp");
	/// let zd = ZeroDowntimeMigration::new(migration, Strategy::ExpandContractPattern);
	/// let phases = zd.get_phases().unwrap();
	/// ```
	pub fn get_phases(&self) -> Result<Vec<MigrationPhase>> {
		match self.strategy {
			Strategy::ExpandContractPattern => self.expand_contract_phases(),
			Strategy::BlueGreenDeployment => self.blue_green_phases(),
			Strategy::RollingDeployment => self.rolling_phases(),
			Strategy::ShadowMode => self.shadow_mode_phases(),
		}
	}

	/// Generate phases for Expand-Contract pattern
	fn expand_contract_phases(&self) -> Result<Vec<MigrationPhase>> {
		let mut phases = Vec::new();

		// Expand - Add new schema elements
		let mut expand_migration = Migration::new(
			format!("{}_expand", self.migration.name),
			self.migration.app_label.clone(),
		);
		expand_migration.operations = self.extract_expand_operations();
		expand_migration.dependencies = self.migration.dependencies.clone();

		phases.push(
			MigrationPhase::new(1, "Expand: Add new schema elements", expand_migration)
				.requires_deployment(false),
		);

		// Dual-write - Application writes to both old and new schema
		phases.push(
			MigrationPhase::new(
				2,
				"Deploy code with dual-write support",
				Migration::new(
					format!("{}_dual_write", self.migration.name),
					self.migration.app_label.clone(),
				),
			)
			.requires_deployment(true),
		);

		// Migrate data from old to new schema
		let mut migrate_migration = Migration::new(
			format!("{}_migrate_data", self.migration.name),
			self.migration.app_label.clone(),
		);
		migrate_migration.operations = self.extract_data_migration_operations();

		phases.push(
			MigrationPhase::new(3, "Migrate data to new schema", migrate_migration)
				.requires_deployment(false),
		);

		// Switch reads to new schema
		phases.push(
			MigrationPhase::new(
				4,
				"Deploy code reading from new schema",
				Migration::new(
					format!("{}_switch_reads", self.migration.name),
					self.migration.app_label.clone(),
				),
			)
			.requires_deployment(true),
		);

		// Contract - Remove old schema elements
		let mut contract_migration = Migration::new(
			format!("{}_contract", self.migration.name),
			self.migration.app_label.clone(),
		);
		contract_migration.operations = self.extract_contract_operations();

		phases.push(
			MigrationPhase::new(
				5,
				"Contract: Remove old schema elements",
				contract_migration,
			)
			.requires_deployment(false),
		);

		Ok(phases)
	}

	/// Generate phases for Blue-Green deployment
	fn blue_green_phases(&self) -> Result<Vec<MigrationPhase>> {
		let mut phases = Vec::new();

		// Setup green environment
		let mut green_migration = Migration::new(
			format!("{}_green", self.migration.name),
			self.migration.app_label.clone(),
		);
		green_migration.operations = self.migration.operations.clone();

		phases.push(
			MigrationPhase::new(
				1,
				"Setup green environment with new schema",
				green_migration,
			)
			.requires_deployment(false),
		);

		// Deploy application to green
		phases.push(
			MigrationPhase::new(
				2,
				"Deploy application to green environment",
				Migration::new(
					format!("{}_deploy_green", self.migration.name),
					self.migration.app_label.clone(),
				),
			)
			.requires_deployment(true),
		);

		// Switch traffic to green
		phases.push(
			MigrationPhase::new(
				3,
				"Switch traffic to green environment",
				Migration::new(
					format!("{}_switch", self.migration.name),
					self.migration.app_label.clone(),
				),
			)
			.requires_deployment(false),
		);

		Ok(phases)
	}

	/// Generate phases for Rolling deployment
	fn rolling_phases(&self) -> Result<Vec<MigrationPhase>> {
		let mut phases = Vec::new();

		// Deploy backward-compatible schema changes
		let mut compat_migration = Migration::new(
			format!("{}_compatible", self.migration.name),
			self.migration.app_label.clone(),
		);
		compat_migration.operations = self.migration.operations.clone();

		phases.push(
			MigrationPhase::new(1, "Deploy backward-compatible changes", compat_migration)
				.requires_deployment(false),
		);

		// Rolling application deployment
		phases.push(
			MigrationPhase::new(
				2,
				"Deploy application updates gradually",
				Migration::new(
					format!("{}_rolling", self.migration.name),
					self.migration.app_label.clone(),
				),
			)
			.requires_deployment(true),
		);

		Ok(phases)
	}

	/// Generate phases for Shadow mode
	fn shadow_mode_phases(&self) -> Result<Vec<MigrationPhase>> {
		let mut phases = Vec::new();

		// Create shadow schema
		let mut shadow_migration = Migration::new(
			format!("{}_shadow", self.migration.name),
			self.migration.app_label.clone(),
		);
		shadow_migration.operations = self.migration.operations.clone();

		phases.push(
			MigrationPhase::new(1, "Create shadow schema for testing", shadow_migration)
				.requires_deployment(false),
		);

		// Deploy with shadow writes
		phases.push(
			MigrationPhase::new(
				2,
				"Deploy code with shadow write support",
				Migration::new(
					format!("{}_shadow_writes", self.migration.name),
					self.migration.app_label.clone(),
				),
			)
			.requires_deployment(true),
		);

		// Validate shadow data
		phases.push(
			MigrationPhase::new(
				3,
				"Validate shadow data matches production",
				Migration::new(
					format!("{}_validate", self.migration.name),
					self.migration.app_label.clone(),
				),
			)
			.requires_deployment(false),
		);

		// Switch to new schema
		phases.push(
			MigrationPhase::new(
				4,
				"Switch to new schema",
				Migration::new(
					format!("{}_switch", self.migration.name),
					self.migration.app_label.clone(),
				),
			)
			.requires_deployment(true),
		);

		Ok(phases)
	}

	/// Extract operations that expand the schema (add new elements)
	fn extract_expand_operations(&self) -> Vec<Operation> {
		self.migration
			.operations
			.iter()
			.filter(|op| {
				matches!(
					op,
					Operation::CreateTable { .. } | Operation::AddColumn { .. }
				)
			})
			.cloned()
			.collect()
	}

	/// Extract operations that contract the schema (remove old elements)
	fn extract_contract_operations(&self) -> Vec<Operation> {
		self.migration
			.operations
			.iter()
			.filter(|op| {
				matches!(
					op,
					Operation::DropTable { .. } | Operation::DropColumn { .. }
				)
			})
			.cloned()
			.collect()
	}

	/// Extract data migration operations
	fn extract_data_migration_operations(&self) -> Vec<Operation> {
		self.migration
			.operations
			.iter()
			.filter(|op| matches!(op, Operation::RunSQL { .. }))
			.cloned()
			.collect()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::migrations::{ColumnDefinition, FieldType};
	use rstest::rstest;

	#[rstest]
	fn test_strategy_name() {
		assert_eq!(
			Strategy::ExpandContractPattern.name(),
			"Expand-Contract Pattern"
		);
		assert_eq!(
			Strategy::BlueGreenDeployment.name(),
			"Blue-Green Deployment"
		);
	}

	#[rstest]
	fn test_strategy_description() {
		let desc = Strategy::ExpandContractPattern.description();
		assert!(desc.contains("Expand"));
		assert!(desc.contains("contract"));
	}

	#[rstest]
	fn test_migration_phase_creation() {
		let migration = Migration::new("0001_test", "myapp");
		let phase = MigrationPhase::new(1, "Test phase", migration);

		assert_eq!(phase.phase_number, 1);
		assert_eq!(phase.description, "Test phase");
		assert!(!phase.requires_deployment);
	}

	#[rstest]
	fn test_phase_requires_deployment() {
		let migration = Migration::new("0001_test", "myapp");
		let phase = MigrationPhase::new(1, "Deploy", migration).requires_deployment(true);

		assert!(phase.requires_deployment);
	}

	#[rstest]
	fn test_zero_downtime_expand_contract() {
		let migration =
			Migration::new("0001_add_column", "myapp").add_operation(Operation::AddColumn {
				table: "users".to_string(),
				column: ColumnDefinition::new("new_field", FieldType::VarChar(100)),
				mysql_options: None,
			});

		let zd = ZeroDowntimeMigration::new(migration, Strategy::ExpandContractPattern);
		let phases = zd.get_phases().unwrap();

		assert_eq!(phases.len(), 5);
		assert_eq!(phases[0].description, "Expand: Add new schema elements");
	}

	#[rstest]
	fn test_zero_downtime_blue_green() {
		let migration = Migration::new("0001_schema_change", "myapp");
		let zd = ZeroDowntimeMigration::new(migration, Strategy::BlueGreenDeployment);
		let phases = zd.get_phases().unwrap();

		assert_eq!(phases.len(), 3);
		assert!(phases[0].description.contains("green environment"));
	}

	#[rstest]
	fn test_zero_downtime_rolling() {
		let migration = Migration::new("0001_update", "myapp");
		let zd = ZeroDowntimeMigration::new(migration, Strategy::RollingDeployment);
		let phases = zd.get_phases().unwrap();

		assert_eq!(phases.len(), 2);
		assert!(phases[0].description.contains("backward-compatible"));
	}

	#[rstest]
	fn test_zero_downtime_shadow() {
		let migration = Migration::new("0001_test_new_schema", "myapp");
		let zd = ZeroDowntimeMigration::new(migration, Strategy::ShadowMode);
		let phases = zd.get_phases().unwrap();

		assert_eq!(phases.len(), 4);
		assert!(phases[0].description.contains("shadow"));
	}

	#[rstest]
	fn test_extract_expand_operations() {
		let migration = Migration::new("0001_mixed", "myapp")
			.add_operation(Operation::CreateTable {
				name: "new_table".to_string(),
				columns: vec![],
				constraints: vec![],
				without_rowid: None,
				partition: None,
				interleave_in_parent: None,
			})
			.add_operation(Operation::AddColumn {
				table: "users".to_string(),
				column: ColumnDefinition::new("field", FieldType::VarChar(100)),
				mysql_options: None,
			})
			.add_operation(Operation::DropColumn {
				table: "users".to_string(),
				column: "old_field".to_string(),
			});

		let zd = ZeroDowntimeMigration::new(migration, Strategy::ExpandContractPattern);
		let expand_ops = zd.extract_expand_operations();

		assert_eq!(expand_ops.len(), 2);
	}

	#[rstest]
	fn test_extract_contract_operations() {
		let migration = Migration::new("0001_mixed", "myapp")
			.add_operation(Operation::AddColumn {
				table: "users".to_string(),
				column: ColumnDefinition::new("field", FieldType::VarChar(100)),
				mysql_options: None,
			})
			.add_operation(Operation::DropColumn {
				table: "users".to_string(),
				column: "old_field".to_string(),
			})
			.add_operation(Operation::DropTable {
				name: "old_table".to_string(),
			});

		let zd = ZeroDowntimeMigration::new(migration, Strategy::ExpandContractPattern);
		let contract_ops = zd.extract_contract_operations();

		assert_eq!(contract_ops.len(), 2);
	}
}
