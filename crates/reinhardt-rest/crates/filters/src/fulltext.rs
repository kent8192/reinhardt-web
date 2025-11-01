//! Full-text search filter
//!
//! Provides full-text search capabilities with database integration.

use std::marker::PhantomData;

/// Full-text search mode
///
/// Determines how the search query is processed.
///
/// # Examples
///
/// ```
/// use reinhardt_filters::FullTextSearchMode;
///
/// let natural = FullTextSearchMode::Natural;
/// let boolean = FullTextSearchMode::Boolean;
/// let phrase = FullTextSearchMode::Phrase;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FullTextSearchMode {
	/// Natural language search
	///
	/// Interprets the search string as a natural language phrase.
	/// Best for user-facing search.
	Natural,

	/// Boolean search
	///
	/// Allows Boolean operators (+, -, *, etc.) in the search query.
	/// More powerful but requires user knowledge of operators.
	Boolean,

	/// Phrase search
	///
	/// Searches for exact phrase matches.
	/// Fastest but least flexible.
	Phrase,

	/// Query expansion
	///
	/// Performs search using query expansion (similar terms).
	/// Useful for finding related content.
	QueryExpansion,
}

impl Default for FullTextSearchMode {
	fn default() -> Self {
		FullTextSearchMode::Natural
	}
}

/// Full-text search filter
///
/// Provides full-text search capabilities for text fields.
///
/// # Type Parameters
///
/// * `M` - The model type being searched
///
/// # Examples
///
/// ```
/// use reinhardt_filters::FullTextSearchFilter;
///
/// #[derive(Clone)]
/// struct Article {
///     id: i64,
///     title: String,
///     content: String,
/// }
///
/// let filter: FullTextSearchFilter<Article> = FullTextSearchFilter::new()
///     .query("rust programming")
///     .add_field("title")
///     .add_field("content");
/// ```
#[derive(Debug, Clone)]
pub struct FullTextSearchFilter<M> {
	/// The search query
	pub query: String,
	/// Fields to search in
	pub fields: Vec<String>,
	/// Search mode
	pub mode: FullTextSearchMode,
	/// Minimum relevance score (0.0 - 1.0)
	pub min_score: Option<f64>,
	/// Language for stemming (e.g., "english", "spanish")
	pub language: Option<String>,
	/// Boost factors for each field
	pub boosts: Vec<f64>,
	_phantom: PhantomData<M>,
}

impl<M> FullTextSearchFilter<M> {
	/// Create a new full-text search filter
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_filters::FullTextSearchFilter;
	///
	/// #[derive(Clone)]
	/// struct Article {
	///     id: i64,
	/// }
	///
	/// let filter: FullTextSearchFilter<Article> = FullTextSearchFilter::new();
	/// ```
	pub fn new() -> Self {
		Self {
			query: String::new(),
			fields: Vec::new(),
			mode: FullTextSearchMode::default(),
			min_score: None,
			language: None,
			boosts: Vec::new(),
			_phantom: PhantomData,
		}
	}

	/// Set the search query
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_filters::FullTextSearchFilter;
	///
	/// #[derive(Clone)]
	/// struct Article {
	///     id: i64,
	/// }
	///
	/// let filter: FullTextSearchFilter<Article> = FullTextSearchFilter::new()
	///     .query("rust programming");
	/// assert_eq!(filter.get_query(), "rust programming");
	/// ```
	pub fn query(mut self, query: impl Into<String>) -> Self {
		self.query = query.into();
		self
	}

	/// Add a field to search in
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_filters::FullTextSearchFilter;
	///
	/// #[derive(Clone)]
	/// struct Article {
	///     id: i64,
	/// }
	///
	/// let filter: FullTextSearchFilter<Article> = FullTextSearchFilter::new()
	///     .add_field("title")
	///     .add_field("content");
	/// assert_eq!(filter.get_fields().len(), 2);
	/// ```
	pub fn add_field(mut self, field: impl Into<String>) -> Self {
		self.fields.push(field.into());
		self.boosts.push(1.0);
		self
	}

	/// Add a field with a boost factor
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_filters::FullTextSearchFilter;
	///
	/// #[derive(Clone)]
	/// struct Article {
	///     id: i64,
	/// }
	///
	/// let filter: FullTextSearchFilter<Article> = FullTextSearchFilter::new()
	///     .add_field_with_boost("title", 2.0)
	///     .add_field_with_boost("content", 1.0);
	/// ```
	pub fn add_field_with_boost(mut self, field: impl Into<String>, boost: f64) -> Self {
		self.fields.push(field.into());
		self.boosts.push(boost);
		self
	}

	/// Set the search mode
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_filters::{FullTextSearchFilter, FullTextSearchMode};
	///
	/// #[derive(Clone)]
	/// struct Article {
	///     id: i64,
	/// }
	///
	/// let filter: FullTextSearchFilter<Article> = FullTextSearchFilter::new()
	///     .mode(FullTextSearchMode::Boolean);
	/// assert_eq!(filter.get_mode(), FullTextSearchMode::Boolean);
	/// ```
	pub fn mode(mut self, mode: FullTextSearchMode) -> Self {
		self.mode = mode;
		self
	}

	/// Set the minimum relevance score
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_filters::FullTextSearchFilter;
	///
	/// #[derive(Clone)]
	/// struct Article {
	///     id: i64,
	/// }
	///
	/// let filter: FullTextSearchFilter<Article> = FullTextSearchFilter::new()
	///     .min_score(0.5);
	/// assert_eq!(filter.get_min_score(), Some(0.5));
	/// ```
	pub fn min_score(mut self, score: f64) -> Self {
		self.min_score = Some(score);
		self
	}

	/// Set the language for stemming
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_filters::FullTextSearchFilter;
	///
	/// #[derive(Clone)]
	/// struct Article {
	///     id: i64,
	/// }
	///
	/// let filter: FullTextSearchFilter<Article> = FullTextSearchFilter::new()
	///     .language("english");
	/// assert_eq!(filter.get_language(), Some("english"));
	/// ```
	pub fn language(mut self, language: impl Into<String>) -> Self {
		self.language = Some(language.into());
		self
	}

	/// Get the search query
	pub fn get_query(&self) -> &str {
		&self.query
	}

	/// Get the fields to search in
	pub fn get_fields(&self) -> &[String] {
		&self.fields
	}

	/// Get the search mode
	pub fn get_mode(&self) -> FullTextSearchMode {
		self.mode
	}

	/// Get the minimum score
	pub fn get_min_score(&self) -> Option<f64> {
		self.min_score
	}

	/// Get the language
	pub fn get_language(&self) -> Option<&str> {
		self.language.as_deref()
	}

	/// Get the boost factors
	pub fn get_boosts(&self) -> &[f64] {
		&self.boosts
	}

	/// Get the boost for a specific field
	pub fn get_boost(&self, index: usize) -> Option<f64> {
		self.boosts.get(index).copied()
	}
}

impl<M> Default for FullTextSearchFilter<M> {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Clone)]
	struct Article {
		id: i64,
		title: String,
		content: String,
	}

	#[test]
	fn test_search_mode_default() {
		assert_eq!(FullTextSearchMode::default(), FullTextSearchMode::Natural);
	}

	#[test]
	fn test_search_modes() {
		let modes = vec![
			FullTextSearchMode::Natural,
			FullTextSearchMode::Boolean,
			FullTextSearchMode::Phrase,
			FullTextSearchMode::QueryExpansion,
		];

		for mode in modes {
			assert_eq!(mode, mode);
		}
	}

	#[test]
	fn test_filter_creation() {
		let filter: FullTextSearchFilter<Article> = FullTextSearchFilter::new();
		assert_eq!(filter.get_query(), "");
		assert_eq!(filter.get_fields().len(), 0);
		assert_eq!(filter.get_mode(), FullTextSearchMode::Natural);
		assert_eq!(filter.get_min_score(), None);
		assert_eq!(filter.get_language(), None);
	}

	#[test]
	fn test_filter_query() {
		let filter: FullTextSearchFilter<Article> =
			FullTextSearchFilter::new().query("rust programming");
		assert_eq!(filter.get_query(), "rust programming");
	}

	#[test]
	fn test_filter_add_field() {
		let filter: FullTextSearchFilter<Article> = FullTextSearchFilter::new()
			.add_field("title")
			.add_field("content");

		assert_eq!(filter.get_fields().len(), 2);
		assert_eq!(filter.get_fields()[0], "title");
		assert_eq!(filter.get_fields()[1], "content");
	}

	#[test]
	fn test_filter_add_field_with_boost() {
		let filter: FullTextSearchFilter<Article> = FullTextSearchFilter::new()
			.add_field_with_boost("title", 2.0)
			.add_field_with_boost("content", 1.0);

		assert_eq!(filter.get_fields().len(), 2);
		assert_eq!(filter.get_boost(0), Some(2.0));
		assert_eq!(filter.get_boost(1), Some(1.0));
	}

	#[test]
	fn test_filter_mode() {
		let filter: FullTextSearchFilter<Article> =
			FullTextSearchFilter::new().mode(FullTextSearchMode::Boolean);
		assert_eq!(filter.get_mode(), FullTextSearchMode::Boolean);

		let filter2: FullTextSearchFilter<Article> =
			FullTextSearchFilter::new().mode(FullTextSearchMode::Phrase);
		assert_eq!(filter2.get_mode(), FullTextSearchMode::Phrase);
	}

	#[test]
	fn test_filter_min_score() {
		let filter: FullTextSearchFilter<Article> = FullTextSearchFilter::new().min_score(0.75);
		assert_eq!(filter.get_min_score(), Some(0.75));
	}

	#[test]
	fn test_filter_language() {
		let filter: FullTextSearchFilter<Article> = FullTextSearchFilter::new().language("english");
		assert_eq!(filter.get_language(), Some("english"));

		let filter2: FullTextSearchFilter<Article> =
			FullTextSearchFilter::new().language("spanish");
		assert_eq!(filter2.get_language(), Some("spanish"));
	}

	#[test]
	fn test_filter_complex() {
		let filter: FullTextSearchFilter<Article> = FullTextSearchFilter::new()
			.query("rust web framework")
			.add_field_with_boost("title", 2.0)
			.add_field_with_boost("content", 1.0)
			.mode(FullTextSearchMode::Boolean)
			.min_score(0.6)
			.language("english");

		assert_eq!(filter.get_query(), "rust web framework");
		assert_eq!(filter.get_fields().len(), 2);
		assert_eq!(filter.get_mode(), FullTextSearchMode::Boolean);
		assert_eq!(filter.get_min_score(), Some(0.6));
		assert_eq!(filter.get_language(), Some("english"));
		assert_eq!(filter.get_boost(0), Some(2.0));
		assert_eq!(filter.get_boost(1), Some(1.0));
	}

	#[test]
	fn test_default_boosts() {
		let filter: FullTextSearchFilter<Article> = FullTextSearchFilter::new()
			.add_field("title")
			.add_field("content");

		assert_eq!(filter.get_boost(0), Some(1.0));
		assert_eq!(filter.get_boost(1), Some(1.0));
	}
}
