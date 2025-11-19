use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

/// A context for template rendering.
///
/// This structure provides a type-safe way to pass data to templates,
/// replacing the generic `HashMap<K, V>` pattern.
///
/// # Examples
///
/// ```
/// use reinhardt_shortcuts::TemplateContext;
///
/// let mut context = TemplateContext::new();
/// context.insert("user", "Alice");
/// context.insert("count", 42);
/// ```
#[derive(Debug, Clone, Default)]
pub struct TemplateContext {
	inner: HashMap<String, Value>,
}

impl TemplateContext {
	/// Creates a new empty template context.
	pub fn new() -> Self {
		Self {
			inner: HashMap::new(),
		}
	}

	/// Inserts a key-value pair into the context.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_shortcuts::TemplateContext;
	///
	/// let mut context = TemplateContext::new();
	/// context.insert("name", "Bob");
	/// context.insert("age", 30);
	/// ```
	pub fn insert<K, V>(&mut self, key: K, value: V)
	where
		K: Into<String>,
		V: Serialize,
	{
		let key = key.into();
		let value = serde_json::to_value(value).unwrap_or(Value::Null);
		self.inner.insert(key, value);
	}

	/// Creates a context from a HashMap.
	///
	/// # Examples
	///
	/// ```
	/// use std::collections::HashMap;
	/// use reinhardt_shortcuts::TemplateContext;
	///
	/// let mut map = HashMap::new();
	/// map.insert("key", "value");
	///
	/// let context = TemplateContext::from_map(map);
	/// ```
	pub fn from_map<K, V>(map: HashMap<K, V>) -> Self
	where
		K: AsRef<str>,
		V: Serialize,
	{
		let mut inner = HashMap::new();
		for (k, v) in map {
			let key = k.as_ref().to_string();
			let value = serde_json::to_value(v).unwrap_or(Value::Null);
			inner.insert(key, value);
		}
		Self { inner }
	}

	/// Returns a reference to the inner HashMap.
	pub(crate) fn as_map(&self) -> &HashMap<String, Value> {
		&self.inner
	}
}

impl<K, V> From<HashMap<K, V>> for TemplateContext
where
	K: AsRef<str>,
	V: Serialize,
{
	fn from(map: HashMap<K, V>) -> Self {
		Self::from_map(map)
	}
}
