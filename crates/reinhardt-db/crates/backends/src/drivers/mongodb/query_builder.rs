//! MongoDB query builder
//!
//! This module provides a builder for constructing MongoDB queries using BSON documents.
//!
//! # Example
//!
//! ```rust
//! # use bson::{Document, doc};
//! # #[derive(Debug, Clone)]
//! # pub struct MongoDBQueryBuilder {
//! #     collection: String,
//! #     filter: Document,
//! #     sort: Option<Document>,
//! #     limit: Option<i64>,
//! #     skip: Option<u64>,
//! #     projection: Option<Document>,
//! # }
//! # impl MongoDBQueryBuilder {
//! #     pub fn new(collection: &str) -> Self {
//! #         Self {
//! #             collection: collection.to_string(),
//! #             filter: Document::new(),
//! #             sort: None,
//! #             limit: None,
//! #             skip: None,
//! #             projection: None,
//! #         }
//! #     }
//! #     pub fn filter(mut self, filter: Document) -> Self {
//! #         self.filter = filter;
//! #         self
//! #     }
//! #     pub fn sort(mut self, sort: Document) -> Self {
//! #         self.sort = Some(sort);
//! #         self
//! #     }
//! #     pub fn limit(mut self, limit: i64) -> Self {
//! #         self.limit = Some(limit);
//! #         self
//! #     }
//! #     pub fn build_filter(&self) -> Document {
//! #         self.filter.clone()
//! #     }
//! #     pub fn collection_name(&self) -> &str {
//! #         &self.collection
//! #     }
//! #     pub fn get_limit(&self) -> Option<i64> {
//! #         self.limit
//! #     }
//! # }
//! let builder = MongoDBQueryBuilder::new("users")
//!     .filter(doc! { "age": { "$gte": 18 } })
//!     .sort(doc! { "name": 1 })
//!     .limit(10);
//! assert_eq!(builder.collection_name(), "users");
//! assert_eq!(builder.get_limit(), Some(10));
//! let filter = builder.build_filter();
//! assert!(filter.contains_key("age"));
//! ```

use bson::{Document, doc};

/// MongoDB query builder for constructing BSON queries
///
/// # Example
///
/// ```rust
/// # use bson::{Document, doc};
/// # #[derive(Debug, Clone)]
/// # pub struct MongoDBQueryBuilder {
/// #     collection: String,
/// #     filter: Document,
/// #     sort: Option<Document>,
/// #     limit: Option<i64>,
/// #     skip: Option<u64>,
/// #     projection: Option<Document>,
/// # }
/// # impl MongoDBQueryBuilder {
/// #     pub fn new(collection: &str) -> Self {
/// #         Self {
/// #             collection: collection.to_string(),
/// #             filter: Document::new(),
/// #             sort: None,
/// #             limit: None,
/// #             skip: None,
/// #             projection: None,
/// #         }
/// #     }
/// #     pub fn filter(mut self, filter: Document) -> Self {
/// #         self.filter = filter;
/// #         self
/// #     }
/// #     pub fn limit(mut self, limit: i64) -> Self {
/// #         self.limit = Some(limit);
/// #         self
/// #     }
/// #     pub fn skip(mut self, skip: u64) -> Self {
/// #         self.skip = Some(skip);
/// #         self
/// #     }
/// # }
/// let builder = MongoDBQueryBuilder::new("users")
///     .filter(doc! { "active": true })
///     .limit(10)
///     .skip(0);
/// assert_eq!(builder.collection.as_str(), "users");
/// assert!(builder.filter.contains_key("active"));
/// assert_eq!(builder.limit, Some(10));
/// assert_eq!(builder.skip, Some(0));
/// ```
#[derive(Debug, Clone)]
pub struct MongoDBQueryBuilder {
	collection: String,
	filter: Document,
	sort: Option<Document>,
	limit: Option<i64>,
	skip: Option<u64>,
	projection: Option<Document>,
}

impl MongoDBQueryBuilder {
	/// Create a new query builder for a collection
	///
	/// # Example
	///
	/// ```rust
	/// # use bson::Document;
	/// # #[derive(Debug, Clone)]
	/// # pub struct MongoDBQueryBuilder {
	/// #     collection: String,
	/// #     filter: Document,
	/// #     sort: Option<Document>,
	/// #     limit: Option<i64>,
	/// #     skip: Option<u64>,
	/// #     projection: Option<Document>,
	/// # }
	/// # impl MongoDBQueryBuilder {
	/// #     pub fn new(collection: &str) -> Self {
	/// #         Self {
	/// #             collection: collection.to_string(),
	/// #             filter: Document::new(),
	/// #             sort: None,
	/// #             limit: None,
	/// #             skip: None,
	/// #             projection: None,
	/// #         }
	/// #     }
	/// #     pub fn collection_name(&self) -> &str {
	/// #         &self.collection
	/// #     }
	/// # }
	/// let builder = MongoDBQueryBuilder::new("users");
	/// assert_eq!(builder.collection_name(), "users");
	/// ```
	pub fn new(collection: &str) -> Self {
		Self {
			collection: collection.to_string(),
			filter: Document::new(),
			sort: None,
			limit: None,
			skip: None,
			projection: None,
		}
	}

	/// Set the filter document
	///
	/// # Example
	///
	/// ```rust
	/// # use bson::{Document, doc};
	/// # #[derive(Debug, Clone)]
	/// # pub struct MongoDBQueryBuilder {
	/// #     collection: String,
	/// #     filter: Document,
	/// #     sort: Option<Document>,
	/// #     limit: Option<i64>,
	/// #     skip: Option<u64>,
	/// #     projection: Option<Document>,
	/// # }
	/// # impl MongoDBQueryBuilder {
	/// #     pub fn new(collection: &str) -> Self {
	/// #         Self {
	/// #             collection: collection.to_string(),
	/// #             filter: Document::new(),
	/// #             sort: None,
	/// #             limit: None,
	/// #             skip: None,
	/// #             projection: None,
	/// #         }
	/// #     }
	/// #     pub fn filter(mut self, filter: Document) -> Self {
	/// #         self.filter = filter;
	/// #         self
	/// #     }
	/// # }
	/// let builder = MongoDBQueryBuilder::new("users")
	///     .filter(doc! { "age": { "$gte": 18 } });
	/// assert!(builder.filter.contains_key("age"));
	/// ```
	pub fn filter(mut self, filter: Document) -> Self {
		self.filter = filter;
		self
	}

	/// Set the sort document
	///
	/// # Example
	///
	/// ```rust
	/// # use bson::{Document, doc};
	/// # #[derive(Debug, Clone)]
	/// # pub struct MongoDBQueryBuilder {
	/// #     collection: String,
	/// #     filter: Document,
	/// #     sort: Option<Document>,
	/// #     limit: Option<i64>,
	/// #     skip: Option<u64>,
	/// #     projection: Option<Document>,
	/// # }
	/// # impl MongoDBQueryBuilder {
	/// #     pub fn new(collection: &str) -> Self {
	/// #         Self {
	/// #             collection: collection.to_string(),
	/// #             filter: Document::new(),
	/// #             sort: None,
	/// #             limit: None,
	/// #             skip: None,
	/// #             projection: None,
	/// #         }
	/// #     }
	/// #     pub fn sort(mut self, sort: Document) -> Self {
	/// #         self.sort = Some(sort);
	/// #         self
	/// #     }
	/// # }
	/// let builder = MongoDBQueryBuilder::new("users")
	///     .sort(doc! { "name": 1, "age": -1 });
	/// assert!(builder.sort.is_some());
	/// let sort = builder.sort.unwrap();
	/// assert!(sort.contains_key("name"));
	/// assert!(sort.contains_key("age"));
	/// ```
	pub fn sort(mut self, sort: Document) -> Self {
		self.sort = Some(sort);
		self
	}

	/// Set the limit
	///
	/// # Example
	///
	/// ```rust
	/// # use bson::Document;
	/// # #[derive(Debug, Clone)]
	/// # pub struct MongoDBQueryBuilder {
	/// #     collection: String,
	/// #     filter: Document,
	/// #     sort: Option<Document>,
	/// #     limit: Option<i64>,
	/// #     skip: Option<u64>,
	/// #     projection: Option<Document>,
	/// # }
	/// # impl MongoDBQueryBuilder {
	/// #     pub fn new(collection: &str) -> Self {
	/// #         Self {
	/// #             collection: collection.to_string(),
	/// #             filter: Document::new(),
	/// #             sort: None,
	/// #             limit: None,
	/// #             skip: None,
	/// #             projection: None,
	/// #         }
	/// #     }
	/// #     pub fn limit(mut self, limit: i64) -> Self {
	/// #         self.limit = Some(limit);
	/// #         self
	/// #     }
	/// # }
	/// let builder = MongoDBQueryBuilder::new("users")
	///     .limit(10);
	/// assert_eq!(builder.limit, Some(10));
	/// ```
	pub fn limit(mut self, limit: i64) -> Self {
		self.limit = Some(limit);
		self
	}

	/// Set the skip offset
	///
	/// # Example
	///
	/// ```rust
	/// # use bson::Document;
	/// # #[derive(Debug, Clone)]
	/// # pub struct MongoDBQueryBuilder {
	/// #     collection: String,
	/// #     filter: Document,
	/// #     sort: Option<Document>,
	/// #     limit: Option<i64>,
	/// #     skip: Option<u64>,
	/// #     projection: Option<Document>,
	/// # }
	/// # impl MongoDBQueryBuilder {
	/// #     pub fn new(collection: &str) -> Self {
	/// #         Self {
	/// #             collection: collection.to_string(),
	/// #             filter: Document::new(),
	/// #             sort: None,
	/// #             limit: None,
	/// #             skip: None,
	/// #             projection: None,
	/// #         }
	/// #     }
	/// #     pub fn skip(mut self, skip: u64) -> Self {
	/// #         self.skip = Some(skip);
	/// #         self
	/// #     }
	/// # }
	/// let builder = MongoDBQueryBuilder::new("users")
	///     .skip(20);
	/// assert_eq!(builder.skip, Some(20));
	/// ```
	pub fn skip(mut self, skip: u64) -> Self {
		self.skip = Some(skip);
		self
	}

	/// Set the projection document
	///
	/// # Example
	///
	/// ```rust
	/// # use bson::{Document, doc};
	/// # #[derive(Debug, Clone)]
	/// # pub struct MongoDBQueryBuilder {
	/// #     collection: String,
	/// #     filter: Document,
	/// #     sort: Option<Document>,
	/// #     limit: Option<i64>,
	/// #     skip: Option<u64>,
	/// #     projection: Option<Document>,
	/// # }
	/// # impl MongoDBQueryBuilder {
	/// #     pub fn new(collection: &str) -> Self {
	/// #         Self {
	/// #             collection: collection.to_string(),
	/// #             filter: Document::new(),
	/// #             sort: None,
	/// #             limit: None,
	/// #             skip: None,
	/// #             projection: None,
	/// #         }
	/// #     }
	/// #     pub fn projection(mut self, projection: Document) -> Self {
	/// #         self.projection = Some(projection);
	/// #         self
	/// #     }
	/// # }
	/// let builder = MongoDBQueryBuilder::new("users")
	///     .projection(doc! { "name": 1, "email": 1, "_id": 0 });
	/// assert!(builder.projection.is_some());
	/// let projection = builder.projection.unwrap();
	/// assert!(projection.contains_key("name"));
	/// assert!(projection.contains_key("email"));
	/// assert!(projection.contains_key("_id"));
	/// ```
	pub fn projection(mut self, projection: Document) -> Self {
		self.projection = Some(projection);
		self
	}

	/// Get the collection name
	///
	/// # Example
	///
	/// ```rust
	/// # use bson::Document;
	/// # #[derive(Debug, Clone)]
	/// # pub struct MongoDBQueryBuilder {
	/// #     collection: String,
	/// #     filter: Document,
	/// #     sort: Option<Document>,
	/// #     limit: Option<i64>,
	/// #     skip: Option<u64>,
	/// #     projection: Option<Document>,
	/// # }
	/// # impl MongoDBQueryBuilder {
	/// #     pub fn new(collection: &str) -> Self {
	/// #         Self {
	/// #             collection: collection.to_string(),
	/// #             filter: Document::new(),
	/// #             sort: None,
	/// #             limit: None,
	/// #             skip: None,
	/// #             projection: None,
	/// #         }
	/// #     }
	/// #     pub fn collection_name(&self) -> &str {
	/// #         &self.collection
	/// #     }
	/// # }
	/// let builder = MongoDBQueryBuilder::new("users");
	/// assert_eq!(builder.collection_name(), "users");
	/// ```
	pub fn collection_name(&self) -> &str {
		&self.collection
	}

	/// Build the filter document
	///
	/// # Example
	///
	/// ```rust
	/// # use bson::{Document, doc};
	/// # #[derive(Debug, Clone)]
	/// # pub struct MongoDBQueryBuilder {
	/// #     collection: String,
	/// #     filter: Document,
	/// #     sort: Option<Document>,
	/// #     limit: Option<i64>,
	/// #     skip: Option<u64>,
	/// #     projection: Option<Document>,
	/// # }
	/// # impl MongoDBQueryBuilder {
	/// #     pub fn new(collection: &str) -> Self {
	/// #         Self {
	/// #             collection: collection.to_string(),
	/// #             filter: Document::new(),
	/// #             sort: None,
	/// #             limit: None,
	/// #             skip: None,
	/// #             projection: None,
	/// #         }
	/// #     }
	/// #     pub fn filter(mut self, filter: Document) -> Self {
	/// #         self.filter = filter;
	/// #         self
	/// #     }
	/// #     pub fn build_filter(&self) -> Document {
	/// #         self.filter.clone()
	/// #     }
	/// # }
	/// let builder = MongoDBQueryBuilder::new("users")
	///     .filter(doc! { "age": { "$gte": 18 } });
	/// let filter = builder.build_filter();
	/// assert!(filter.contains_key("age"));
	/// ```
	pub fn build_filter(&self) -> Document {
		self.filter.clone()
	}

	/// Build the sort document
	///
	/// # Example
	///
	/// ```rust
	/// # use bson::{Document, doc};
	/// # #[derive(Debug, Clone)]
	/// # pub struct MongoDBQueryBuilder {
	/// #     collection: String,
	/// #     filter: Document,
	/// #     sort: Option<Document>,
	/// #     limit: Option<i64>,
	/// #     skip: Option<u64>,
	/// #     projection: Option<Document>,
	/// # }
	/// # impl MongoDBQueryBuilder {
	/// #     pub fn new(collection: &str) -> Self {
	/// #         Self {
	/// #             collection: collection.to_string(),
	/// #             filter: Document::new(),
	/// #             sort: None,
	/// #             limit: None,
	/// #             skip: None,
	/// #             projection: None,
	/// #         }
	/// #     }
	/// #     pub fn sort(mut self, sort: Document) -> Self {
	/// #         self.sort = Some(sort);
	/// #         self
	/// #     }
	/// #     pub fn build_sort(&self) -> Option<Document> {
	/// #         self.sort.clone()
	/// #     }
	/// # }
	/// let builder = MongoDBQueryBuilder::new("users")
	///     .sort(doc! { "name": 1 });
	/// let sort = builder.build_sort();
	/// assert!(sort.is_some());
	/// assert!(sort.unwrap().contains_key("name"));
	/// ```
	pub fn build_sort(&self) -> Option<Document> {
		self.sort.clone()
	}

	/// Get the limit value
	///
	/// # Example
	///
	/// ```rust
	/// # use bson::Document;
	/// # #[derive(Debug, Clone)]
	/// # pub struct MongoDBQueryBuilder {
	/// #     collection: String,
	/// #     filter: Document,
	/// #     sort: Option<Document>,
	/// #     limit: Option<i64>,
	/// #     skip: Option<u64>,
	/// #     projection: Option<Document>,
	/// # }
	/// # impl MongoDBQueryBuilder {
	/// #     pub fn new(collection: &str) -> Self {
	/// #         Self {
	/// #             collection: collection.to_string(),
	/// #             filter: Document::new(),
	/// #             sort: None,
	/// #             limit: None,
	/// #             skip: None,
	/// #             projection: None,
	/// #         }
	/// #     }
	/// #     pub fn limit(mut self, limit: i64) -> Self {
	/// #         self.limit = Some(limit);
	/// #         self
	/// #     }
	/// #     pub fn get_limit(&self) -> Option<i64> {
	/// #         self.limit
	/// #     }
	/// # }
	/// let builder = MongoDBQueryBuilder::new("users")
	///     .limit(10);
	/// assert_eq!(builder.get_limit(), Some(10));
	/// ```
	pub fn get_limit(&self) -> Option<i64> {
		self.limit
	}

	/// Get the skip value
	///
	/// # Example
	///
	/// ```rust
	/// # use bson::Document;
	/// # #[derive(Debug, Clone)]
	/// # pub struct MongoDBQueryBuilder {
	/// #     collection: String,
	/// #     filter: Document,
	/// #     sort: Option<Document>,
	/// #     limit: Option<i64>,
	/// #     skip: Option<u64>,
	/// #     projection: Option<Document>,
	/// # }
	/// # impl MongoDBQueryBuilder {
	/// #     pub fn new(collection: &str) -> Self {
	/// #         Self {
	/// #             collection: collection.to_string(),
	/// #             filter: Document::new(),
	/// #             sort: None,
	/// #             limit: None,
	/// #             skip: None,
	/// #             projection: None,
	/// #         }
	/// #     }
	/// #     pub fn skip(mut self, skip: u64) -> Self {
	/// #         self.skip = Some(skip);
	/// #         self
	/// #     }
	/// #     pub fn get_skip(&self) -> Option<u64> {
	/// #         self.skip
	/// #     }
	/// # }
	/// let builder = MongoDBQueryBuilder::new("users")
	///     .skip(20);
	/// assert_eq!(builder.get_skip(), Some(20));
	/// ```
	pub fn get_skip(&self) -> Option<u64> {
		self.skip
	}

	/// Build the projection document
	///
	/// # Example
	///
	/// ```rust
	/// # use bson::{Document, doc};
	/// # #[derive(Debug, Clone)]
	/// # pub struct MongoDBQueryBuilder {
	/// #     collection: String,
	/// #     filter: Document,
	/// #     sort: Option<Document>,
	/// #     limit: Option<i64>,
	/// #     skip: Option<u64>,
	/// #     projection: Option<Document>,
	/// # }
	/// # impl MongoDBQueryBuilder {
	/// #     pub fn new(collection: &str) -> Self {
	/// #         Self {
	/// #             collection: collection.to_string(),
	/// #             filter: Document::new(),
	/// #             sort: None,
	/// #             limit: None,
	/// #             skip: None,
	/// #             projection: None,
	/// #         }
	/// #     }
	/// #     pub fn projection(mut self, projection: Document) -> Self {
	/// #         self.projection = Some(projection);
	/// #         self
	/// #     }
	/// #     pub fn build_projection(&self) -> Option<Document> {
	/// #         self.projection.clone()
	/// #     }
	/// # }
	/// let builder = MongoDBQueryBuilder::new("users")
	///     .projection(doc! { "name": 1 });
	/// let projection = builder.build_projection();
	/// assert!(projection.is_some());
	/// assert!(projection.unwrap().contains_key("name"));
	/// ```
	pub fn build_projection(&self) -> Option<Document> {
		self.projection.clone()
	}

	/// Build an aggregation pipeline
	///
	/// # Example
	///
	/// ```rust
	/// # use bson::{Document, doc};
	/// # #[derive(Debug, Clone)]
	/// # pub struct MongoDBQueryBuilder {
	/// #     collection: String,
	/// #     filter: Document,
	/// #     sort: Option<Document>,
	/// #     limit: Option<i64>,
	/// #     skip: Option<u64>,
	/// #     projection: Option<Document>,
	/// # }
	/// # impl MongoDBQueryBuilder {
	/// #     pub fn new(collection: &str) -> Self {
	/// #         Self {
	/// #             collection: collection.to_string(),
	/// #             filter: Document::new(),
	/// #             sort: None,
	/// #             limit: None,
	/// #             skip: None,
	/// #             projection: None,
	/// #         }
	/// #     }
	/// #     pub fn filter(mut self, filter: Document) -> Self {
	/// #         self.filter = filter;
	/// #         self
	/// #     }
	/// #     pub fn sort(mut self, sort: Document) -> Self {
	/// #         self.sort = Some(sort);
	/// #         self
	/// #     }
	/// #     pub fn limit(mut self, limit: i64) -> Self {
	/// #         self.limit = Some(limit);
	/// #         self
	/// #     }
	/// #     pub fn build_aggregation_pipeline(&self) -> Vec<Document> {
	/// #         let mut pipeline = Vec::new();
	/// #         if !self.filter.is_empty() {
	/// #             pipeline.push(doc! { "$match": self.filter.clone() });
	/// #         }
	/// #         if let Some(ref sort) = self.sort {
	/// #             pipeline.push(doc! { "$sort": sort.clone() });
	/// #         }
	/// #         if let Some(skip) = self.skip {
	/// #             pipeline.push(doc! { "$skip": skip as i64 });
	/// #         }
	/// #         if let Some(limit) = self.limit {
	/// #             pipeline.push(doc! { "$limit": limit });
	/// #         }
	/// #         if let Some(ref projection) = self.projection {
	/// #             pipeline.push(doc! { "$project": projection.clone() });
	/// #         }
	/// #         pipeline
	/// #     }
	/// # }
	/// let builder = MongoDBQueryBuilder::new("users")
	///     .filter(doc! { "age": { "$gte": 18 } })
	///     .sort(doc! { "name": 1 })
	///     .limit(10);
	///
	/// let pipeline = builder.build_aggregation_pipeline();
	/// assert!(!pipeline.is_empty());
	/// assert_eq!(pipeline.len(), 3); // match, sort, limit
	/// assert!(pipeline[0].contains_key("$match"));
	/// assert!(pipeline[1].contains_key("$sort"));
	/// assert!(pipeline[2].contains_key("$limit"));
	/// ```
	pub fn build_aggregation_pipeline(&self) -> Vec<Document> {
		let mut pipeline = Vec::new();

		// Add $match stage if filter is not empty
		if !self.filter.is_empty() {
			pipeline.push(doc! { "$match": self.filter.clone() });
		}

		// Add $sort stage if specified
		if let Some(ref sort) = self.sort {
			pipeline.push(doc! { "$sort": sort.clone() });
		}

		// Add $skip stage if specified
		if let Some(skip) = self.skip {
			pipeline.push(doc! { "$skip": skip as i64 });
		}

		// Add $limit stage if specified
		if let Some(limit) = self.limit {
			pipeline.push(doc! { "$limit": limit });
		}

		// Add $project stage if specified
		if let Some(ref projection) = self.projection {
			pipeline.push(doc! { "$project": projection.clone() });
		}

		pipeline
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_new_builder() {
		let builder = MongoDBQueryBuilder::new("users");
		assert_eq!(builder.collection_name(), "users");
		assert!(builder.build_filter().is_empty());
		assert!(builder.build_sort().is_none());
		assert_eq!(builder.get_limit(), None);
		assert_eq!(builder.get_skip(), None);
	}

	#[test]
	fn test_filter() {
		let builder = MongoDBQueryBuilder::new("users").filter(doc! { "age": { "$gte": 18 } });
		let filter = builder.build_filter();
		assert!(!filter.is_empty());
		assert!(filter.contains_key("age"));
	}

	#[test]
	fn test_sort() {
		let builder = MongoDBQueryBuilder::new("users").sort(doc! { "name": 1 });
		let sort = builder.build_sort();
		let sort = sort.unwrap();
		assert!(sort.contains_key("name"));
	}

	#[test]
	fn test_limit_and_skip() {
		let builder = MongoDBQueryBuilder::new("users").limit(10).skip(20);
		assert_eq!(builder.get_limit(), Some(10));
		assert_eq!(builder.get_skip(), Some(20));
	}

	#[test]
	fn test_projection() {
		let builder = MongoDBQueryBuilder::new("users").projection(doc! { "name": 1, "email": 1 });
		let projection = builder.build_projection();
		let projection = projection.unwrap();
		assert!(projection.contains_key("name"));
		assert!(projection.contains_key("email"));
	}

	#[test]
	fn test_aggregation_pipeline() {
		let builder = MongoDBQueryBuilder::new("users")
			.filter(doc! { "age": { "$gte": 18 } })
			.sort(doc! { "name": 1 })
			.skip(10)
			.limit(5)
			.projection(doc! { "name": 1 });

		let pipeline = builder.build_aggregation_pipeline();
		assert_eq!(pipeline.len(), 5); // match, sort, skip, limit, project

		// Verify stages
		assert!(pipeline[0].contains_key("$match"));
		assert!(pipeline[1].contains_key("$sort"));
		assert!(pipeline[2].contains_key("$skip"));
		assert!(pipeline[3].contains_key("$limit"));
		assert!(pipeline[4].contains_key("$project"));
	}

	#[test]
	fn test_aggregation_pipeline_minimal() {
		let builder = MongoDBQueryBuilder::new("users").filter(doc! { "active": true });

		let pipeline = builder.build_aggregation_pipeline();
		assert_eq!(pipeline.len(), 1); // only match
		assert!(pipeline[0].contains_key("$match"));
	}
}
