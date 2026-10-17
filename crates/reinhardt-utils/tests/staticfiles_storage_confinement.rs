//! Regression tests confining hashed and manifest static storage to their roots.
//!
//! Each storage root is nested inside a scratch directory that also holds a
//! secret file, so an escape is observable as a read of, or a write next to,
//! that secret.

use reinhardt_utils::staticfiles::{HashedFileStorage, ManifestStaticFilesStorage};
use rstest::{fixture, rstest};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

struct Sandbox {
	outer: TempDir,
	root: PathBuf,
}

impl Sandbox {
	fn secret(&self) -> PathBuf {
		self.outer.path().join("secret.txt")
	}

	fn outer_entries(&self) -> Vec<String> {
		entries(self.outer.path())
	}

	fn root_entries(&self) -> Vec<String> {
		entries(&self.root)
	}
}

fn entries(dir: &Path) -> Vec<String> {
	let mut names: Vec<String> = std::fs::read_dir(dir)
		.unwrap()
		.map(|entry| entry.unwrap().file_name().into_string().unwrap())
		.collect();
	names.sort();
	names
}

#[fixture]
fn sandbox() -> Sandbox {
	let outer = TempDir::new().unwrap();
	let root = outer.path().join("root");
	std::fs::create_dir(&root).unwrap();
	std::fs::write(outer.path().join("secret.txt"), b"outside").unwrap();
	Sandbox { outer, root }
}

// ---------------------------------------------------------------------------
// ManifestStaticFilesStorage
// ---------------------------------------------------------------------------

#[rstest]
#[case::parent("../secret.txt")]
#[case::nested_parent("css/../../secret.txt")]
#[case::rooted_parent("/../secret.txt")]
#[tokio::test]
async fn manifest_open_rejects_unmapped_names_outside_root(sandbox: Sandbox, #[case] name: &str) {
	// Arrange
	let storage = ManifestStaticFilesStorage::new(sandbox.root.clone(), "/static/");

	// Act
	let error = storage.open(name).await.unwrap_err();

	// Assert
	assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
}

// Unix absolute paths become root-relative after the leading `/` is trimmed;
// Windows absolute paths keep their drive prefix and are rejected instead.
#[cfg(unix)]
#[rstest]
#[tokio::test]
async fn manifest_open_resolves_absolute_names_inside_root(sandbox: Sandbox) {
	// Arrange
	let storage = ManifestStaticFilesStorage::new(sandbox.root.clone(), "/static/");
	let absolute = sandbox.secret().to_str().unwrap().to_owned();

	// Act
	let error = storage.open(&absolute).await.unwrap_err();

	// Assert
	assert_eq!(error.kind(), io::ErrorKind::NotFound);
}

#[rstest]
fn manifest_exists_rejects_names_outside_root(sandbox: Sandbox) {
	// Arrange
	let storage = ManifestStaticFilesStorage::new(sandbox.root.clone(), "/static/");
	assert!(sandbox.secret().is_file());

	// Act
	let exists = storage.exists("../secret.txt");

	// Assert
	assert!(!exists);
}

#[rstest]
#[case::parent_value(r#"{"paths":{"a.css":"a.0123456789ab.css","z.css":"../secret.txt"}}"#)]
#[case::absolute_value(r#"{"paths":{"a.css":"a.0123456789ab.css","z.css":"/etc/passwd"}}"#)]
#[case::parent_key(r#"{"paths":{"a.css":"a.0123456789ab.css","../z.css":"z.css"}}"#)]
#[tokio::test]
async fn manifest_load_rejects_escaping_mappings_before_installing(
	sandbox: Sandbox,
	#[case] manifest: &str,
) {
	// Arrange
	std::fs::write(sandbox.root.join("staticfiles.json"), manifest).unwrap();
	let storage = ManifestStaticFilesStorage::new(sandbox.root.clone(), "/static/");

	// Act
	let error = storage.load_manifest().await.unwrap_err();

	// Assert
	assert_eq!(error.kind(), io::ErrorKind::InvalidData);
	assert_eq!(storage.get_hashed_path("a.css"), None);
}

#[rstest]
#[case::parent("../escape.css")]
#[case::nested_parent("css/../../escape.css")]
#[case::rooted("/escape.css")]
#[case::platform_prefix("C:/escape.css")]
#[case::backslash("css\\..\\..\\escape.css")]
#[case::empty("")]
#[tokio::test]
async fn manifest_save_rejects_escaping_names_without_writing(
	sandbox: Sandbox,
	#[case] name: &str,
) {
	// Arrange
	let storage = ManifestStaticFilesStorage::new(sandbox.root.clone(), "/static/");
	let files = HashMap::from([
		(name.to_owned(), b"body{}".to_vec()),
		("ok.css".to_owned(), b"p{}".to_vec()),
	]);

	// Act
	let error = storage.save_with_dependencies(files).await.unwrap_err();

	// Assert
	assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
	assert_eq!(sandbox.outer_entries(), ["root", "secret.txt"]);
	assert!(sandbox.root_entries().is_empty());
	assert_eq!(storage.get_hashed_path("ok.css"), None);
}

#[rstest]
#[tokio::test]
async fn manifest_storage_round_trips_names_inside_root(sandbox: Sandbox) {
	// Arrange
	let storage = ManifestStaticFilesStorage::new(sandbox.root.clone(), "/static/");
	let files = HashMap::from([("css/app.css".to_owned(), b"p{}".to_vec())]);
	storage.save_with_dependencies(files).await.unwrap();
	std::fs::write(sandbox.root.join("robots.txt"), b"ok").unwrap();
	let reloaded = ManifestStaticFilesStorage::new(sandbox.root.clone(), "/static/");
	reloaded.load_manifest().await.unwrap();

	// Act
	let mapped = reloaded.open("css/app.css").await.unwrap();
	let rooted = reloaded.open("/robots.txt").await.unwrap();

	// Assert
	assert_eq!(mapped, b"p{}");
	assert_eq!(rooted, b"ok");
	assert!(reloaded.exists("css/app.css"));
	assert_eq!(sandbox.outer_entries(), ["root", "secret.txt"]);
}

// ---------------------------------------------------------------------------
// HashedFileStorage
// ---------------------------------------------------------------------------

#[rstest]
#[case::parent("../escape.css")]
#[case::nested_parent("css/../../escape.css")]
#[case::platform_prefix("C:/escape.css")]
#[case::backslash("css\\..\\..\\escape.css")]
#[case::current_dir(".")]
#[case::empty("")]
#[tokio::test]
async fn hashed_save_rejects_names_outside_root(sandbox: Sandbox, #[case] name: &str) {
	// Arrange
	let storage = HashedFileStorage::new(sandbox.root.clone(), "/static/");

	// Act
	let error = storage.save(name, b"body{}").await.unwrap_err();

	// Assert
	assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
	assert_eq!(storage.get_hashed_path(name), None);
	assert_eq!(sandbox.outer_entries(), ["root", "secret.txt"]);
	assert!(sandbox.root_entries().is_empty());
}

#[rstest]
#[tokio::test]
async fn hashed_save_rejects_absolute_names(sandbox: Sandbox) {
	// Arrange
	let storage = HashedFileStorage::new(sandbox.root.clone(), "/static/");
	let absolute = sandbox.outer.path().join("escape.css");

	// Act
	let error = storage
		.save(absolute.to_str().unwrap(), b"body{}")
		.await
		.unwrap_err();

	// Assert
	assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
	assert_eq!(sandbox.outer_entries(), ["root", "secret.txt"]);
	assert!(sandbox.root_entries().is_empty());
}

#[rstest]
#[tokio::test]
async fn hashed_save_with_dependencies_rejects_batch_without_writing(sandbox: Sandbox) {
	// Arrange
	let storage = HashedFileStorage::new(sandbox.root.clone(), "/static/");
	let files = HashMap::from([
		("../escape.css".to_owned(), b"body{}".to_vec()),
		("ok.css".to_owned(), b"p{}".to_vec()),
	]);

	// Act
	let error = storage.save_with_dependencies(files).await.unwrap_err();

	// Assert
	assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
	assert_eq!(storage.get_hashed_path("ok.css"), None);
	assert_eq!(sandbox.outer_entries(), ["root", "secret.txt"]);
	assert!(sandbox.root_entries().is_empty());
}

#[rstest]
#[tokio::test]
async fn hashed_storage_round_trips_nested_names_inside_root(sandbox: Sandbox) {
	// Arrange
	let storage = HashedFileStorage::new(sandbox.root.clone(), "/static/");

	// Act
	let hashed_name = storage.save("css/site.css", b"body{}").await.unwrap();
	let content = storage.open("css/site.css").await.unwrap();

	// Assert
	assert_eq!(content, b"body{}");
	assert!(storage.exists("css/site.css"));
	assert!(sandbox.root.join(&hashed_name).is_file());
	assert_eq!(sandbox.outer_entries(), ["root", "secret.txt"]);
}

// ---------------------------------------------------------------------------
// Symlink escapes
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[rstest]
#[tokio::test]
async fn storages_reject_symlinks_that_leave_root(sandbox: Sandbox) {
	// Arrange
	std::os::unix::fs::symlink(sandbox.outer.path(), sandbox.root.join("link")).unwrap();
	let manifest = ManifestStaticFilesStorage::new(sandbox.root.clone(), "/static/");
	let hashed = HashedFileStorage::new(sandbox.root.clone(), "/static/");

	// Act
	let read = manifest.open("link/secret.txt").await;
	let exists = manifest.exists("link/secret.txt");
	let write = hashed.save("link/escape.css", b"body{}").await;

	// Assert
	assert_eq!(read.unwrap_err().kind(), io::ErrorKind::InvalidInput);
	assert!(!exists);
	assert_eq!(write.unwrap_err().kind(), io::ErrorKind::InvalidInput);
	assert_eq!(sandbox.outer_entries(), ["root", "secret.txt"]);
}
