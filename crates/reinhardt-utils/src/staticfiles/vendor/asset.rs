//! Vendor asset descriptor and path validation.
//!
//! `AppVendorAsset` describes one external file (JS / CSS / font) that an
//! application wants downloaded into its `vendor/` subdirectory. Entries are
//! registered cross-crate via `inventory::submit!`, typically through the
//! `#[app_config(... vendor_assets(asset(url = ..., target = ...)))]`
//! attribute macro.

use std::fmt;

/// A single vendor asset declared by an application.
///
/// All fields are `&'static str` because entries are submitted via `inventory`,
/// which requires a `'static` lifetime.
#[derive(Debug, Clone, Copy)]
pub struct AppVendorAsset {
	/// The app label this asset belongs to (matches `register_app_static_files!`).
	pub app_label: &'static str,
	/// Version-pinned source URL (typically a CDN).
	pub url: &'static str,
	/// Path relative to the app's static directory. MUST start with `vendor/`.
	pub target: &'static str,
	/// Expected SHA-256 hex digest. Empty string = unverified; SHA will be
	/// logged after first successful download so the developer can pin it.
	pub sha256: &'static str,
}

inventory::collect!(AppVendorAsset);

/// Errors produced when validating a vendor asset declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VendorAssetError {
	/// `target` did not begin with `vendor/`.
	BadTargetPrefix(&'static str),
	/// `target` contained a `..` path segment.
	PathTraversal(&'static str),
	/// `target` contained a null byte.
	NullByte(&'static str),
}

impl fmt::Display for VendorAssetError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::BadTargetPrefix(t) => {
				write!(f, "vendor asset target {:?} must start with \"vendor/\"", t)
			}
			Self::PathTraversal(t) => {
				write!(
					f,
					"vendor asset target {:?} must not contain \"..\" segments",
					t
				)
			}
			Self::NullByte(t) => {
				write!(f, "vendor asset target {:?} must not contain null byte", t)
			}
		}
	}
}

impl std::error::Error for VendorAssetError {}

impl AppVendorAsset {
	/// Validate that `target` is safe to use as a relative path.
	///
	/// Rejects: missing `vendor/` prefix, `..` segments, null bytes.
	pub fn validate(&self) -> Result<(), VendorAssetError> {
		if !self.target.starts_with("vendor/") {
			return Err(VendorAssetError::BadTargetPrefix(self.target));
		}
		if self.target.split('/').any(|seg| seg == "..") {
			return Err(VendorAssetError::PathTraversal(self.target));
		}
		if self.target.contains('\0') {
			return Err(VendorAssetError::NullByte(self.target));
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	#[case::valid_simple("vendor/htmx.min.js")]
	#[case::valid_nested("vendor/fonts/inter-400.woff2")]
	fn validate_target_accepts_well_formed(#[case] target: &'static str) {
		// Arrange
		let asset = AppVendorAsset {
			app_label: "blog",
			url: "https://example.test/x.js",
			target,
			sha256: "",
		};

		// Act
		let result = asset.validate();

		// Assert
		assert!(
			result.is_ok(),
			"expected {:?} to be accepted, got {:?}",
			target,
			result
		);
	}

	#[rstest]
	#[case::missing_prefix("htmx.min.js", "must start with \"vendor/\"")]
	#[case::parent_traversal("vendor/../etc/passwd", "must not contain")]
	#[case::absolute_unix("/etc/passwd", "must start with \"vendor/\"")]
	#[case::null_byte("vendor/foo\0.js", "must not contain null byte")]
	#[case::empty("", "must start with \"vendor/\"")]
	fn validate_target_rejects_bad(
		#[case] target: &'static str,
		#[case] expected_msg: &'static str,
	) {
		// Arrange
		let asset = AppVendorAsset {
			app_label: "blog",
			url: "https://example.test/x.js",
			target,
			sha256: "",
		};

		// Act
		let err = asset.validate().expect_err("validation should fail");

		// Assert
		assert!(
			err.to_string().contains(expected_msg),
			"expected error to contain {:?}, got {:?}",
			expected_msg,
			err.to_string()
		);
	}
}
