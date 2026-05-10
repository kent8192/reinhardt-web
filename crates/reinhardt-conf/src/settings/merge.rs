//! Deep merge primitive shared by the settings builder and test overrides.
//!
//! The settings system layers configuration from multiple sources (TOML
//! files, environment variables, defaults, thread-local test overrides).
//! Two distinct merge semantics are required:
//!
//! - **Shallow** — top-level key replacement. The legacy default for
//!   [`SettingsBuilder::build`](crate::settings::builder::SettingsBuilder::build).
//! - **Deep** — recursive merge of nested tables, with scalars and arrays
//!   still replaced wholesale. The default for
//!   [`SettingsBuilder::build_composed`](crate::settings::builder::SettingsBuilder::build_composed).
//!
//! The `deep_merge` function in this module implements the deep variant.
//! It is intentionally conservative: only `Value::Object` versus
//! `Value::Object` collisions recurse; every other shape (scalar, array,
//! mixed) defers to the source value, matching the behaviour described
//! in [issue #4260](https://github.com/kent8192/reinhardt-web/issues/4260).

use indexmap::IndexMap;
use serde_json::Value;

/// Deep-merges `source` into `target`.
///
/// For each `(key, source_value)` pair in `source`:
///
/// - If `target[key]` and `source_value` are both [`Value::Object`], merge
///   them recursively, preserving sibling keys that appear in only one
///   side.
/// - Otherwise replace `target[key]` with `source_value`. This includes
///   the case where one side is an object and the other is a scalar or
///   array — there is no safe way to merge mismatched shapes.
///
/// Arrays are **never** merged element-wise; the source array always
/// replaces the target array. This keeps the rule predictable for users:
/// "objects deep-merge, everything else replaces".
///
/// Insertion order of top-level `target` keys is preserved for keys that
/// already exist; new top-level keys from `source` are appended in their
/// original order. Nested object ordering follows `serde_json::Map`.
///
pub(crate) fn deep_merge(target: &mut IndexMap<String, Value>, source: IndexMap<String, Value>) {
	for (key, source_val) in source {
		match source_val {
			Value::Object(source_obj) => match target.get_mut(&key) {
				Some(Value::Object(target_obj)) => {
					// Both are objects — merge recursively.
					for (k, v) in source_obj {
						deep_merge_json(target_obj, k, v);
					}
				}
				_ => {
					target.insert(key, Value::Object(source_obj));
				}
			},
			source_val => {
				target.insert(key, source_val);
			}
		}
	}
}

/// Recursive helper for merging into a [`serde_json::Map`].
///
/// Mirrors [`deep_merge`] but operates on `serde_json::Map` to support
/// the nested-table case after the first level.
fn deep_merge_json(target: &mut serde_json::Map<String, Value>, key: String, value: Value) {
	match value {
		Value::Object(source_obj) => match target.get_mut(&key) {
			Some(Value::Object(target_obj)) => {
				for (k, v) in source_obj {
					deep_merge_json(target_obj, k, v);
				}
			}
			_ => {
				target.insert(key, Value::Object(source_obj));
			}
		},
		value => {
			target.insert(key, value);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;
	use serde_json::json;

	#[rstest]
	fn deep_merge_replaces_top_level_scalar() {
		// Arrange
		let mut target: IndexMap<String, Value> = IndexMap::new();
		target.insert("port".to_string(), json!(8080));
		let mut source: IndexMap<String, Value> = IndexMap::new();
		source.insert("port".to_string(), json!(9090));

		// Act
		deep_merge(&mut target, source);

		// Assert
		assert_eq!(target.get("port").unwrap(), &json!(9090));
	}

	#[rstest]
	fn deep_merge_recurses_into_nested_objects() {
		// Arrange
		let mut target: IndexMap<String, Value> = IndexMap::new();
		target.insert(
			"core".to_string(),
			json!({"secret_key": "from-base", "debug": false}),
		);
		let mut source: IndexMap<String, Value> = IndexMap::new();
		source.insert("core".to_string(), json!({"debug": true}));

		// Act
		deep_merge(&mut target, source);

		// Assert: `secret_key` survives, `debug` flips
		let core = target.get("core").unwrap().as_object().unwrap();
		assert_eq!(core.get("secret_key").unwrap(), &json!("from-base"));
		assert_eq!(core.get("debug").unwrap(), &json!(true));
	}

	#[rstest]
	fn deep_merge_preserves_distinct_top_level_keys() {
		// Arrange
		let mut target: IndexMap<String, Value> = IndexMap::new();
		target.insert("core".to_string(), json!({"debug": false}));
		let mut source: IndexMap<String, Value> = IndexMap::new();
		source.insert("cache".to_string(), json!({"ttl": 60}));

		// Act
		deep_merge(&mut target, source);

		// Assert: both top-level keys present
		assert!(target.get("core").is_some());
		assert!(target.get("cache").is_some());
	}

	#[rstest]
	fn deep_merge_replaces_array_wholesale() {
		// Arrange
		let mut target: IndexMap<String, Value> = IndexMap::new();
		target.insert("hosts".to_string(), json!(["a", "b", "c"]));
		let mut source: IndexMap<String, Value> = IndexMap::new();
		source.insert("hosts".to_string(), json!(["x"]));

		// Act
		deep_merge(&mut target, source);

		// Assert: arrays do not concatenate
		assert_eq!(target.get("hosts").unwrap(), &json!(["x"]));
	}

	#[rstest]
	fn deep_merge_replaces_when_shapes_mismatch() {
		// Arrange: target is object, source is scalar — cannot merge
		let mut target: IndexMap<String, Value> = IndexMap::new();
		target.insert("core".to_string(), json!({"debug": false}));
		let mut source: IndexMap<String, Value> = IndexMap::new();
		source.insert("core".to_string(), json!("disabled"));

		// Act
		deep_merge(&mut target, source);

		// Assert: source replaces wholesale
		assert_eq!(target.get("core").unwrap(), &json!("disabled"));
	}

	#[rstest]
	fn deep_merge_recurses_through_three_levels() {
		// Arrange: [core.security.secure_ssl_redirect] survives a partial
		// override at [core.security.session_cookie_secure].
		let mut target: IndexMap<String, Value> = IndexMap::new();
		target.insert(
			"core".to_string(),
			json!({
				"security": {
					"secure_ssl_redirect": true,
					"session_cookie_secure": false,
				}
			}),
		);
		let mut source: IndexMap<String, Value> = IndexMap::new();
		source.insert(
			"core".to_string(),
			json!({
				"security": {
					"session_cookie_secure": true,
				}
			}),
		);

		// Act
		deep_merge(&mut target, source);

		// Assert
		let security = target
			.get("core")
			.unwrap()
			.get("security")
			.unwrap()
			.as_object()
			.unwrap();
		assert_eq!(security.get("secure_ssl_redirect").unwrap(), &json!(true));
		assert_eq!(security.get("session_cookie_secure").unwrap(), &json!(true));
	}
}
