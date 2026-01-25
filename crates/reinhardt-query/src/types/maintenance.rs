//! Database maintenance types
//!
//! This module provides types for database maintenance operations:
//!
//! - [`VacuumOption`]: Options for VACUUM statement

use crate::types::{DynIden, IntoIden};

/// VACUUM statement options
///
/// This struct represents options for the VACUUM statement.
///
/// # Examples
///
/// ```rust
/// use reinhardt_query::types::maintenance::VacuumOption;
///
/// // Basic VACUUM
/// let opt = VacuumOption::new();
///
/// // VACUUM FULL
/// let opt = VacuumOption::new().full(true);
///
/// // VACUUM FULL ANALYZE
/// let opt = VacuumOption::new().full(true).analyze(true);
/// ```
#[derive(Debug, Clone, Default)]
pub struct VacuumOption {
	pub(crate) full: bool,
	pub(crate) freeze: bool,
	pub(crate) verbose: bool,
	pub(crate) analyze: bool,
}

impl VacuumOption {
	/// Create a new VACUUM option
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_query::types::maintenance::VacuumOption;
	///
	/// let opt = VacuumOption::new();
	/// ```
	pub fn new() -> Self {
		Self::default()
	}

	/// Set FULL option
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_query::types::maintenance::VacuumOption;
	///
	/// let opt = VacuumOption::new().full(true);
	/// ```
	pub fn full(mut self, full: bool) -> Self {
		self.full = full;
		self
	}

	/// Set FREEZE option
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_query::types::maintenance::VacuumOption;
	///
	/// let opt = VacuumOption::new().freeze(true);
	/// ```
	pub fn freeze(mut self, freeze: bool) -> Self {
		self.freeze = freeze;
		self
	}

	/// Set VERBOSE option
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_query::types::maintenance::VacuumOption;
	///
	/// let opt = VacuumOption::new().verbose(true);
	/// ```
	pub fn verbose(mut self, verbose: bool) -> Self {
		self.verbose = verbose;
		self
	}

	/// Set ANALYZE option
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_query::types::maintenance::VacuumOption;
	///
	/// let opt = VacuumOption::new().analyze(true);
	/// ```
	pub fn analyze(mut self, analyze: bool) -> Self {
		self.analyze = analyze;
		self
	}
}

/// Table specification for ANALYZE statement
///
/// This struct represents a table and its optional columns for ANALYZE.
///
/// # Examples
///
/// ```rust
/// use reinhardt_query::types::maintenance::AnalyzeTable;
///
/// // Analyze entire table
/// let tbl = AnalyzeTable::new("users");
///
/// // Analyze specific columns
/// let tbl = AnalyzeTable::new("users")
///     .add_column("email")
///     .add_column("name");
/// ```
#[derive(Debug, Clone)]
pub struct AnalyzeTable {
	// Allow dead_code: table field will be used when Phase B ANALYZE implementation is completed
	#[allow(dead_code)]
	pub(crate) table: DynIden,
	pub(crate) columns: Vec<DynIden>,
}

impl AnalyzeTable {
	/// Create a new ANALYZE table specification
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_query::types::maintenance::AnalyzeTable;
	///
	/// let tbl = AnalyzeTable::new("users");
	/// ```
	pub fn new<T: IntoIden>(table: T) -> Self {
		Self {
			table: table.into_iden(),
			columns: Vec::new(),
		}
	}

	/// Add a column to analyze
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_query::types::maintenance::AnalyzeTable;
	///
	/// let tbl = AnalyzeTable::new("users")
	///     .add_column("email");
	/// ```
	pub fn add_column<C: IntoIden>(mut self, column: C) -> Self {
		self.columns.push(column.into_iden());
		self
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::*;

	// VacuumOption tests
	#[rstest]
	fn test_vacuum_option_default() {
		let opt = VacuumOption::new();
		assert!(!opt.full);
		assert!(!opt.freeze);
		assert!(!opt.verbose);
		assert!(!opt.analyze);
	}

	#[rstest]
	fn test_vacuum_option_full() {
		let opt = VacuumOption::new().full(true);
		assert!(opt.full);
		assert!(!opt.freeze);
		assert!(!opt.verbose);
		assert!(!opt.analyze);
	}

	#[rstest]
	fn test_vacuum_option_freeze() {
		let opt = VacuumOption::new().freeze(true);
		assert!(!opt.full);
		assert!(opt.freeze);
		assert!(!opt.verbose);
		assert!(!opt.analyze);
	}

	#[rstest]
	fn test_vacuum_option_verbose() {
		let opt = VacuumOption::new().verbose(true);
		assert!(!opt.full);
		assert!(!opt.freeze);
		assert!(opt.verbose);
		assert!(!opt.analyze);
	}

	#[rstest]
	fn test_vacuum_option_analyze() {
		let opt = VacuumOption::new().analyze(true);
		assert!(!opt.full);
		assert!(!opt.freeze);
		assert!(!opt.verbose);
		assert!(opt.analyze);
	}

	#[rstest]
	fn test_vacuum_option_combined() {
		let opt = VacuumOption::new()
			.full(true)
			.freeze(true)
			.verbose(true)
			.analyze(true);
		assert!(opt.full);
		assert!(opt.freeze);
		assert!(opt.verbose);
		assert!(opt.analyze);
	}

	// AnalyzeTable tests
	#[rstest]
	fn test_analyze_table_basic() {
		let tbl = AnalyzeTable::new("users");
		assert_eq!(tbl.table.to_string(), "users");
		assert!(tbl.columns.is_empty());
	}

	#[rstest]
	fn test_analyze_table_with_column() {
		let tbl = AnalyzeTable::new("users").add_column("email");
		assert_eq!(tbl.table.to_string(), "users");
		assert_eq!(tbl.columns.len(), 1);
		assert_eq!(tbl.columns[0].to_string(), "email");
	}

	#[rstest]
	fn test_analyze_table_with_multiple_columns() {
		let tbl = AnalyzeTable::new("users")
			.add_column("email")
			.add_column("name")
			.add_column("age");
		assert_eq!(tbl.table.to_string(), "users");
		assert_eq!(tbl.columns.len(), 3);
		assert_eq!(tbl.columns[0].to_string(), "email");
		assert_eq!(tbl.columns[1].to_string(), "name");
		assert_eq!(tbl.columns[2].to_string(), "age");
	}
}
