//! Dependency injection support for GraphQL resolvers
//!
//! This module provides extensions to integrate Reinhardt's DI system with GraphQL resolvers.
//!
//! # Overview
//!
//! The DI system for GraphQL works by storing an `InjectionContext` in the schema's data,
//! which can then be extracted from the GraphQL `Context` and used to resolve dependencies.
//!
//! # Example
//!
//! ```rust,no_run
//! # use async_graphql::{Context, Object, Result, ID, Schema, EmptyMutation, EmptySubscription, SimpleObject};
//! # use reinhardt_graphql::{GraphQLContextExt, SchemaBuilderExt, graphql_handler};
//! # use reinhardt_di::{InjectionContext, Injectable, DiResult, SingletonScope};
//! # use async_trait::async_trait;
//! # use std::sync::Arc;
//! #
//! # #[derive(Clone, SimpleObject)]
//! # struct User {
//! #     id: ID,
//! #     name: String,
//! # }
//! #
//! # #[derive(Clone)]
//! # struct DatabaseConnection;
//! #
//! # #[async_trait]
//! # impl Injectable for DatabaseConnection {
//! #     async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
//! #         Ok(DatabaseConnection)
//! #     }
//! # }
//! #
//! # impl DatabaseConnection {
//! #     async fn fetch_user(&self, id: &ID) -> Result<User> {
//! #         Ok(User { id: id.clone(), name: "Test User".to_string() })
//! #     }
//! # }
//! pub struct Query;
//!
//! #[Object]
//! impl Query {
//!     async fn user(&self, ctx: &Context<'_>, id: ID) -> Result<User> {
//!         user_impl(ctx, id).await
//!     }
//! }
//!
//! #[graphql_handler]
//! async fn user_impl(
//!     ctx: &Context<'_>,
//!     id: ID,
//!     #[inject] db: DatabaseConnection,
//! ) -> Result<User> {
//!     let user = db.fetch_user(&id).await?;
//!     Ok(user)
//! }
//!
//! let singleton = Arc::new(SingletonScope::new());
//! let injection_ctx = Arc::new(InjectionContext::builder(singleton).build());
//! let schema = Schema::build(Query, EmptyMutation, EmptySubscription)
//!     .with_di_context(injection_ctx)
//!     .finish();
//! ```

use async_graphql::Context;
use reinhardt_di::InjectionContext;
use std::sync::Arc;

/// Extension trait for `async_graphql::Context` to support DI context extraction
///
/// This trait adds methods to `async_graphql::Context` for working with Reinhardt's
/// dependency injection system.
pub trait GraphQLContextExt {
	/// Extract DI context from GraphQL context
	///
	/// Returns `Result<&Arc<InjectionContext>>` if the context exists,
	/// `Err(async_graphql::Error)` otherwise.
	///
	/// # Example
	///
	/// ```rust,no_run
	/// # use reinhardt_graphql::GraphQLContextExt;
	/// # use reinhardt_di::InjectionContext;
	/// # use std::sync::Arc;
	/// # use async_graphql::Context;
	/// # fn example(ctx: &Context<'_>) -> async_graphql::Result<()> {
	/// let di_ctx = ctx.get_di_context()?;
	/// # Ok(())
	/// # }
	/// ```
	fn get_di_context(&self) -> async_graphql::Result<&Arc<InjectionContext>>;
}

impl GraphQLContextExt for Context<'_> {
	fn get_di_context(&self) -> async_graphql::Result<&Arc<InjectionContext>> {
		self.data::<Arc<InjectionContext>>()
	}
}

/// Extension trait for `async_graphql::SchemaBuilder` to easily add DI context
///
/// This trait provides a convenience method to add the `InjectionContext` to the schema data.
///
/// # Example
///
/// ```rust,no_run
/// # use async_graphql::{Schema, EmptyMutation, EmptySubscription, Object};
/// # use reinhardt_graphql::SchemaBuilderExt;
/// # use reinhardt_di::InjectionContext;
/// # use std::sync::Arc;
/// # struct Query;
/// # #[Object]
/// # impl Query {
/// #     async fn hello(&self) -> &str { "world" }
/// # }
/// # use reinhardt_di::SingletonScope;
/// let singleton = Arc::new(SingletonScope::new());
/// let injection_ctx = Arc::new(InjectionContext::builder(singleton).build());
/// let schema = Schema::build(Query, EmptyMutation, EmptySubscription)
///     .with_di_context(injection_ctx)
///     .finish();
/// ```
pub trait SchemaBuilderExt<Query, Mutation, Subscription> {
	/// Add DI context to the schema
	///
	/// This is a convenience method equivalent to `.data(injection_ctx)`.
	fn with_di_context(self, ctx: Arc<InjectionContext>) -> Self;
}

impl<Query, Mutation, Subscription> SchemaBuilderExt<Query, Mutation, Subscription>
	for async_graphql::SchemaBuilder<Query, Mutation, Subscription>
where
	Query: async_graphql::ObjectType + 'static,
	Mutation: async_graphql::ObjectType + 'static,
	Subscription: async_graphql::SubscriptionType + 'static,
{
	fn with_di_context(self, ctx: Arc<InjectionContext>) -> Self {
		self.data(ctx)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use async_graphql::{EmptyMutation, EmptySubscription, Object, Schema};
	use rstest::rstest;

	struct Query;

	#[Object]
	impl Query {
		async fn test(&self) -> i32 {
			42
		}
	}

	#[rstest]
	#[tokio::test]
	async fn test_graphql_context_ext_get_di_context() {
		// Create a mock InjectionContext
		let singleton_scope = reinhardt_di::SingletonScope::new();
		let injection_ctx = Arc::new(InjectionContext::builder(singleton_scope).build());

		// Build schema with DI context
		let schema = Schema::build(Query, EmptyMutation, EmptySubscription)
			.data(injection_ctx.clone())
			.finish();

		// Execute a query to get a context
		let result = schema.execute("{ test }").await;
		assert!(result.errors.is_empty());
	}

	#[rstest]
	#[tokio::test]
	async fn test_graphql_context_ext_missing_context() {
		// Build schema without DI context
		let schema = Schema::build(Query, EmptyMutation, EmptySubscription).finish();

		// Execute a query
		let result = schema.execute("{ test }").await;
		assert!(result.errors.is_empty());
	}
}
