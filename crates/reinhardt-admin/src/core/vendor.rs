//! Deprecated vendor asset shim.
//!
//! This module preserves the previous public API surface
//! (`VendorAsset`, `Verbosity`, `verify_integrity`, `download_vendor_assets`,
//! `ensure_vendor_assets`, `admin_vendor_assets`) for backwards compatibility.
//! All items here are deprecated and forward to
//! `reinhardt_utils::staticfiles::vendor`, which is now the canonical home for
//! vendor asset declaration and download. Admin's own assets are now declared
//! via `inventory::submit!` in `reinhardt-admin/src/lib.rs`.
//!
//! These items will be removed in the next major version bump.

#![allow(deprecated, reason = "Internal implementation of deprecated shims")]

#[cfg(server)]
use std::path::Path;

/// Deprecated. Use `reinhardt_utils::staticfiles::vendor::AppVendorAsset`.
#[cfg(server)]
#[deprecated(
	since = "0.1.0-rc.27",
	note = "Use reinhardt_utils::staticfiles::vendor::AppVendorAsset instead"
)]
pub struct VendorAsset {
	/// The version-pinned CDN URL to download the asset from.
	pub url: &'static str,
	/// Relative path within the static directory where the asset will be stored.
	pub target: &'static str,
	/// Expected SHA-256 hex digest of the file content.
	pub sha256: &'static str,
}

/// Deprecated. Use `reinhardt_utils::staticfiles::vendor::Verbosity`.
#[cfg(server)]
#[deprecated(
	since = "0.1.0-rc.27",
	note = "Use reinhardt_utils::staticfiles::vendor::Verbosity instead"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verbosity {
	/// No output.
	Silent,
	/// Print one line per downloaded file.
	Normal,
	/// Print detailed progress including skip messages.
	Verbose,
}

#[cfg(server)]
impl From<Verbosity> for reinhardt_utils::staticfiles::vendor::Verbosity {
	fn from(v: Verbosity) -> Self {
		match v {
			Verbosity::Silent => Self::Silent,
			Verbosity::Normal => Self::Normal,
			Verbosity::Verbose => Self::Verbose,
		}
	}
}

/// Deprecated. Admin vendor assets are now registered via `inventory::submit!`
/// in `reinhardt-admin/src/lib.rs` and queried via
/// `reinhardt_utils::staticfiles::vendor::registered_assets_for_app("admin")`.
///
/// This function returns an empty slice; previous callers received a
/// hard-coded array now superseded by the inventory-based registration.
#[cfg(server)]
#[deprecated(
	since = "0.1.0-rc.27",
	note = "Admin vendor assets are now registered via inventory; \
	        use reinhardt_utils::staticfiles::vendor::registered_assets_for_app(\"admin\") instead"
)]
pub fn admin_vendor_assets() -> &'static [VendorAsset] {
	&[]
}

/// Deprecated. Use `reinhardt_utils::staticfiles::vendor::verify_integrity`.
#[cfg(server)]
#[deprecated(
	since = "0.1.0-rc.27",
	note = "Use reinhardt_utils::staticfiles::vendor::verify_integrity instead"
)]
pub fn verify_integrity(path: &Path, expected_sha256: &str) -> Result<(), String> {
	reinhardt_utils::staticfiles::vendor::verify_integrity(path, expected_sha256)
		.map_err(|e| e.to_string())
}

/// Deprecated. Use
/// `reinhardt_utils::staticfiles::vendor::ensure_vendor_assets_for_app` or
/// `download_all_vendor_assets`. This shim downloads only the assets registered
/// for the `admin` app label.
#[cfg(server)]
#[deprecated(
	since = "0.1.0-rc.27",
	note = "Use reinhardt_utils::staticfiles::vendor::ensure_vendor_assets_for_app instead"
)]
pub async fn download_vendor_assets(
	base_dir: &Path,
	verbosity: Verbosity,
) -> Result<(), anyhow::Error> {
	let assets = reinhardt_utils::staticfiles::vendor::registered_assets_for_app("admin");
	reinhardt_utils::staticfiles::vendor::download_assets(base_dir, &assets, verbosity.into())
		.await
		.map_err(|e| anyhow::anyhow!("{}", e))
}

/// Deprecated. Use
/// `reinhardt_utils::staticfiles::vendor::ensure_vendor_assets_for_app("admin", base_dir)`.
#[cfg(server)]
#[deprecated(
	since = "0.1.0-rc.27",
	note = "Use reinhardt_utils::staticfiles::vendor::ensure_vendor_assets_for_app instead"
)]
pub async fn ensure_vendor_assets(base_dir: &std::path::Path) {
	let _ =
		reinhardt_utils::staticfiles::vendor::ensure_vendor_assets_for_app("admin", base_dir).await;
}
