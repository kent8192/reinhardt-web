//! Multi-term search with type-safe fields
//!
//! Provides utilities for building queries that search multiple terms across
//! multiple fields, combining them with AND/OR logic.

use super::searchable::SearchableModel;
use reinhardt_db::orm::{Field, Lookup, Model};

/// Combines multiple search terms across multiple fields
///
/// Search for posts containing "rust" AND "programming".
/// This creates: `(title ICONTAINS 'rust' OR content ICONTAINS 'rust') AND (title ICONTAINS 'programming' OR content ICONTAINS 'programming')`
///
/// # Examples
///
/// ```rust
/// # use reinhardt_rest::filters::{MultiTermSearch, SearchableModel};
/// # use reinhardt_db::orm::{Field, FieldSelector, Model};
/// #
/// # #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// # struct Post {
/// #     id: i64,
/// #     title: String,
/// #     content: String,
/// # }
/// #
/// # #[derive(Clone)]
/// # struct PostFields;
/// # impl FieldSelector for PostFields {
/// #     fn with_alias(self, _alias: &str) -> Self { self }
/// # }
/// #
/// # impl Model for Post {
/// #     type PrimaryKey = i64;
/// #     type Fields = PostFields;
/// #     fn table_name() -> &'static str { "posts" }
/// #     fn new_fields() -> Self::Fields { PostFields }
/// #     fn primary_key(&self) -> Option<Self::PrimaryKey> { Some(self.id) }
/// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = value; }
/// # }
/// #
/// # impl SearchableModel for Post {
/// #     fn searchable_fields() -> Vec<Field<Self, String>> {
/// #         vec![
/// #             Field::<Post, String>::new(vec!["title"]),
/// #             Field::<Post, String>::new(vec!["content"]),
/// #         ]
/// #     }
/// # }
/// let terms = vec!["rust", "programming"];
/// let lookups = MultiTermSearch::search_terms::<Post>(terms);
/// assert_eq!(lookups.len(), 2); // Two terms
/// assert_eq!(lookups[0].len(), 2); // Two fields per term
/// ```
pub struct MultiTermSearch;

/// Search term representation with advanced features
#[derive(Debug, Clone, PartialEq)]
pub struct SearchTerm {
	pub value: String,
	pub term_type: TermType,
	pub field: Option<String>,
	pub operator: Operator,
}

/// Term type for different search patterns
#[derive(Debug, Clone, PartialEq)]
pub enum TermType {
	Word,
	Phrase,
	Wildcard,
	FieldValue,
}

/// Boolean operators for combining terms
#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
	And,
	Or,
	Not,
}

impl MultiTermSearch {
	/// Create lookups for searching multiple terms across searchable fields
	///
	/// For each term, creates an OR clause across all searchable fields,
	/// then combines all terms with AND.
	pub fn search_terms<M: SearchableModel>(terms: Vec<&str>) -> Vec<Vec<Lookup<M>>> {
		let fields = M::searchable_fields();

		terms
			.into_iter()
			.map(|term| {
				// For each term, create lookups for all searchable fields
				fields
					.iter()
					.map(|field| {
						// Create a new field with same path and create an icontains lookup
						let new_field = Field::<M, String>::new(field.path().to_vec());
						new_field.icontains(term)
					})
					.collect()
			})
			.collect()
	}

	/// Create lookups for exact match search across multiple terms
	pub fn exact_terms<M: SearchableModel>(terms: Vec<&str>) -> Vec<Vec<Lookup<M>>> {
		let fields = M::searchable_fields();

		terms
			.into_iter()
			.map(|term| {
				fields
					.iter()
					.map(|field| {
						let new_field = Field::<M, String>::new(field.path().to_vec());
						new_field.iexact(term.to_string())
					})
					.collect()
			})
			.collect()
	}

	/// Create lookups for prefix search (startswith) across multiple terms
	pub fn prefix_terms<M: SearchableModel>(terms: Vec<&str>) -> Vec<Vec<Lookup<M>>> {
		let fields = M::searchable_fields();

		terms
			.into_iter()
			.map(|term| {
				fields
					.iter()
					.map(|field| {
						let new_field = Field::<M, String>::new(field.path().to_vec());
						new_field.startswith(term)
					})
					.collect()
			})
			.collect()
	}

	/// Parse a comma-separated search string into individual terms
	///
	/// Handles quoted strings properly, keeping quoted content together.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::filters::multi_term::MultiTermSearch;
	///
	/// let terms = MultiTermSearch::parse_search_terms("rust, programming");
	/// assert_eq!(terms, vec!["rust", "programming"]);
	///
	/// let terms = MultiTermSearch::parse_search_terms("\"hello world\", rust");
	/// assert_eq!(terms, vec!["hello world", "rust"]);
	/// ```
	pub fn parse_search_terms(search: &str) -> Vec<String> {
		let mut terms = Vec::new();
		let mut current_term = String::new();
		let mut in_quotes = false;
		let chars = search.chars().peekable();

		for c in chars {
			match c {
				'"' => {
					in_quotes = !in_quotes;
				}
				',' if !in_quotes => {
					let trimmed = current_term.trim().to_string();
					if !trimmed.is_empty() {
						terms.push(trimmed);
					}
					current_term.clear();
				}
				_ => {
					current_term.push(c);
				}
			}
		}

		// Don't forget the last term
		let trimmed = current_term.trim().to_string();
		if !trimmed.is_empty() {
			terms.push(trimmed);
		}

		terms
	}

	/// Compile multi-term lookups into SQL WHERE clause
	///
	/// Terms are combined with AND, fields within each term are combined with OR.
	///
	/// # Examples
	///
	/// For terms ["rust", "web"] across fields [title, content]:
	/// ```sql
	/// ((title ILIKE '%rust%' OR content ILIKE '%rust%')
	///  AND
	///  (title ILIKE '%web%' OR content ILIKE '%web%'))
	/// ```
	pub fn compile_to_sql<M: Model>(term_lookups: Vec<Vec<Lookup<M>>>) -> Option<String> {
		if term_lookups.is_empty() {
			return None;
		}

		use reinhardt_db::orm::QueryFieldCompiler;

		let term_clauses: Vec<String> = term_lookups
			.into_iter()
			.filter(|lookups| !lookups.is_empty())
			.map(|lookups| {
				// Each term: OR across all fields
				let field_conditions: Vec<String> = lookups
					.iter()
					.map(|lookup| QueryFieldCompiler::compile(lookup))
					.collect();

				if field_conditions.len() == 1 {
					field_conditions[0].clone()
				} else {
					format!("({})", field_conditions.join(" OR "))
				}
			})
			.collect();

		if term_clauses.is_empty() {
			return None;
		}

		if term_clauses.len() == 1 {
			Some(term_clauses[0].clone())
		} else {
			// All terms: AND together
			Some(format!("({})", term_clauses.join(" AND ")))
		}
	}

	/// Parse search query into structured terms with advanced operators
	///
	/// Supports:
	/// - Quoted phrases (`"exact phrase"`)
	/// - Boolean operators (`AND`, `OR`, `NOT`)
	/// - Field-specific search (`field:value`)
	/// - Wildcard search (`term*`)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_rest::filters::multi_term::MultiTermSearch;
	///
	/// let query = r#"title:"machine learning" AND author:Smith"#;
	/// let terms = MultiTermSearch::parse_query(query);
	/// assert_eq!(terms.len(), 2);
	/// ```
	pub fn parse_query(query: &str) -> Vec<SearchTerm> {
		let mut terms = Vec::new();
		let mut current_term = String::new();
		let mut in_quotes = false;
		let mut chars = query.chars().peekable();
		let mut current_operator = Operator::And;

		while let Some(ch) = chars.next() {
			match ch {
				'"' => {
					in_quotes = !in_quotes;
					if !in_quotes && !current_term.is_empty() {
						// End of quoted phrase
						terms.push(SearchTerm {
							value: current_term.clone(),
							term_type: TermType::Phrase,
							field: None,
							operator: current_operator.clone(),
						});
						current_term.clear();
					}
				}
				' ' if !in_quotes => {
					if !current_term.is_empty() {
						// Check for operators
						match current_term.to_uppercase().as_str() {
							"AND" => {
								current_operator = Operator::And;
							}
							"OR" => {
								current_operator = Operator::Or;
							}
							"NOT" => {
								current_operator = Operator::Not;
							}
							_ => {
								terms.push(Self::parse_single_term(
									&current_term,
									current_operator.clone(),
								));
							}
						}
						current_term.clear();
					}
				}
				':' if !in_quotes => {
					// Field-specific search
					let field = current_term.clone();
					current_term.clear();

					// Check if value starts with quote
					let mut field_in_quotes = false;
					if let Some(&'"') = chars.peek() {
						chars.next(); // Consume opening quote
						field_in_quotes = true;
					}

					// Get the value after ':'
					for next_ch in chars.by_ref() {
						if field_in_quotes {
							if next_ch == '"' {
								// End of quoted value
								break;
							} else {
								current_term.push(next_ch);
							}
						} else if next_ch == ' ' {
							// End of unquoted value
							break;
						} else {
							current_term.push(next_ch);
						}
					}

					terms.push(SearchTerm {
						value: current_term.clone(),
						term_type: TermType::FieldValue,
						field: Some(field),
						operator: current_operator.clone(),
					});
					current_term.clear();
				}
				_ => {
					current_term.push(ch);
				}
			}
		}

		// Handle remaining term
		if !current_term.is_empty() {
			if in_quotes {
				terms.push(SearchTerm {
					value: current_term,
					term_type: TermType::Phrase,
					field: None,
					operator: current_operator,
				});
			} else {
				match current_term.to_uppercase().as_str() {
					"AND" | "OR" | "NOT" => {
						// Skip trailing operators
					}
					_ => {
						terms.push(Self::parse_single_term(&current_term, current_operator));
					}
				}
			}
		}

		terms
	}

	/// Parse single term with wildcard detection
	fn parse_single_term(term: &str, operator: Operator) -> SearchTerm {
		let term_type = if term.ends_with('*') {
			TermType::Wildcard
		} else {
			TermType::Word
		};

		SearchTerm {
			value: term.to_string(),
			term_type,
			field: None,
			operator,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_db::orm::Field;

	#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
	struct TestPost {
		id: Option<i64>,
		title: String,
		content: String,
	}

	reinhardt_test::impl_test_model!(TestPost, i64, "test_posts");

	impl SearchableModel for TestPost {
		fn searchable_fields() -> Vec<Field<Self, String>> {
			vec![
				Field::<TestPost, String>::new(vec!["title"]),
				Field::<TestPost, String>::new(vec!["content"]),
			]
		}
	}

	#[test]
	fn test_search_single_term() {
		let terms = vec!["rust"];
		let lookups = MultiTermSearch::search_terms::<TestPost>(terms);

		assert_eq!(lookups.len(), 1); // One term
		assert_eq!(lookups[0].len(), 2); // Two fields (title, content)
	}

	#[test]
	fn test_search_multiple_terms() {
		let terms = vec!["rust", "programming"];
		let lookups = MultiTermSearch::search_terms::<TestPost>(terms);

		assert_eq!(lookups.len(), 2); // Two terms
		assert_eq!(lookups[0].len(), 2); // Each term searches 2 fields
		assert_eq!(lookups[1].len(), 2);
	}

	#[test]
	fn test_exact_terms() {
		let terms = vec!["rust"];
		let lookups = MultiTermSearch::exact_terms::<TestPost>(terms);

		assert_eq!(lookups.len(), 1);
		assert_eq!(lookups[0].len(), 2);
	}

	#[test]
	fn test_prefix_terms() {
		let terms = vec!["rust"];
		let lookups = MultiTermSearch::prefix_terms::<TestPost>(terms);

		assert_eq!(lookups.len(), 1);
		assert_eq!(lookups[0].len(), 2);
	}

	#[test]
	fn test_compile_single_term_to_sql() {
		let terms = vec!["rust"];
		let lookups = MultiTermSearch::search_terms::<TestPost>(terms);
		let sql = MultiTermSearch::compile_to_sql(lookups).unwrap();

		assert!(sql.contains("title"));
		assert!(sql.contains("content"));
		assert!(sql.contains("OR"));
		// SQLite uses LIKE with LOWER() for case-insensitive
		assert!(sql.contains("LIKE"));
		assert!(sql.contains("LOWER"));
	}

	#[test]
	fn test_compile_multiple_terms_to_sql() {
		let terms = vec!["rust", "web"];
		let lookups = MultiTermSearch::search_terms::<TestPost>(terms);
		let sql = MultiTermSearch::compile_to_sql(lookups).unwrap();

		assert!(sql.contains("title"));
		assert!(sql.contains("content"));
		assert!(sql.contains("OR"));
		assert!(sql.contains("AND"));
		// SQLite uses LIKE with LOWER() for case-insensitive
		assert!(sql.contains("LIKE"));
		assert!(sql.contains("LOWER"));
	}

	#[test]
	fn test_compile_empty_terms() {
		let lookups: Vec<Vec<Lookup<TestPost>>> = vec![];
		let sql = MultiTermSearch::compile_to_sql(lookups);

		assert!(sql.is_none());
	}

	#[test]
	fn test_parse_simple_terms() {
		let input = "rust, programming";
		let terms = MultiTermSearch::parse_search_terms(input);

		assert_eq!(terms.len(), 2);
	}

	#[test]
	fn test_parse_quoted_terms() {
		let input = "\"hello world\", rust";
		let terms = MultiTermSearch::parse_search_terms(input);

		assert_eq!(terms.len(), 2);
		assert_eq!(terms[0], "hello world");
		assert_eq!(terms[1], "rust");
	}

	#[test]
	fn test_parse_empty_string() {
		let input = "";
		let terms = MultiTermSearch::parse_search_terms(input);

		assert_eq!(terms.len(), 0);
	}

	#[test]
	fn test_parse_query_simple_terms() {
		let query = "hello world";
		let terms = MultiTermSearch::parse_query(query);

		assert_eq!(terms.len(), 2);
		assert_eq!(terms[0].value, "hello");
		assert_eq!(terms[0].term_type, TermType::Word);
		assert_eq!(terms[1].value, "world");
		assert_eq!(terms[1].term_type, TermType::Word);
	}

	#[test]
	fn test_parse_query_quoted_phrase() {
		let query = r#""exact phrase""#;
		let terms = MultiTermSearch::parse_query(query);

		assert_eq!(terms.len(), 1);
		assert_eq!(terms[0].value, "exact phrase");
		assert_eq!(terms[0].term_type, TermType::Phrase);
	}

	#[test]
	fn test_parse_query_field_search() {
		let query = "title:rust";
		let terms = MultiTermSearch::parse_query(query);

		assert_eq!(terms.len(), 1);
		assert_eq!(terms[0].value, "rust");
		assert_eq!(terms[0].field, Some("title".to_string()));
		assert_eq!(terms[0].term_type, TermType::FieldValue);
	}

	#[test]
	fn test_parse_query_field_with_quoted_value() {
		let query = r#"title:"machine learning""#;
		let terms = MultiTermSearch::parse_query(query);

		assert_eq!(terms.len(), 1);
		assert_eq!(terms[0].value, "machine learning");
		assert_eq!(terms[0].field, Some("title".to_string()));
		assert_eq!(terms[0].term_type, TermType::FieldValue);
	}

	#[test]
	fn test_parse_query_wildcard() {
		let query = "rust*";
		let terms = MultiTermSearch::parse_query(query);

		assert_eq!(terms.len(), 1);
		assert_eq!(terms[0].value, "rust*");
		assert_eq!(terms[0].term_type, TermType::Wildcard);
	}

	#[test]
	fn test_parse_query_with_and_operator() {
		let query = "rust AND web";
		let terms = MultiTermSearch::parse_query(query);

		assert_eq!(terms.len(), 2);
		assert_eq!(terms[0].value, "rust");
		assert_eq!(terms[0].operator, Operator::And);
		assert_eq!(terms[1].value, "web");
		assert_eq!(terms[1].operator, Operator::And);
	}

	#[test]
	fn test_parse_query_with_or_operator() {
		let query = "rust OR python";
		let terms = MultiTermSearch::parse_query(query);

		assert_eq!(terms.len(), 2);
		assert_eq!(terms[0].value, "rust");
		assert_eq!(terms[1].value, "python");
		assert_eq!(terms[1].operator, Operator::Or);
	}

	#[test]
	fn test_parse_query_complex() {
		let query = r#"title:"machine learning" AND author:Smith OR year:2024"#;
		let terms = MultiTermSearch::parse_query(query);

		assert_eq!(terms.len(), 3);

		// First term: title:"machine learning"
		assert_eq!(terms[0].value, "machine learning");
		assert_eq!(terms[0].field, Some("title".to_string()));
		assert_eq!(terms[0].term_type, TermType::FieldValue);

		// Second term: author:Smith (with AND)
		assert_eq!(terms[1].value, "Smith");
		assert_eq!(terms[1].field, Some("author".to_string()));
		assert_eq!(terms[1].operator, Operator::And);

		// Third term: year:2024 (with OR)
		assert_eq!(terms[2].value, "2024");
		assert_eq!(terms[2].field, Some("year".to_string()));
		assert_eq!(terms[2].operator, Operator::Or);
	}

	#[test]
	fn test_parse_query_multiple_phrases() {
		let query = r#""hello world" AND "rust programming""#;
		let terms = MultiTermSearch::parse_query(query);

		assert_eq!(terms.len(), 2);
		assert_eq!(terms[0].value, "hello world");
		assert_eq!(terms[0].term_type, TermType::Phrase);
		assert_eq!(terms[1].value, "rust programming");
		assert_eq!(terms[1].term_type, TermType::Phrase);
		assert_eq!(terms[1].operator, Operator::And);
	}

	#[test]
	fn test_parse_single_term() {
		let term = MultiTermSearch::parse_single_term("test", Operator::And);
		assert_eq!(term.value, "test");
		assert_eq!(term.term_type, TermType::Word);
		assert_eq!(term.operator, Operator::And);

		let wildcard = MultiTermSearch::parse_single_term("test*", Operator::Or);
		assert_eq!(wildcard.term_type, TermType::Wildcard);
	}
}
