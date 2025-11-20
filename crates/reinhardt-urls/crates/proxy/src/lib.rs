//! # Reinhardt Association Proxy
//!
//! SQLAlchemy-style association proxies for transparent attribute access through relationships.
//!
//! ## Overview
//!
//! Association proxies allow you to access attributes on related objects as if they were
//! attributes on the parent object. This is particularly useful for many-to-many relationships
//! where you want to work with related objects' attributes directly.
//!
//! ## Example
//!
//! ```rust,ignore
//! use reinhardt_proxy::AssociationProxy;
//!
//! // User has many keywords through user_keywords
//! struct User {
//!     id: i64,
//!     user_keywords: Vec<UserKeyword>,
//! }
//!
//! struct UserKeyword {
//!     user_id: i64,
//!     keyword_id: i64,
//!     keyword: Keyword,
//! }
//!
//! struct Keyword {
//!     id: i64,
//!     name: String,
//! }
//!
//! // Create proxy to access keyword names directly
//! let keywords_proxy = AssociationProxy::new("user_keywords", "keyword");
//! let keyword_names: Vec<String> = keywords_proxy.get_names(&user).await?;
//! ```

pub mod builder;
pub mod collection;
pub mod joins;
pub mod lazy_url;
pub mod loading;
pub mod orm_integration;
pub mod proxy;
pub mod query;
pub mod reflection;
pub mod scalar;
pub mod url_namespace;
pub mod url_pattern;
pub mod url_resolver;

pub use builder::ProxyBuilder;
pub use collection::{CollectionAggregations, CollectionOperations, CollectionProxy};
pub use joins::{
	CircularReferenceError, JoinConfig, NestedProxy, RelationshipPath, extract_through_path,
	filter_through_path, traverse_and_extract, traverse_relationships,
};
// Re-export LoadingStrategy from reinhardt-orm for consistency
pub use lazy_url::LazyUrl;
pub use loading::{
	EagerLoadConfig, EagerLoadable, LazyLoadable, LazyLoaded, LoadStrategy, RelationshipCache,
};
pub use orm_integration::OrmReflectable;
pub use reinhardt_db::orm::LoadingStrategy;

// Note: impl_orm_reflectable macro is exported at crate root via #[macro_export]
// and can be used directly as reinhardt_proxy::impl_orm_reflectable!
pub use proxy::{AssociationProxy, ProxyAccessor, ProxyTarget, ScalarValue};
pub use query::{FilterCondition, FilterOp, QueryFilter};
pub use reflection::{
	AttributeExtractor, ProxyCollection, Reflectable, ReflectableFactory, downcast_relationship,
	extract_collection_values,
};
pub use scalar::{ScalarComparison, ScalarProxy};
pub use url_namespace::UrlNamespace;
pub use url_pattern::UrlPattern;
pub use url_resolver::UrlResolver;

use thiserror::Error;

/// Result type for association proxy operations
pub type ProxyResult<T> = Result<T, ProxyError>;

/// Errors that can occur in association proxy operations
#[derive(Debug, Error)]
pub enum ProxyError {
	/// Target relationship not found
	#[error("Target relationship '{0}' not found")]
	RelationshipNotFound(String),

	/// Attribute not found on target object
	#[error("Attribute '{0}' not found on target")]
	AttributeNotFound(String),

	/// Type mismatch in proxy operation
	#[error("Type mismatch: expected {expected}, got {actual}")]
	TypeMismatch { expected: String, actual: String },

	/// Invalid proxy configuration
	#[error("Invalid proxy configuration: {0}")]
	InvalidConfiguration(String),

	/// Database error during proxy operation
	#[error("Database error: {0}")]
	DatabaseError(String),

	/// Serialization error
	#[error("Serialization error: {0}")]
	SerializationError(String),

	/// Factory not configured for collection proxy
	#[error(
		"Factory not configured for collection proxy - required for creating objects from scalar values"
	)]
	FactoryNotConfigured,
}
