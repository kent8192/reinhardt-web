//! Inventory query helpers for vendor assets.

use crate::staticfiles::vendor::asset::AppVendorAsset;

/// Return all `AppVendorAsset` entries registered for the given app label.
pub fn registered_assets_for_app(app_label: &str) -> Vec<AppVendorAsset> {
	inventory::iter::<AppVendorAsset>()
		.copied()
		.filter(|a| a.app_label == app_label)
		.collect()
}

/// Return all registered `AppVendorAsset` entries across all apps.
pub fn all_registered_assets() -> Vec<AppVendorAsset> {
	inventory::iter::<AppVendorAsset>().copied().collect()
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	// Use a synthetic submission to exercise the iterator regardless of which
	// other crates have registered entries. inventory::submit! works at any
	// crate so this test can stand on its own.
	inventory::submit! {
		AppVendorAsset {
			app_label: "__registry_test_app__",
			url: "https://example.test/file.js",
			target: "vendor/file.js",
			sha256: "",
		}
	}

	#[rstest]
	fn registered_assets_for_app_returns_only_matching_label() {
		// Arrange — synthetic entry above is registered at compile time.

		// Act
		let entries = registered_assets_for_app("__registry_test_app__");

		// Assert
		assert_eq!(entries.len(), 1, "expected exactly one matching entry");
		assert_eq!(entries[0].url, "https://example.test/file.js");
		assert_eq!(entries[0].target, "vendor/file.js");
	}

	#[rstest]
	fn registered_assets_for_app_returns_empty_for_unknown_label() {
		// Arrange — no entries registered for this label.

		// Act
		let entries = registered_assets_for_app("__no_such_app_anywhere__");

		// Assert
		assert!(entries.is_empty(), "expected empty, got {:?}", entries);
	}

	#[rstest]
	fn all_registered_assets_includes_synthetic_entry() {
		// Arrange — synthetic entry above is registered at compile time.

		// Act
		let all = all_registered_assets();

		// Assert
		assert!(
			all.iter().any(|a| a.app_label == "__registry_test_app__"),
			"all_registered_assets must include synthetic entry"
		);
	}
}
