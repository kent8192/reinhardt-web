//! Vendor asset downloader: fetch + SHA-256 verification + atomic write.

use std::path::Path;

use sha2::{Digest, Sha256};
use thiserror::Error;

/// Errors from vendor asset operations.
#[derive(Debug, Error)]
pub enum VendorDownloadError {
	/// SHA-256 digest of the downloaded file did not match the declared digest.
	#[error("integrity check failed for {path}: expected {expected}, got {actual}")]
	IntegrityMismatch {
		/// Path of the file whose digest was checked.
		path: String,
		/// Declared (expected) hex digest.
		expected: String,
		/// Computed hex digest of the file that was actually found on disk.
		actual: String,
	},
	/// An I/O error occurred while reading the file.
	#[error("io error for {path}: {source}")]
	Io {
		/// Path of the file that triggered the I/O error.
		path: String,
		/// Underlying I/O error.
		#[source]
		source: std::io::Error,
	},
}

/// Verify a file on disk matches `expected_sha256` (hex). An empty `expected_sha256`
/// short-circuits as success (used in unverified-bootstrap mode).
pub fn verify_integrity(path: &Path, expected_sha256: &str) -> Result<(), VendorDownloadError> {
	if expected_sha256.is_empty() {
		return Ok(());
	}
	let bytes = std::fs::read(path).map_err(|source| VendorDownloadError::Io {
		path: path.display().to_string(),
		source,
	})?;
	let mut hasher = Sha256::new();
	hasher.update(&bytes);
	let actual = format!("{:x}", hasher.finalize());
	if actual.eq_ignore_ascii_case(expected_sha256) {
		Ok(())
	} else {
		Err(VendorDownloadError::IntegrityMismatch {
			path: path.display().to_string(),
			expected: expected_sha256.to_string(),
			actual,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;
	use std::io::Write;

	fn write_temp(bytes: &[u8]) -> tempfile::NamedTempFile {
		let mut f = tempfile::NamedTempFile::new().expect("temp file");
		f.write_all(bytes).expect("write");
		f.flush().expect("flush");
		f
	}

	#[rstest]
	fn verify_integrity_passes_when_sha_matches() {
		// Arrange
		// SHA-256 of "hello" = 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
		let f = write_temp(b"hello");
		let expected = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

		// Act
		let result = verify_integrity(f.path(), expected);

		// Assert
		assert!(result.is_ok(), "expected pass, got {:?}", result);
	}

	#[rstest]
	fn verify_integrity_fails_when_sha_differs() {
		// Arrange
		let f = write_temp(b"hello");
		let wrong = "0000000000000000000000000000000000000000000000000000000000000000";

		// Act
		let err = verify_integrity(f.path(), wrong).expect_err("must fail");

		// Assert
		match err {
			VendorDownloadError::IntegrityMismatch {
				expected, actual, ..
			} => {
				assert_eq!(expected, wrong);
				assert_eq!(
					actual,
					"2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
				);
			}
			other => panic!("expected IntegrityMismatch, got {:?}", other),
		}
	}

	#[rstest]
	fn verify_integrity_skips_when_expected_empty() {
		// Arrange
		let f = write_temp(b"any contents");

		// Act
		let result = verify_integrity(f.path(), "");

		// Assert
		assert!(
			result.is_ok(),
			"empty expected SHA must short-circuit as Ok"
		);
	}

	#[rstest]
	fn verify_integrity_errors_on_missing_file() {
		// Arrange
		let path = std::path::PathBuf::from("/nonexistent/path/that/does/not/exist.bin");

		// Act
		let err = verify_integrity(&path, "abc").expect_err("missing file must error");

		// Assert
		assert!(
			matches!(err, VendorDownloadError::Io { .. }),
			"got {:?}",
			err
		);
	}
}
