//! Vendor asset subsystem.
//!
//! Apps declare external assets (JS / CSS / fonts) via
//! `#[app_config(... vendor_assets(asset(url = ..., target = ...)))]`. The
//! macro emits `inventory::submit!` entries collected here, downloaded lazily
//! on first request, and served through the existing static files pipeline.

pub mod asset;
pub mod downloader;

pub use asset::{AppVendorAsset, VendorAssetError};
pub use downloader::{
	VendorDownloadError, Verbosity, download_all_vendor_assets, download_assets,
	ensure_vendor_assets_for_app, verify_integrity,
};
