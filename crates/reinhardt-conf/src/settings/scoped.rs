//! Selected-value resolution for command-aware settings bootstrap.
//!
//! Source syntax and precedence are evaluated at build time. Environment
//! references and typed conversion are evaluated only when a command requests
//! an effective path. An eager-only custom source cannot enter this path.

use super::builder::{BuildError, MergeStrategy};
use super::interpolation::{InterpolationError, Interpolator};
use super::profile::Profile;
use super::sources::ConfigSource;
use super::typed_deserializer::TypedSettingsDeserializer;
use indexmap::IndexMap;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Raw merged settings with interpolation provenance for effective leaves.
pub struct ScopedSettings {
	values: IndexMap<String, Value>,
	provenance: HashMap<Vec<String>, Provenance>,
	profile: Option<Profile>,
	typed_coercion: bool,
}

struct Provenance {
	interpolation_file: Option<PathBuf>,
	rank: usize,
}

impl ScopedSettings {
	pub(crate) fn build(
		mut sources: Vec<Box<dyn ConfigSource>>,
		profile: Option<Profile>,
		typed_coercion: bool,
		strategy: MergeStrategy,
	) -> Result<Self, BuildError> {
		sources.sort_by_key(|source| source.priority());
		let mut values = IndexMap::new();
		let mut provenance = HashMap::new();
		for (rank, source) in sources.iter().enumerate() {
			let description = source.description();
			let loaded = source
				.load_scoped()
				.map_err(|error| BuildError::Source { description, error })?;
			for (key, value) in loaded.values {
				let path = vec![key.clone()];
				let existed = values.contains_key(&key);
				let previous = values.get_mut(&key);
				merge_value(
					previous,
					&value,
					&path,
					strategy,
					&mut provenance,
					loaded.interpolation_file.as_deref(),
					rank,
				);
				if !existed {
					values.insert(key, value);
				}
			}
		}
		Ok(Self {
			values,
			provenance,
			profile,
			typed_coercion,
		})
	}

	/// Profile selected by the settings builder.
	pub fn profile(&self) -> Option<Profile> {
		self.profile
	}

	/// Resolve and deserialize exactly one effective configuration path.
	///
	/// A missing path is an error. Use [`Self::optional_path`] only for fields
	/// whose schema documents an optional default.
	pub fn require_path<T: DeserializeOwned>(&self, path: &[&str]) -> Result<T, BuildError> {
		let key = path.join(".");
		let Some(value) = self.raw_path(path) else {
			return Err(BuildError::Deserialization(format!(
				"missing required settings path `{key}`"
			)));
		};
		let mut value = value.clone();
		let mut segments: Vec<String> = path.iter().map(|segment| (*segment).to_owned()).collect();
		self.interpolate_value(&mut value, &mut segments)?;
		if self.typed_coercion {
			T::deserialize(TypedSettingsDeserializer::new(&value)).map_err(|_| {
				BuildError::Deserialization(format!(
					"invalid value at settings path `{key}`; check its type and required fields"
				))
			})
		} else {
			serde_json::from_value(value).map_err(|_| {
				BuildError::Deserialization(format!("invalid value at settings path `{key}`"))
			})
		}
	}

	/// Resolve an optional effective path without treating malformed values as absent.
	pub fn optional_path<T: DeserializeOwned>(
		&self,
		path: &[&str],
	) -> Result<Option<T>, BuildError> {
		if self.raw_path(path).is_none() {
			Ok(None)
		} else {
			self.require_path(path).map(Some)
		}
	}

	/// Select the highest-priority present leaf among equivalent schema paths.
	/// Candidate order breaks ties within one source. A lower-priority malformed
	/// alias is not deserialized once a higher-priority path is selected.
	pub fn first_present_path<T: DeserializeOwned>(
		&self,
		paths: &[&[&str]],
	) -> Result<Option<T>, BuildError> {
		let mut selected: Option<(&[&str], usize)> = None;
		for path in paths {
			if self.raw_path(path).is_some() {
				let segments: Vec<String> =
					path.iter().map(|segment| (*segment).to_owned()).collect();
				let rank = self
					.provenance
					.get(&segments)
					.map_or(0, |origin| origin.rank);
				if selected.is_none_or(|(_, selected_rank)| rank > selected_rank) {
					selected = Some((path, rank));
				}
			}
		}
		selected
			.map(|(path, _)| self.require_path(path))
			.transpose()
	}

	fn raw_path(&self, path: &[&str]) -> Option<&Value> {
		let (first, rest) = path.split_first()?;
		let mut value = self.values.get(*first)?;
		for segment in rest {
			value = value.as_object()?.get(*segment)?;
		}
		Some(value)
	}

	fn interpolate_value(
		&self,
		value: &mut Value,
		path: &mut Vec<String>,
	) -> Result<(), BuildError> {
		match value {
			Value::String(raw) => {
				if let Some(file) = self
					.provenance
					.get(path)
					.and_then(|origin| origin.interpolation_file.as_ref())
				{
					let lookup = |name: &str| std::env::var(name).ok();
					let interpolator = Interpolator::new(&lookup);
					*raw = interpolator
						.interpolate_str(raw)
						.map_err(|error| scoped_interpolation_error(error, file, path))?;
				}
			}
			Value::Object(object) => {
				for (key, child) in object {
					path.push(key.clone());
					self.interpolate_value(child, path)?;
					path.pop();
				}
			}
			Value::Array(array) => {
				// Arrays are replaced atomically during merging, so every element
				// inherits the array's source and interpolation policy.
				let origin = self
					.provenance
					.get(path)
					.and_then(|origin| origin.interpolation_file.clone());
				for (index, child) in array.iter_mut().enumerate() {
					path.push(format!("[{index}]"));
					if let Some(origin) = &origin {
						Self::interpolate_array_value(child, path, origin)?;
					}
					path.pop();
				}
			}
			_ => {}
		}
		Ok(())
	}

	fn interpolate_array_value(
		value: &mut Value,
		path: &mut Vec<String>,
		file: &Path,
	) -> Result<(), BuildError> {
		match value {
			Value::String(raw) => {
				let lookup = |name: &str| std::env::var(name).ok();
				*raw = Interpolator::new(&lookup)
					.interpolate_str(raw)
					.map_err(|error| scoped_interpolation_error(error, file, path))?;
			}
			Value::Object(object) => {
				for (key, child) in object {
					path.push(key.clone());
					Self::interpolate_array_value(child, path, file)?;
					path.pop();
				}
			}
			Value::Array(array) => {
				for (index, child) in array.iter_mut().enumerate() {
					path.push(format!("[{index}]"));
					Self::interpolate_array_value(child, path, file)?;
					path.pop();
				}
			}
			_ => {}
		}
		Ok(())
	}
}

fn merge_value(
	previous: Option<&mut Value>,
	incoming: &Value,
	path: &[String],
	strategy: MergeStrategy,
	provenance: &mut HashMap<Vec<String>, Provenance>,
	file: Option<&Path>,
	rank: usize,
) {
	match (previous, incoming) {
		(Some(Value::Object(existing)), Value::Object(new)) if strategy == MergeStrategy::Deep => {
			provenance.insert(
				path.to_vec(),
				Provenance {
					interpolation_file: file.map(Path::to_path_buf),
					rank,
				},
			);
			for (key, child) in new {
				let mut child_path = path.to_vec();
				child_path.push(key.clone());
				let existed = existing.contains_key(key);
				let old = existing.get_mut(key);
				merge_value(old, child, &child_path, strategy, provenance, file, rank);
				if !existed {
					existing.insert(key.clone(), child.clone());
				}
			}
		}
		(old, incoming) => {
			provenance.retain(|key, _| !key.starts_with(path));
			record_provenance(incoming, path, file, rank, provenance);
			if let Some(old) = old {
				*old = incoming.clone();
			}
		}
	}
}

fn record_provenance(
	value: &Value,
	path: &[String],
	file: Option<&Path>,
	rank: usize,
	provenance: &mut HashMap<Vec<String>, Provenance>,
) {
	provenance.insert(
		path.to_vec(),
		Provenance {
			interpolation_file: file.map(Path::to_path_buf),
			rank,
		},
	);
	if let Value::Object(object) = value {
		for (key, child) in object {
			let mut child_path = path.to_vec();
			child_path.push(key.clone());
			record_provenance(child, &child_path, file, rank, provenance);
		}
	}
}

fn scoped_interpolation_error(
	error: InterpolationError,
	file: &Path,
	path: &[String],
) -> BuildError {
	let path = path.join(".");
	let file = file.to_path_buf();
	let error = match error {
		InterpolationError::Required { var, .. } => InterpolationError::Required {
			var,
			path: file,
			key_path: path,
		},
		InterpolationError::RequiredWithMessage { var, .. } => {
			InterpolationError::RequiredWithMessage {
				var,
				message: "check this required setting's environment variable".to_owned(),
				path: file,
				key_path: path,
			}
		}
		InterpolationError::Syntax { .. } => InterpolationError::Syntax {
			detail: "check the interpolation expression".to_owned(),
			snippet: "[REDACTED]".to_owned(),
			path: file,
			key_path: path,
		},
	};
	BuildError::Deserialization(error.to_string())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::settings::builder::SettingsBuilder;
	use crate::settings::sources::{ConfigSource, DefaultSource, SourceError, TomlFileSource};
	use rstest::*;
	use serde_json::json;
	use std::fs;

	#[rstest]
	fn selected_effective_values_ignore_unused_and_shadowed_references() {
		let dir = tempfile::tempdir().unwrap();
		let base = dir.path().join("base.toml");
		let overlay = dir.path().join("overlay.toml");
		fs::write(
			&base,
			"[static]\nroot = \"${REINHARDT_SCOPED_MISSING_ROOT_6336}\"\n[server]\nsecret = \"${REINHARDT_SCOPED_MISSING_SECRET_6336}\"\n",
		).unwrap();
		fs::write(&overlay, "[static]\nroot = \"dist\"\n").unwrap();
		let settings = SettingsBuilder::new()
			.add_source(TomlFileSource::new(&base))
			.add_source(TomlFileSource::new(&overlay))
			.build_scoped()
			.unwrap();
		assert_eq!(
			settings
				.require_path::<String>(&["static", "root"])
				.unwrap(),
			"dist"
		);
		assert!(
			settings
				.require_path::<String>(&["server", "secret"])
				.is_err()
		);
	}

	#[rstest]
	fn invalid_selected_value_fails_without_optional_fallback() {
		let settings = SettingsBuilder::new()
			.add_source(DefaultSource::new().with_value("static", json!({"root": 37})))
			.build_scoped()
			.unwrap();
		assert!(
			settings
				.optional_path::<String>(&["static", "root"])
				.is_err()
		);
		assert_eq!(
			settings
				.optional_path::<String>(&["static", "url"])
				.unwrap(),
			None
		);
	}

	#[rstest]
	fn equivalent_paths_follow_source_precedence_before_schema_preference() {
		let dir = tempfile::tempdir().unwrap();
		let base = dir.path().join("base.toml");
		let overlay = dir.path().join("overlay.toml");
		fs::write(&base, "[static]\nroot = 37\n").unwrap();
		fs::write(&overlay, "static_root = \"dist\"\n").unwrap();
		let settings = SettingsBuilder::new()
			.add_source(TomlFileSource::new(&base))
			.add_source(TomlFileSource::new(&overlay))
			.build_scoped()
			.unwrap();
		assert_eq!(
			settings
				.first_present_path::<String>(&[&["static", "root"], &["static_root"]])
				.unwrap(),
			Some("dist".to_owned())
		);
	}

	#[rstest]
	fn syntax_error_in_unselected_file_still_fails() {
		let dir = tempfile::tempdir().unwrap();
		let bad = dir.path().join("bad.toml");
		fs::write(&bad, "[unused\nsecret = 'anything'").unwrap();
		assert!(
			SettingsBuilder::new()
				.add_source(TomlFileSource::new(bad))
				.build_scoped()
				.is_err()
		);
	}

	#[rstest]
	fn selected_interpolation_error_redacts_value_and_custom_hint() {
		let dir = tempfile::tempdir().unwrap();
		let config = dir.path().join("settings.toml");
		fs::write(
			&config,
			"[static]\nroot = \"${REINHARDT_SCOPED_MISSING_ROOT_6336:?sensitive-hint}\"\n",
		)
		.unwrap();
		let settings = SettingsBuilder::new()
			.add_source(TomlFileSource::new(config))
			.build_scoped()
			.unwrap();
		let error = settings
			.require_path::<String>(&["static", "root"])
			.unwrap_err();
		let message = error.to_string();
		assert!(message.contains("REINHARDT_SCOPED_MISSING_ROOT_6336"));
		assert!(!message.contains("sensitive-hint"));
		assert!(!message.contains("${"));
	}

	struct EagerOnlySource;

	impl ConfigSource for EagerOnlySource {
		fn load(&self) -> Result<IndexMap<String, Value>, SourceError> {
			panic!("scoped bootstrap must not call eager load")
		}

		fn priority(&self) -> u8 {
			50
		}
		fn description(&self) -> String {
			"eager-only source".to_owned()
		}
	}

	#[rstest]
	fn eager_only_sources_report_migration_guidance() {
		let error = SettingsBuilder::new()
			.add_source(EagerOnlySource)
			.build_scoped();
		let message = error.err().unwrap().to_string();
		assert!(message.contains("does not support scoped loading"));
		assert!(message.contains("legacy eager entry point"));
	}
}
