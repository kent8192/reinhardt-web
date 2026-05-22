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

use std::collections::HashSet;
use std::io::Write as _;
use std::sync::Mutex;
use std::sync::OnceLock;

use crate::staticfiles::vendor::asset::AppVendorAsset;

/// Verbosity for download progress output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verbosity {
	/// Suppress all output.
	Silent,
	/// Print download URLs and errors (default).
	Normal,
	/// Print every file including skipped ones.
	Verbose,
}

/// Whether vendor asset download failures should abort the caller.
///
/// Controlled by env var `REINHARDT_VENDOR_ASSETS_REQUIRED=1`. Defaults to soft-fail
/// (matching the existing admin behavior) so that local development and CI are not
/// blocked by transient CDN issues.
fn fail_hard() -> bool {
	std::env::var("REINHARDT_VENDOR_ASSETS_REQUIRED")
		.map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
		.unwrap_or(false)
}

/// Download the given assets into `base_dir`, skipping files that already pass
/// integrity checks and writing atomically via a sibling temp file.
///
/// `base_dir` is the app's static root (e.g., `crates/reinhardt-admin/assets`);
/// `target` paths are joined onto it.
pub async fn download_assets(
	base_dir: &Path,
	assets: &[AppVendorAsset],
	verbosity: Verbosity,
) -> Result<(), VendorDownloadError> {
	let client = reqwest::Client::builder()
		.user_agent(concat!("reinhardt-vendor/", env!("CARGO_PKG_VERSION")))
		.build()
		.map_err(|e| VendorDownloadError::Io {
			path: "<reqwest client>".to_string(),
			source: std::io::Error::other(e.to_string()),
		})?;

	for asset in assets {
		asset.validate().map_err(|e| VendorDownloadError::Io {
			path: asset.target.to_string(),
			source: std::io::Error::other(e.to_string()),
		})?;

		let dest = base_dir.join(asset.target);

		// Skip if already present and integrity passes.
		if dest.exists() && verify_integrity(&dest, asset.sha256).is_ok() {
			if verbosity == Verbosity::Verbose {
				println!("skip (exists): {}", asset.target);
			}
			continue;
		}

		if verbosity != Verbosity::Silent {
			println!("download: {}", asset.url);
		}

		let response = client
			.get(asset.url)
			.send()
			.await
			.map_err(|e| VendorDownloadError::Io {
				path: asset.url.to_string(),
				source: std::io::Error::other(e.to_string()),
			})?;
		if !response.status().is_success() {
			return Err(VendorDownloadError::Io {
				path: asset.url.to_string(),
				source: std::io::Error::other(format!("HTTP {}", response.status())),
			});
		}
		let bytes = response
			.bytes()
			.await
			.map_err(|e| VendorDownloadError::Io {
				path: asset.url.to_string(),
				source: std::io::Error::other(e.to_string()),
			})?;

		if let Some(parent) = dest.parent() {
			std::fs::create_dir_all(parent).map_err(|source| VendorDownloadError::Io {
				path: parent.display().to_string(),
				source,
			})?;
		}

		let parent_dir = dest.parent().ok_or_else(|| VendorDownloadError::Io {
			path: dest.display().to_string(),
			source: std::io::Error::other("destination has no parent directory"),
		})?;
		let mut tmp = tempfile::NamedTempFile::new_in(parent_dir).map_err(|source| {
			VendorDownloadError::Io {
				path: parent_dir.display().to_string(),
				source,
			}
		})?;
		tmp.write_all(&bytes)
			.map_err(|source| VendorDownloadError::Io {
				path: tmp.path().display().to_string(),
				source,
			})?;
		tmp.persist(&dest).map_err(|e| VendorDownloadError::Io {
			path: dest.display().to_string(),
			source: e.error,
		})?;

		// Verify post-download. Empty SHA is short-circuited inside verify_integrity.
		match verify_integrity(&dest, asset.sha256) {
			Ok(()) => {
				if asset.sha256.is_empty() {
					// Compute and log the SHA so the developer can pin it.
					if let Ok(content) = std::fs::read(&dest) {
						let mut h = Sha256::new();
						h.update(&content);
						let computed = format!("{:x}", h.finalize());
						if verbosity != Verbosity::Silent {
							println!(
								"vendor asset {} downloaded; computed sha256 = {}",
								asset.target, computed
							);
						}
					}
				} else if verbosity == Verbosity::Verbose {
					println!("verified: {}", asset.target);
				}
			}
			Err(e) => return Err(e),
		}
	}

	Ok(())
}

/// Map of `(app_label, base_dir)` pairs already ensured this process.
fn ensured_set() -> &'static Mutex<HashSet<(String, std::path::PathBuf)>> {
	static SET: OnceLock<Mutex<HashSet<(String, std::path::PathBuf)>>> = OnceLock::new();
	SET.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Ensure all vendor assets registered for `app_label` exist on disk under `base_dir`.
///
/// Idempotent per `(app_label, base_dir)` pair within a process. On error, logs
/// via `tracing::error!` and returns; with `REINHARDT_VENDOR_ASSETS_REQUIRED=1`,
/// returns the underlying `VendorDownloadError`.
pub async fn ensure_vendor_assets_for_app(
	app_label: &str,
	base_dir: &Path,
) -> Result<(), VendorDownloadError> {
	let key = (app_label.to_string(), base_dir.to_path_buf());
	{
		let guard = ensured_set().lock().expect("ensured_set mutex poisoned");
		if guard.contains(&key) {
			return Ok(());
		}
	}

	let assets: Vec<AppVendorAsset> = inventory::iter::<AppVendorAsset>()
		.copied()
		.filter(|a| a.app_label == app_label)
		.collect();

	let result = download_assets(base_dir, &assets, Verbosity::Normal).await;

	match result {
		Ok(()) => {
			ensured_set()
				.lock()
				.expect("ensured_set mutex poisoned")
				.insert(key);
			Ok(())
		}
		Err(e) => {
			tracing::error!("vendor asset download for {} failed: {}", app_label, e);
			if fail_hard() {
				Err(e)
			} else {
				// Soft-fail: still mark as ensured so we do not retry every request.
				ensured_set()
					.lock()
					.expect("ensured_set mutex poisoned")
					.insert(key);
				Ok(())
			}
		}
	}
}

/// Download every vendor asset across all registered apps (used by `collectstatic`).
///
/// `resolve_base_dir` is given the app_label and returns the on-disk directory
/// for that app; it returns `None` if the app has no static dir registered.
pub async fn download_all_vendor_assets<F>(
	mut resolve_base_dir: F,
	verbosity: Verbosity,
) -> Result<(), VendorDownloadError>
where
	F: FnMut(&str) -> Option<std::path::PathBuf>,
{
	use std::collections::BTreeMap;

	// Group assets by app_label so each base_dir lookup happens once.
	let mut grouped: BTreeMap<&'static str, Vec<AppVendorAsset>> = BTreeMap::new();
	for asset in inventory::iter::<AppVendorAsset>() {
		grouped.entry(asset.app_label).or_default().push(*asset);
	}

	for (label, assets) in grouped {
		let Some(base_dir) = resolve_base_dir(label) else {
			if verbosity != Verbosity::Silent {
				println!("skip vendor for {}: no base dir registered", label);
			}
			continue;
		};
		download_assets(&base_dir, &assets, verbosity).await?;
	}
	Ok(())
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

	#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
	async fn ensure_for_app_is_idempotent_for_unknown_app() {
		// Arrange
		let tmp = tempfile::tempdir().expect("tempdir");
		let label = "__test_app_no_assets_registered__";

		// Act
		let r1 = ensure_vendor_assets_for_app(label, tmp.path()).await;
		let r2 = ensure_vendor_assets_for_app(label, tmp.path()).await;

		// Assert
		assert!(r1.is_ok(), "first call: {:?}", r1);
		assert!(r2.is_ok(), "second call: {:?}", r2);
	}
}
