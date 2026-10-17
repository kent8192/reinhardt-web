//! Regression tests confining `StaticFileHandler` directory indexes to the
//! static root.
//!
//! Each static root is nested inside a scratch directory that also holds a
//! secret file, so an escape is observable as a directory request that
//! returns the secret's bytes.

#![cfg(unix)]

use reinhardt_utils::staticfiles::handler::{StaticError, StaticFileHandler};
use rstest::{fixture, rstest};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const SECRET: &[u8] = b"fixture outside public root";

struct Sandbox {
	outer: TempDir,
	root: PathBuf,
}

impl Sandbox {
	fn secret(&self) -> PathBuf {
		self.outer.path().join("outside.txt")
	}

	fn handler(&self) -> StaticFileHandler {
		StaticFileHandler::new(self.root.clone())
	}

	/// Creates `relative` (and its parents) inside the root as a symlink to `target`.
	fn symlink_in_root(&self, relative: &str, target: &Path) {
		let link = self.root.join(relative);
		std::fs::create_dir_all(link.parent().unwrap()).unwrap();
		symlink(target, link).unwrap();
	}

	/// Creates `relative` (and its parents) inside the root as a regular file.
	fn file_in_root(&self, relative: &str, content: &[u8]) {
		let path = self.root.join(relative);
		std::fs::create_dir_all(path.parent().unwrap()).unwrap();
		std::fs::write(path, content).unwrap();
	}
}

#[fixture]
fn sandbox() -> Sandbox {
	let outer = TempDir::new().unwrap();
	let root = outer.path().join("public");
	std::fs::create_dir(&root).unwrap();
	std::fs::write(outer.path().join("outside.txt"), SECRET).unwrap();
	Sandbox { outer, root }
}

fn assert_traversal_blocked(result: Result<impl std::fmt::Debug, StaticError>, expected: &str) {
	match result {
		Err(StaticError::DirectoryTraversal(name)) => assert_eq!(name, expected),
		other => panic!("expected DirectoryTraversal({expected:?}), got {other:?}"),
	}
}

#[rstest]
#[case::root_directory("index.html", "/", "index.html")]
#[case::root_directory_empty("index.html", "", "index.html")]
#[case::root_explicit("index.html", "/index.html", "index.html")]
#[case::nested_directory("docs/index.html", "docs/", "docs/index.html")]
#[case::nested_directory_without_slash("docs/index.html", "/docs", "docs/index.html")]
#[case::nested_explicit("docs/index.html", "docs/index.html", "docs/index.html")]
#[tokio::test]
async fn index_symlink_outside_root_is_rejected(
	sandbox: Sandbox,
	#[case] index: &str,
	#[case] request: &str,
	#[case] expected: &str,
) {
	// Arrange
	sandbox.symlink_in_root(index, &sandbox.secret());
	let handler = sandbox.handler();

	// Act
	let result = handler.serve(request).await;

	// Assert
	assert_traversal_blocked(result, expected);
}

#[rstest]
#[tokio::test]
async fn relative_index_symlink_outside_root_is_rejected(sandbox: Sandbox) {
	// Arrange
	sandbox.symlink_in_root("index.html", Path::new("../outside.txt"));
	let handler = sandbox.handler();

	// Act
	let result = handler.serve("/").await;

	// Assert
	assert_traversal_blocked(result, "index.html");
}

#[rstest]
#[tokio::test]
async fn fallback_index_symlink_outside_root_is_rejected(sandbox: Sandbox) {
	// Arrange
	sandbox.symlink_in_root("default.html", &sandbox.secret());
	let handler = sandbox
		.handler()
		.with_index_files(vec!["index.html".to_string(), "default.html".to_string()]);

	// Act
	let result = handler.serve("/").await;

	// Assert
	assert_traversal_blocked(result, "default.html");
}

#[rstest]
#[tokio::test]
async fn absolute_index_name_outside_root_is_rejected(sandbox: Sandbox) {
	// Arrange
	let secret = sandbox.secret().to_str().unwrap().to_string();
	let handler = sandbox.handler().with_index_files(vec![secret.clone()]);

	// Act
	let result = handler.serve("/").await;

	// Assert
	assert_traversal_blocked(result, &secret);
}

#[rstest]
#[case::root("index.html", "/")]
#[case::nested("docs/index.html", "docs/")]
#[tokio::test]
async fn regular_index_inside_root_is_served(
	sandbox: Sandbox,
	#[case] index: &str,
	#[case] request: &str,
) {
	// Arrange
	sandbox.file_in_root(index, b"<html>in root</html>");
	let handler = sandbox.handler();

	// Act
	let file = handler.serve(request).await.unwrap();

	// Assert
	assert_eq!(file.content, b"<html>in root</html>");
	assert_eq!(file.path, sandbox.root.canonicalize().unwrap().join(index));
	assert_eq!(file.mime_type, "text/html");
}

#[rstest]
#[tokio::test]
async fn index_symlink_inside_root_is_served_under_index_name(sandbox: Sandbox) {
	// Arrange
	sandbox.file_in_root("releases/v2.html", b"<html>release</html>");
	sandbox.symlink_in_root("index.html", Path::new("releases/v2.html"));
	let handler = sandbox.handler();

	// Act
	let file = handler.serve("/").await.unwrap();

	// Assert
	assert_eq!(file.content, b"<html>release</html>");
	assert_eq!(
		file.path,
		sandbox.root.canonicalize().unwrap().join("index.html")
	);
}

#[rstest]
#[tokio::test]
async fn dangling_index_symlink_is_not_found(sandbox: Sandbox) {
	// Arrange
	sandbox.symlink_in_root("index.html", Path::new("missing.html"));
	let handler = sandbox.handler();

	// Act
	let result = handler.serve("/").await;

	// Assert
	assert!(
		matches!(result, Err(StaticError::NotFound(_))),
		"got {result:?}"
	);
}
