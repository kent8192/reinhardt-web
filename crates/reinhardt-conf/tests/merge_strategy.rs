// Integration tests for `MergeStrategy` on `SettingsBuilder`.
//
// Layers two `DefaultSource` instances (both at priority 0, so iteration
// order follows insertion order) to simulate `base.toml` + `local.toml`
// composition without writing temporary files. The "base" source is added
// first; the "profile" source second. Under `Shallow` the profile entirely
// replaces overlapping top-level keys; under `Deep` nested tables are
// merged and sibling keys survive. See issue #4260.

use reinhardt_conf::settings::ComposedSettings;
use reinhardt_conf::settings::MergeStrategy;
use reinhardt_conf::settings::builder::SettingsBuilder;
use reinhardt_conf::settings::sources::DefaultSource;
use rstest::rstest;
use serde::{Deserialize, Serialize};
use serde_json::json;

// ---------------------------------------------------------------------------
// Helper: ComposedSettings target with a nested `[core]` section and one
// flat top-level key, mirroring the `redis_url` flat-key fallback path
// described in the issue.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct CoreFragment {
	debug: bool,
	secret_key: String,
	#[serde(default)]
	security: SecurityFragment,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Default)]
struct SecurityFragment {
	#[serde(default)]
	secure_ssl_redirect: bool,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct LayeredSettings {
	core: CoreFragment,
	#[serde(default)]
	redis_url: String,
}

impl ComposedSettings for LayeredSettings {
	fn validate_requirements(
		_merged: &indexmap::IndexMap<String, serde_json::Value>,
	) -> Result<(), reinhardt_conf::settings::builder::BuildError> {
		Ok(())
	}

	fn validate_fragments(
		&self,
		_profile: &reinhardt_conf::settings::profile::Profile,
	) -> reinhardt_conf::settings::validation::ValidationResult {
		Ok(())
	}
}

// ---------------------------------------------------------------------------
// build() — `Shallow` is the historical default; the profile layer wipes
// the entire `[core]` table from the base.
// ---------------------------------------------------------------------------

#[rstest]
fn build_with_shallow_strategy_keeps_legacy_replacement() {
	// Arrange: base provides full [core], profile only overrides debug.
	let base = DefaultSource::new().with_value(
		"core",
		json!({
			"secret_key": "from-base",
			"security": {"secure_ssl_redirect": true},
		}),
	);
	let profile = DefaultSource::new().with_value("core", json!({"debug": true}));

	// Act: default for build() is Shallow.
	let merged = SettingsBuilder::new()
		.add_source(base)
		.add_source(profile)
		.build()
		.unwrap();

	// Assert: profile's [core] replaced the base entirely; only `debug` survives.
	let core = merged
		.get_raw("core")
		.expect("core key present")
		.as_object()
		.unwrap();
	assert_eq!(core.get("debug"), Some(&json!(true)));
	assert!(
		core.get("secret_key").is_none(),
		"shallow merge must drop base's secret_key"
	);
	assert!(
		core.get("security").is_none(),
		"shallow merge must drop base's nested security"
	);
}

// ---------------------------------------------------------------------------
// build() — opt-in `Deep` strategy preserves base sibling keys inside the
// nested table.
// ---------------------------------------------------------------------------

#[rstest]
fn build_with_deep_strategy_preserves_nested_siblings() {
	// Arrange: same layering as above.
	let base = DefaultSource::new().with_value(
		"core",
		json!({
			"secret_key": "from-base",
			"security": {"secure_ssl_redirect": true},
		}),
	);
	let profile = DefaultSource::new().with_value("core", json!({"debug": true}));

	// Act: explicit opt-in to Deep.
	let merged = SettingsBuilder::new()
		.with_merge_strategy(MergeStrategy::Deep)
		.add_source(base)
		.add_source(profile)
		.build()
		.unwrap();

	// Assert: profile's `debug` was added without erasing siblings.
	let core = merged
		.get_raw("core")
		.expect("core key present")
		.as_object()
		.unwrap();
	assert_eq!(core.get("debug"), Some(&json!(true)));
	assert_eq!(core.get("secret_key"), Some(&json!("from-base")));
	let security = core
		.get("security")
		.and_then(serde_json::Value::as_object)
		.expect("security sub-table present");
	assert_eq!(security.get("secure_ssl_redirect"), Some(&json!(true)));
}

// ---------------------------------------------------------------------------
// build() — under Deep, scalar (flat-key) layering still replaces
// wholesale. This protects the `redis_url` / `jwt_secret` fallback paths
// that callers rely on.
// ---------------------------------------------------------------------------

#[rstest]
fn build_with_deep_strategy_replaces_top_level_scalars() {
	// Arrange: base sets a scalar, profile overrides it.
	let base = DefaultSource::new().with_value("redis_url", json!("redis://base"));
	let profile = DefaultSource::new().with_value("redis_url", json!("redis://profile"));

	// Act
	let merged = SettingsBuilder::new()
		.with_merge_strategy(MergeStrategy::Deep)
		.add_source(base)
		.add_source(profile)
		.build()
		.unwrap();

	// Assert
	assert_eq!(merged.get_raw("redis_url"), Some(&json!("redis://profile")));
}

// ---------------------------------------------------------------------------
// build_composed() — defaults to Deep, so consumers of layered TOML
// profiles no longer need to redeclare every field of the sections they
// touch. This is the main behaviour change of #4260.
// ---------------------------------------------------------------------------

#[rstest]
fn build_composed_defaults_to_deep() {
	// Arrange: the profile only changes `debug`, base owns secret_key and security.
	let base = DefaultSource::new().with_value(
		"core",
		json!({
			"debug": false,
			"secret_key": "from-base",
			"security": {"secure_ssl_redirect": true},
		}),
	);
	let profile = DefaultSource::new().with_value("core", json!({"debug": true}));

	// Act: build_composed() with no explicit strategy → Deep.
	let settings: LayeredSettings = SettingsBuilder::new()
		.add_source(base)
		.add_source(profile)
		.build_composed()
		.unwrap();

	// Assert: every base field is preserved, only `debug` was overridden.
	assert!(settings.core.debug);
	assert_eq!(settings.core.secret_key, "from-base");
	assert!(settings.core.security.secure_ssl_redirect);
}

// ---------------------------------------------------------------------------
// build_composed() — explicit `Shallow` opt-out restores the legacy
// behaviour (profile layer drops base sibling fields). Smoke-tests the
// escape hatch promised in `with_merge_strategy`.
// ---------------------------------------------------------------------------

#[rstest]
fn build_composed_with_explicit_shallow_drops_siblings() {
	// Arrange: same layering, but profile MUST redeclare every required
	// field to satisfy `serde` since Shallow drops the base [core] entirely.
	let base = DefaultSource::new().with_value(
		"core",
		json!({
			"debug": false,
			"secret_key": "from-base",
			"security": {"secure_ssl_redirect": true},
		}),
	);
	let profile = DefaultSource::new().with_value(
		"core",
		json!({
			"debug": true,
			"secret_key": "from-profile",
		}),
	);

	// Act
	let settings: LayeredSettings = SettingsBuilder::new()
		.with_merge_strategy(MergeStrategy::Shallow)
		.add_source(base)
		.add_source(profile)
		.build_composed()
		.unwrap();

	// Assert: profile values won wholesale; base's `security` defaulted
	// (proving it was dropped, not preserved).
	assert!(settings.core.debug);
	assert_eq!(settings.core.secret_key, "from-profile");
	assert!(!settings.core.security.secure_ssl_redirect);
}
