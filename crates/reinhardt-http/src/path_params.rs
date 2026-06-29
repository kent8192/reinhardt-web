//! Ordered path parameter storage.
//!
//! `PathParams` preserves the order in which path parameters appear in the URL
//! pattern, which is essential for correct tuple-based extraction such as
//! `Path<(T1, T2)>`. Internally it uses inline storage for the common small
//! parameter sets, but it exposes a small subset of the `HashMap`-like API
//! (`get`, `iter`, `len`, `is_empty`, `insert`, `values`) so existing callers
//! can continue to look up parameters by name without any code changes.
//!
//! # Why ordered storage and not a `HashMap`?
//!
//! `HashMap` iteration order is non-deterministic. URL routers (matchit in
//! particular) yield parameters in URL declaration order, which is the order
//! users expect when destructuring `Path<(T1, T2)>`. Storing parameters as an
//! ordered sequence preserves that order all the way from the router to the
//! extractor.
//!
//! See issue #4013 for details.

use std::{collections::HashMap, sync::Arc};

use smallvec::SmallVec;

const INLINE_PARAM_CAPACITY: usize = 4;
const INLINE_VALUE_CAPACITY: usize = 32;
type PathParamNames = SmallVec<[String; INLINE_PARAM_CAPACITY]>;
type PathParamValues = SmallVec<[PathParamValue; INLINE_PARAM_CAPACITY]>;
type PathParamValueBytes = SmallVec<[u8; INLINE_VALUE_CAPACITY]>;

#[derive(Debug, Clone, PartialEq, Eq)]
struct PathParamValue {
	inner: PathParamValueBytes,
}

impl PathParamValue {
	fn as_str(&self) -> &str {
		std::str::from_utf8(&self.inner)
			.expect("path parameter values are created from valid UTF-8 strings")
	}

	fn into_string(self) -> String {
		String::from_utf8(self.inner.into_vec())
			.expect("path parameter values are created from valid UTF-8 strings")
	}
}

impl From<&str> for PathParamValue {
	fn from(value: &str) -> Self {
		Self {
			inner: value.as_bytes().iter().copied().collect(),
		}
	}
}

impl From<String> for PathParamValue {
	fn from(value: String) -> Self {
		Self {
			inner: value.into_bytes().into_iter().collect(),
		}
	}
}

impl From<&String> for PathParamValue {
	fn from(value: &String) -> Self {
		value.as_str().into()
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PathParamNameStorage {
	Owned(PathParamNames),
	Shared(Arc<[String]>),
}

impl Default for PathParamNameStorage {
	fn default() -> Self {
		Self::Owned(PathParamNames::new())
	}
}

impl PathParamNameStorage {
	fn with_capacity(capacity: usize) -> Self {
		Self::Owned(PathParamNames::with_capacity(capacity))
	}

	fn get(&self, index: usize) -> Option<&String> {
		match self {
			Self::Owned(names) => names.get(index),
			Self::Shared(names) => names.get(index),
		}
	}

	fn position(&self, key: &str) -> Option<usize> {
		match self {
			Self::Owned(names) => names.iter().position(|name| name == key),
			Self::Shared(names) => names.iter().position(|name| name == key),
		}
	}

	fn push(&mut self, key: String) {
		self.ensure_owned().push(key);
	}

	fn ensure_owned(&mut self) -> &mut PathParamNames {
		if let Self::Shared(names) = self {
			*self = Self::Owned(names.iter().cloned().collect());
		}
		match self {
			Self::Owned(names) => names,
			Self::Shared(_) => unreachable!("shared names are materialized before mutation"),
		}
	}
}

/// Iterator over `(key, value)` pairs in declaration order.
pub struct PathParamsIter<'a> {
	params: &'a PathParams,
	index: usize,
}

impl<'a> Iterator for PathParamsIter<'a> {
	type Item = (&'a String, &'a str);

	fn next(&mut self) -> Option<Self::Item> {
		let index = self.index;
		self.index += 1;
		Some((
			self.params.names.get(index)?,
			self.params.values.get(index)?.as_str(),
		))
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let remaining = self.params.len().saturating_sub(self.index);
		(remaining, Some(remaining))
	}
}

impl ExactSizeIterator for PathParamsIter<'_> {}

/// Ordered collection of path parameters extracted from a URL pattern.
///
/// Preserves insertion order so that tuple extractors like `Path<(T1, T2)>`
/// can rely on URL pattern declaration order when populating tuple fields.
///
/// # Example
///
/// ```
/// use reinhardt_http::PathParams;
///
/// let mut params = PathParams::new();
/// params.insert("org", "myslug");
/// params.insert("cluster_id", "5");
///
/// // Insertion order is preserved.
/// let collected: Vec<_> = params.iter().map(|(k, v)| (k.as_str(), v)).collect();
/// assert_eq!(collected, vec![("org", "myslug"), ("cluster_id", "5")]);
///
/// // Named lookup still works.
/// assert_eq!(params.get("org"), Some("myslug"));
/// ```
#[derive(Debug, Clone, Default)]
pub struct PathParams {
	names: PathParamNameStorage,
	values: PathParamValues,
}

impl PartialEq for PathParams {
	fn eq(&self, other: &Self) -> bool {
		self.iter().eq(other.iter())
	}
}

impl Eq for PathParams {}

impl PathParams {
	/// Create a new, empty `PathParams`.
	pub fn new() -> Self {
		Self {
			names: PathParamNameStorage::default(),
			values: PathParamValues::new(),
		}
	}

	/// Create an empty `PathParams` with capacity for `capacity` entries.
	pub fn with_capacity(capacity: usize) -> Self {
		Self {
			names: PathParamNameStorage::with_capacity(capacity),
			values: PathParamValues::with_capacity(capacity),
		}
	}

	/// Build path params from shared route parameter names and request-local values.
	///
	/// Routers use this to avoid allocating key strings on every request.
	pub fn from_shared_names<I, V>(names: Arc<[String]>, values: I) -> Self
	where
		I: IntoIterator<Item = V>,
		V: AsRef<str>,
	{
		let values: PathParamValues = values
			.into_iter()
			.map(|value| PathParamValue::from(value.as_ref()))
			.collect();
		assert_eq!(
			names.len(),
			values.len(),
			"shared path parameter names and values must have the same length"
		);
		Self {
			names: PathParamNameStorage::Shared(names),
			values,
		}
	}

	/// Number of stored parameters.
	pub fn len(&self) -> usize {
		self.values.len()
	}

	/// `true` if no parameters are stored.
	pub fn is_empty(&self) -> bool {
		self.values.is_empty()
	}

	/// Look up a parameter by name.
	///
	/// Returns the first match if multiple entries share the same name (which
	/// should not happen in practice because URL patterns require unique names).
	pub fn get(&self, key: &str) -> Option<&str> {
		let index = self.names.position(key)?;
		self.values.get(index).map(PathParamValue::as_str)
	}

	/// Insert or update a parameter.
	///
	/// If `key` already exists, its value is replaced and its position is kept.
	/// Otherwise the new entry is appended, preserving insertion order.
	pub fn insert(&mut self, key: impl Into<String>, value: impl AsRef<str>) {
		let key = key.into();
		let value = PathParamValue::from(value.as_ref());
		if let Some(index) = self.names.position(&key) {
			self.values[index] = value;
		} else {
			self.names.push(key);
			self.values.push(value);
		}
	}

	/// Iterate over `(key, value)` pairs in insertion order.
	pub fn iter(&self) -> PathParamsIter<'_> {
		PathParamsIter {
			params: self,
			index: 0,
		}
	}

	/// Iterate over values in insertion order.
	pub fn values(&self) -> impl Iterator<Item = &str> {
		self.values.iter().map(PathParamValue::as_str)
	}

	/// Clone the ordered `(key, value)` pairs into a `Vec`.
	pub fn to_vec(&self) -> Vec<(String, String)> {
		self.iter()
			.map(|(key, value)| (key.clone(), value.to_string()))
			.collect()
	}

	/// Consume the wrapper and return the inner ordered `Vec`.
	pub fn into_vec(self) -> Vec<(String, String)> {
		match self.names {
			PathParamNameStorage::Owned(names) => names
				.into_iter()
				.zip(self.values.into_iter().map(PathParamValue::into_string))
				.collect(),
			PathParamNameStorage::Shared(names) => names
				.iter()
				.cloned()
				.zip(self.values.into_iter().map(PathParamValue::into_string))
				.collect(),
		}
	}
}

impl<K, V> FromIterator<(K, V)> for PathParams
where
	K: Into<String>,
	V: AsRef<str>,
{
	fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
		let mut params = PathParams::new();
		for (k, v) in iter {
			params.insert(k, v);
		}
		params
	}
}

impl IntoIterator for PathParams {
	type Item = (String, String);
	type IntoIter = std::vec::IntoIter<(String, String)>;

	fn into_iter(self) -> Self::IntoIter {
		self.into_vec().into_iter()
	}
}

impl<'a> IntoIterator for &'a PathParams {
	type Item = (&'a String, &'a str);
	type IntoIter = PathParamsIter<'a>;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

impl From<Vec<(String, String)>> for PathParams {
	fn from(inner: Vec<(String, String)>) -> Self {
		// Caller is responsible for the ordering of the supplied vector.
		let mut params = Self::with_capacity(inner.len());
		for (key, value) in inner {
			params.insert(key, value);
		}
		params
	}
}

impl From<HashMap<String, String>> for PathParams {
	/// Convert from a `HashMap`. Iteration order is **not** preserved because
	/// `HashMap` does not have a defined order. Prefer `From<Vec<_>>` when
	/// order matters.
	fn from(map: HashMap<String, String>) -> Self {
		map.into_iter().collect()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn insert_preserves_order() {
		// Arrange
		let mut params = PathParams::new();

		// Act
		params.insert("z", "first");
		params.insert("a", "second");
		params.insert("m", "third");

		// Assert
		let order: Vec<&str> = params.iter().map(|(k, _)| k.as_str()).collect();
		assert_eq!(order, vec!["z", "a", "m"]);
	}

	#[rstest]
	fn get_finds_by_name() {
		// Arrange
		let mut params = PathParams::new();
		params.insert("org", "myslug");
		params.insert("cluster_id", "5");

		// Act
		let org = params.get("org");
		let cluster_id = params.get("cluster_id");
		let missing = params.get("missing");

		// Assert
		assert_eq!(org, Some("myslug"));
		assert_eq!(cluster_id, Some("5"));
		assert_eq!(missing, None);
	}

	#[rstest]
	fn insert_replaces_existing_in_place() {
		// Arrange
		let mut params = PathParams::new();
		params.insert("a", "1");
		params.insert("b", "2");

		// Act
		params.insert("a", "updated");

		// Assert: order unchanged, value replaced
		let collected: Vec<_> = params.iter().map(|(k, v)| (k.as_str(), v)).collect();
		assert_eq!(collected, vec![("a", "updated"), ("b", "2")]);
	}

	#[rstest]
	fn from_vec_preserves_caller_order() {
		// Arrange
		let vec = vec![
			("org".to_string(), "myslug".to_string()),
			("cluster_id".to_string(), "5".to_string()),
		];

		// Act
		let params = PathParams::from(vec);

		// Assert
		let order: Vec<&str> = params.iter().map(|(k, _)| k.as_str()).collect();
		assert_eq!(order, vec!["org", "cluster_id"]);
	}

	#[rstest]
	fn from_iter_collects_in_order() {
		// Arrange
		let pairs = vec![("z", "1"), ("a", "2")];

		// Act
		let params: PathParams = pairs.into_iter().collect();

		// Assert
		let order: Vec<&str> = params.iter().map(|(k, _)| k.as_str()).collect();
		assert_eq!(order, vec!["z", "a"]);
	}

	#[rstest]
	fn consuming_iterator_keeps_public_vec_iterator_type() {
		// Arrange
		let params = PathParams::from(vec![("org".to_string(), "myslug".to_string())]);

		// Act
		let mut iter: std::vec::IntoIter<(String, String)> = params.into_iter();

		// Assert
		assert_eq!(iter.next(), Some(("org".to_string(), "myslug".to_string())));
		assert_eq!(iter.next(), None);
	}
}
