//! Error classification tests for the Manouche v2 migration codemod.

use std::path::PathBuf;

use reinhardt_admin_cli::migrate_v2::{
	MigrateV2Args, error::MigrateV2Error, walker::find_rs_files,
};
use tempfile::tempdir;

#[test]
fn run_sorts_and_reports_unknown_skip_rules() {
	// Arrange
	let args = MigrateV2Args {
		path: PathBuf::from("."),
		dry_run: true,
		skip: vec!["zeta".to_owned(), "alpha".to_owned()],
	};

	// Act
	let error = reinhardt_admin_cli::migrate_v2::run(args).unwrap_err();

	// Assert
	match error {
		MigrateV2Error::UnknownSkipRules(names) => assert_eq!(names, "alpha, zeta"),
		other => panic!("expected unknown skip rules, got {other:?}"),
	}
}

#[test]
fn walker_classifies_a_missing_root_as_walk_error() {
	// Arrange
	let directory = tempdir().unwrap();
	let missing_root = directory.path().join("missing");

	// Act
	let error = find_rs_files(&missing_root).unwrap_err();

	// Assert
	assert!(matches!(error, MigrateV2Error::Walk(_)));
}
