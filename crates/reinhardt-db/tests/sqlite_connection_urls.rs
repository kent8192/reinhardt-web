#![cfg(all(unix, feature = "backends", feature = "sqlite"))]

use std::path::PathBuf;

use reinhardt_db::backends::DatabaseConnection;
use reinhardt_query::prelude::{
	ColumnDef, ColumnType, Expr, ExprTrait, Iden, IntoIden, Query, QueryStatementBuilder,
	SqliteQueryBuilder,
};
use rstest::{fixture, rstest};
use serial_test::serial;
use tempfile::TempDir;

#[derive(Debug, Iden)]
enum Probe {
	Table,
	Id,
}

#[derive(Debug, Iden)]
enum PragmaDatabaseList {
	Table,
	Name,
	File,
}

struct WorkingDirectory {
	original: PathBuf,
	directory: TempDir,
}

impl WorkingDirectory {
	fn enter(directory: TempDir) -> Self {
		let original = std::env::current_dir().unwrap();
		std::env::set_current_dir(directory.path()).unwrap();
		// Both the intended database and any incorrectly resolved relative file
		// remain inside this guard's directory, including when an assertion panics.
		Self {
			original,
			directory,
		}
	}
}

impl Drop for WorkingDirectory {
	fn drop(&mut self) {
		std::env::set_current_dir(&self.original).expect("restore the original working directory");
	}
}

#[fixture]
fn temporary_directory() -> TempDir {
	tempfile::tempdir_in("/tmp").unwrap()
}

async fn create_probe_table(connection: &DatabaseConnection) {
	let sql = Query::create_table()
		.table(Probe::Table.into_iden())
		.col(ColumnDef::new(Probe::Id).column_type(ColumnType::Integer))
		.to_string(SqliteQueryBuilder);
	connection.execute(&sql, vec![]).await.unwrap();
}

async fn database_filename(connection: &DatabaseConnection) -> String {
	// SQLite exposes PRAGMA database_list through this table-valued interface.
	let sql = Query::select()
		.columns([PragmaDatabaseList::Name, PragmaDatabaseList::File])
		.from(PragmaDatabaseList::Table.into_iden())
		.and_where(Expr::col(PragmaDatabaseList::Name).eq("main"))
		.to_string(SqliteQueryBuilder);
	let rows = connection.fetch_all(&sql, vec![]).await.unwrap();
	assert_eq!(rows.len(), 1);
	assert_eq!(rows[0].get::<String>("name").unwrap(), "main");
	rows[0].get("file").unwrap()
}

#[rstest]
#[case::absolute_url("sqlite://", true)]
#[case::legacy_absolute_url("sqlite:///", true)]
#[case::short_absolute_url("sqlite:", true)]
#[case::absolute_path("", true)]
#[case::relative_url("sqlite://", false)]
#[case::short_relative_url("sqlite:", false)]
#[case::relative_path("", false)]
#[tokio::test]
#[serial(sqlite_connection_urls)]
async fn sqlite_file_urls_open_intended_database(
	temporary_directory: TempDir,
	#[case] prefix: &str,
	#[case] absolute: bool,
	#[values(false, true)] existing_file: bool,
) {
	// Arrange
	// Enter after the serial guard is acquired, rather than inside the fixture.
	let working_directory = WorkingDirectory::enter(temporary_directory);
	let relative_path = PathBuf::from("nested/intended.sqlite");
	let intended_path = working_directory
		.directory
		.path()
		.canonicalize()
		.unwrap()
		.join(&relative_path);
	if existing_file {
		std::fs::create_dir_all(intended_path.parent().unwrap()).unwrap();
		std::fs::write(&intended_path, []).unwrap();
		assert_eq!(std::fs::metadata(&intended_path).unwrap().len(), 0);
	}
	let url_path = if absolute {
		&intended_path
	} else {
		&relative_path
	};
	let url = format!("{prefix}{}", url_path.display());

	// Act
	let connection = DatabaseConnection::connect_sqlite(&url).await.unwrap();
	create_probe_table(&connection).await;
	let actual_filename = database_filename(&connection).await;

	// Assert: the opened database and its writes belong to the intended path.
	assert_eq!(PathBuf::from(actual_filename), intended_path);
	let contents = std::fs::read(&intended_path).unwrap();
	assert_eq!(contents.get(..16), Some(b"SQLite format 3\0".as_slice()));
	connection.into_sqlite().unwrap().close().await;
}

#[rstest]
#[tokio::test]
async fn sqlite_memory_url_keeps_database_in_memory() {
	// Arrange
	let connection = DatabaseConnection::connect_sqlite("sqlite::memory:")
		.await
		.unwrap();

	// Act
	create_probe_table(&connection).await;
	let actual_filename = database_filename(&connection).await;

	// Assert
	assert_eq!(actual_filename, "");
	connection.into_sqlite().unwrap().close().await;
}
