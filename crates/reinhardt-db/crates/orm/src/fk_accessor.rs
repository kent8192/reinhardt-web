//! ForeignKey accessor for reverse relationship access.
//!
//! This module provides the `ForeignKeyAccessor` type, which enables type-safe
//! access to reverse ForeignKey relationships without using string literals.
//!
//! ## Overview
//!
//! When a model has a ForeignKey field, the `#[model(...)]` macro automatically
//! generates a `{field_name}_accessor()` static method that returns a
//! `ForeignKeyAccessor`. This accessor provides a `.reverse()` method to access
//! related records from the target side of the relationship.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use reinhardt_db::orm::ForeignKeyAccessor;
//!
//! // Tweet model has: #[rel(foreign_key)] user: ForeignKeyField<User>
//!
//! // Get a reverse accessor to fetch all tweets for a user
//! let tweets_accessor = Tweet::user_accessor().reverse(&user, db.clone());
//!
//! // Use the accessor to query related records
//! let tweets = tweets_accessor.all().await?;
//! let tweet_count = tweets_accessor.count().await?;
//!
//! // Paginate results
//! let page1 = tweets_accessor.paginate(1, 10).all().await?;
//! ```
//!
//! ## API Design
//!
//! This design provides several advantages:
//! - **Type-safe**: No string literals required
//! - **IDE support**: Full auto-completion for accessor methods
//! - **Compile-time checks**: Invalid relationships cause compilation errors
//! - **Consistent pattern**: Follows the same pattern as ManyToMany accessors

use crate::Model;
use crate::connection::DatabaseConnection;
use crate::reverse_accessor::ReverseAccessor;
use serde::{Serialize, de::DeserializeOwned};
use std::marker::PhantomData;

/// Accessor for ForeignKey relationships that enables reverse access.
///
/// This type is returned by `{field_name}_accessor()` methods generated
/// by the `#[model(...)]` macro for ForeignKey fields.
///
/// # Type Parameters
///
/// - `Source`: The model containing the ForeignKey field (e.g., Tweet)
/// - `Target`: The model being referenced (e.g., User)
///
/// # Examples
///
/// ```rust,ignore
/// // Tweet has: #[rel(foreign_key)] user: ForeignKeyField<User>
///
/// // Get reverse accessor for User → Tweets relationship
/// let tweets_accessor = Tweet::user_accessor().reverse(&user, db);
/// let tweets = tweets_accessor.all().await?;
/// ```
pub struct ForeignKeyAccessor<Source, Target> {
	db_column: &'static str,
	_phantom: PhantomData<(Source, Target)>,
}

impl<Source, Target> ForeignKeyAccessor<Source, Target>
where
	Source: Model + Serialize + DeserializeOwned,
	Target: Model,
{
	/// Create a new ForeignKeyAccessor.
	///
	/// This is typically called by generated code, not directly by users.
	///
	/// # Parameters
	///
	/// - `db_column`: The name of the database column for the foreign key
	///   (e.g., "user_id" for a `user` field)
	pub const fn new(db_column: &'static str) -> Self {
		Self {
			db_column,
			_phantom: PhantomData,
		}
	}

	/// Get the reverse accessor for this ForeignKey relationship.
	///
	/// This allows accessing related records from the target side.
	/// For example, if Tweet has a ForeignKey to User, this method
	/// allows getting all Tweets for a given User.
	///
	/// # Parameters
	///
	/// - `target`: The target model instance (e.g., User)
	/// - `db`: Database connection
	///
	/// # Returns
	///
	/// A `ReverseAccessor` for querying related records.
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// // Get all tweets for a user
	/// let tweets_accessor = Tweet::user_accessor().reverse(&user, db);
	/// let tweets = tweets_accessor.all().await?;
	///
	/// // Count related records
	/// let count = tweets_accessor.count().await?;
	///
	/// // Paginate results
	/// let page1 = tweets_accessor.paginate(1, 10).all().await?;
	/// ```
	pub fn reverse(
		&self,
		target: &Target,
		db: DatabaseConnection,
	) -> ReverseAccessor<Target, Source> {
		ReverseAccessor::new(target, self.db_column, db)
	}
}

// Note: Tests for ForeignKeyAccessor require actual model types and database connections.
// Integration tests are in the examples-twitter project which uses real model types.
