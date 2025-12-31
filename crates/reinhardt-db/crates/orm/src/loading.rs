//! # Loading Strategies
//!
//! Implements SQLAlchemy-inspired loading strategies for relationships.
//!
//! This module provides different strategies for loading related objects:
//! - Joined: Load via JOIN in a single query
//! - Selectin: Load in separate SELECT IN query (most efficient for collections)
//! - Subquery: Load via subquery
//! - Lazy: Load on first access
//! - Raise: Raise error if accessed when not loaded
//! - NoLoad: Never load automatically
//! - WriteOnly: Write-only collections (no read)
//!
//! This module is inspired by SQLAlchemy's loading.py
//! Copyright 2005-2025 SQLAlchemy authors and contributors
//! Licensed under MIT License. See THIRD-PARTY-NOTICES for details.

use crate::Model;
use std::marker::PhantomData;

/// Loading strategy for relationships
/// Corresponds to SQLAlchemy's lazy parameter and loader options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoadingStrategy {
	/// Load immediately with parent via JOIN
	/// Most efficient for single objects, can cause cartesian product for collections
	/// SQLAlchemy: lazy='joined' or joinedload()
	Joined,

	/// Load in a separate SELECT IN query after parent loads
	/// Most efficient for collections, avoids cartesian product
	/// SQLAlchemy: lazy='selectin' or selectinload()
	Selectin,

	/// Load via subquery that wraps the parent query
	/// SQLAlchemy: lazy='subquery' or subqueryload()
	Subquery,

	/// Load on first access (traditional lazy loading)
	/// SQLAlchemy: lazy='select' or lazy=True
	Lazy,

	/// Raise error if accessed when not loaded
	/// Useful to catch N+1 query problems during development
	/// SQLAlchemy: lazy='raise' or raiseload()
	Raise,

	/// Never load automatically, must be loaded explicitly
	/// SQLAlchemy: lazy='noload' or noload()
	NoLoad,

	/// Write-only relationship, cannot be read
	/// Useful for large collections that should only be modified
	/// SQLAlchemy: lazy='write_only'
	WriteOnly,

	/// Return a dynamic query object instead of loading
	/// Allows further filtering on the relationship
	/// SQLAlchemy: lazy='dynamic'
	Dynamic,
}

impl LoadingStrategy {
	/// Check if this strategy requires immediate loading
	///
	pub fn is_eager(&self) -> bool {
		matches!(
			self,
			LoadingStrategy::Joined | LoadingStrategy::Selectin | LoadingStrategy::Subquery
		)
	}
	/// Check if this strategy loads on access
	///
	pub fn is_lazy(&self) -> bool {
		matches!(self, LoadingStrategy::Lazy)
	}
	/// Check if this strategy prevents loading
	///
	pub fn prevents_load(&self) -> bool {
		matches!(
			self,
			LoadingStrategy::Raise | LoadingStrategy::NoLoad | LoadingStrategy::WriteOnly
		)
	}
	/// Get SQL hint for query planner
	///
	pub fn sql_hint(&self) -> Option<&'static str> {
		match self {
			LoadingStrategy::Joined => Some("/* +JOINEDLOAD */"),
			LoadingStrategy::Selectin => Some("/* +SELECTINLOAD */"),
			LoadingStrategy::Subquery => Some("/* +SUBQUERYLOAD */"),
			_ => None,
		}
	}
}

/// Load option for a specific relationship path
/// Corresponds to SQLAlchemy's Load object
#[derive(Debug, Clone)]
pub struct LoadOption {
	/// Relationship path (e.g., "author.posts.comments")
	path: String,
	/// Loading strategy to use
	strategy: LoadingStrategy,
}

impl LoadOption {
	/// Create a new load option for a path
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::loading::{LoadOption, LoadingStrategy};
	///
	/// let option = LoadOption::new("posts", LoadingStrategy::Joined);
	/// assert_eq!(option.path(), "posts");
	/// ```
	pub fn new(path: impl Into<String>, strategy: LoadingStrategy) -> Self {
		Self {
			path: path.into(),
			strategy,
		}
	}
	/// Get the relationship path
	///
	pub fn path(&self) -> &str {
		&self.path
	}
	/// Get the loading strategy
	///
	pub fn strategy(&self) -> LoadingStrategy {
		self.strategy
	}
	/// Parse path into components
	///
	pub fn path_components(&self) -> Vec<&str> {
		self.path.split('.').collect()
	}
}

/// Builder for load options
/// Provides a fluent API similar to SQLAlchemy's Load
pub struct LoadOptionBuilder<T: Model> {
	options: Vec<LoadOption>,
	_phantom: PhantomData<T>,
}

impl<T: Model> LoadOptionBuilder<T> {
	/// Create a new builder
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::{Model, loading::LoadOptionBuilder};
	/// use serde::{Serialize, Deserialize};
	///
	/// #[derive(Debug, Serialize, Deserialize)]
	/// struct User {
	///     id: Option<i32>,
	/// }
	///
	/// impl Model for User {
	///     type PrimaryKey = i32;
	///     fn table_name() -> &'static str {
	///         "users"
	///     }
	///     fn primary_key(&self) -> Option<Self::PrimaryKey> {
	///         self.id
	///     }
	///     fn set_primary_key(&mut self, value: Self::PrimaryKey) {
	///         self.id = Some(value);
	///     }
	/// }
	///
	/// let builder: LoadOptionBuilder<User> = LoadOptionBuilder::new();
	/// // Builder is ready to configure loading options
	/// ```
	pub fn new() -> Self {
		Self {
			options: Vec::new(),
			_phantom: PhantomData,
		}
	}
	/// Load relationship via JOIN
	/// SQLAlchemy: query.options(joinedload(User.addresses))
	///
	pub fn joinedload(mut self, path: impl Into<String>) -> Self {
		self.options
			.push(LoadOption::new(path, LoadingStrategy::Joined));
		self
	}
	/// Load relationship via SELECT IN
	/// SQLAlchemy: query.options(selectinload(User.addresses))
	///
	pub fn selectinload(mut self, path: impl Into<String>) -> Self {
		self.options
			.push(LoadOption::new(path, LoadingStrategy::Selectin));
		self
	}
	/// Load relationship via subquery
	/// SQLAlchemy: query.options(subqueryload(User.addresses))
	///
	pub fn subqueryload(mut self, path: impl Into<String>) -> Self {
		self.options
			.push(LoadOption::new(path, LoadingStrategy::Subquery));
		self
	}
	/// Load relationship lazily on access
	/// SQLAlchemy: query.options(lazyload(User.addresses))
	///
	pub fn lazyload(mut self, path: impl Into<String>) -> Self {
		self.options
			.push(LoadOption::new(path, LoadingStrategy::Lazy));
		self
	}
	/// Raise error if relationship is accessed
	/// SQLAlchemy: query.options(raiseload(User.addresses))
	///
	pub fn raiseload(mut self, path: impl Into<String>) -> Self {
		self.options
			.push(LoadOption::new(path, LoadingStrategy::Raise));
		self
	}
	/// Never load relationship
	/// SQLAlchemy: query.options(noload(User.addresses))
	///
	pub fn noload(mut self, path: impl Into<String>) -> Self {
		self.options
			.push(LoadOption::new(path, LoadingStrategy::NoLoad));
		self
	}
	/// Build the list of load options
	///
	pub fn build(self) -> Vec<LoadOption> {
		self.options
	}
}

impl<T: Model> Default for LoadOptionBuilder<T> {
	fn default() -> Self {
		Self::new()
	}
}

/// Helper functions to create load options
/// These mirror SQLAlchemy's top-level functions
/// Helper functions to create load options
/// These mirror SQLAlchemy's top-level functions
/// Create a joinedload option
///
pub fn joinedload(path: impl Into<String>) -> LoadOption {
	LoadOption::new(path, LoadingStrategy::Joined)
}
/// Create a selectinload option
///
pub fn selectinload(path: impl Into<String>) -> LoadOption {
	LoadOption::new(path, LoadingStrategy::Selectin)
}
/// Create a subqueryload option
///
pub fn subqueryload(path: impl Into<String>) -> LoadOption {
	LoadOption::new(path, LoadingStrategy::Subquery)
}
/// Create a lazyload option
///
pub fn lazyload(path: impl Into<String>) -> LoadOption {
	LoadOption::new(path, LoadingStrategy::Lazy)
}
/// Create a raiseload option
///
pub fn raiseload(path: impl Into<String>) -> LoadOption {
	LoadOption::new(path, LoadingStrategy::Raise)
}
/// Create a noload option
///
pub fn noload(path: impl Into<String>) -> LoadOption {
	LoadOption::new(path, LoadingStrategy::NoLoad)
}

/// Load execution context
/// Tracks which relationships have been loaded and how
#[derive(Debug, Default)]
pub struct LoadContext {
	/// Paths that have been loaded
	loaded_paths: Vec<String>,
	/// Strategies used for each path
	strategies: Vec<(String, LoadingStrategy)>,
}

impl LoadContext {
	/// Create a new load context
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::loading::LoadContext;
	///
	/// let context = LoadContext::new();
	/// ```
	pub fn new() -> Self {
		Self::default()
	}
	/// Mark a path as loaded with a strategy
	///
	pub fn mark_loaded(&mut self, path: String, strategy: LoadingStrategy) {
		self.loaded_paths.push(path.clone());
		self.strategies.push((path, strategy));
	}
	/// Check if a path has been loaded
	///
	pub fn is_loaded(&self, path: &str) -> bool {
		self.loaded_paths.contains(&path.to_string())
	}
	/// Get the strategy used for a path
	///
	pub fn strategy_for(&self, path: &str) -> Option<LoadingStrategy> {
		self.strategies
			.iter()
			.find(|(p, _)| p == path)
			.map(|(_, s)| *s)
	}
	/// Get all loaded paths
	///
	pub fn loaded_paths(&self) -> &[String] {
		&self.loaded_paths
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_core::validators::TableName;
	use serde::{Deserialize, Serialize};

	#[derive(Debug, Clone, Serialize, Deserialize)]
	struct User {
		id: Option<i64>,
	}

	#[derive(Clone)]
	struct UserFields;
	impl crate::model::FieldSelector for UserFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	const USER_TABLE: TableName = TableName::new_const("users");

	impl Model for User {
		type PrimaryKey = i64;
		type Fields = UserFields;

		fn table_name() -> &'static str {
			USER_TABLE.as_str()
		}

		fn new_fields() -> Self::Fields {
			UserFields
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}
	}

	#[test]
	fn test_loading_strategy_properties_unit() {
		assert!(LoadingStrategy::Joined.is_eager());
		assert!(LoadingStrategy::Selectin.is_eager());
		assert!(LoadingStrategy::Subquery.is_eager());
		assert!(LoadingStrategy::Lazy.is_lazy());
		assert!(LoadingStrategy::Raise.prevents_load());
		assert!(LoadingStrategy::NoLoad.prevents_load());
	}

	#[test]
	fn test_load_option_creation() {
		let opt = LoadOption::new("author.posts", LoadingStrategy::Joined);
		assert_eq!(opt.path(), "author.posts");
		assert_eq!(opt.strategy(), LoadingStrategy::Joined);
		assert_eq!(opt.path_components(), vec!["author", "posts"]);
	}

	#[test]
	fn test_load_option_builder() {
		let options = LoadOptionBuilder::<User>::new()
			.joinedload("posts")
			.selectinload("comments")
			.raiseload("profile")
			.build();

		assert_eq!(options.len(), 3);
		assert_eq!(options[0].path(), "posts");
		assert_eq!(options[0].strategy(), LoadingStrategy::Joined);
		assert_eq!(options[1].path(), "comments");
		assert_eq!(options[1].strategy(), LoadingStrategy::Selectin);
		assert_eq!(options[2].path(), "profile");
		assert_eq!(options[2].strategy(), LoadingStrategy::Raise);
	}

	#[test]
	fn test_load_context() {
		let mut ctx = LoadContext::new();
		ctx.mark_loaded("posts".to_string(), LoadingStrategy::Joined);
		ctx.mark_loaded("comments".to_string(), LoadingStrategy::Selectin);

		assert!(ctx.is_loaded("posts"));
		assert!(ctx.is_loaded("comments"));
		assert!(!ctx.is_loaded("profile"));
		assert_eq!(ctx.strategy_for("posts"), Some(LoadingStrategy::Joined));
		assert_eq!(
			ctx.strategy_for("comments"),
			Some(LoadingStrategy::Selectin)
		);
	}

	#[test]
	fn test_helper_functions() {
		let joined = joinedload("posts");
		assert_eq!(joined.strategy(), LoadingStrategy::Joined);

		let selectin = selectinload("comments");
		assert_eq!(selectin.strategy(), LoadingStrategy::Selectin);

		let raise = raiseload("profile");
		assert_eq!(raise.strategy(), LoadingStrategy::Raise);
	}

	#[test]
	fn test_loading_sql_hints() {
		assert!(LoadingStrategy::Joined.sql_hint().is_some());
		assert!(LoadingStrategy::Selectin.sql_hint().is_some());
		assert!(LoadingStrategy::Subquery.sql_hint().is_some());
		assert!(LoadingStrategy::Lazy.sql_hint().is_none());
	}
}
