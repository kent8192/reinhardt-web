//! Vendor asset subsystem.
//!
//! Apps declare external assets (JS / CSS / fonts) via
//! `#[app_config(... vendor_assets(asset(url = ..., target = ...)))]`. The
//! macro emits `inventory::submit!` entries collected here, downloaded lazily
//! on first request, and served through the existing static files pipeline.

pub mod asset;

pub use asset::{AppVendorAsset, VendorAssetError};
