//! Dependency Injection support for ViewSets

use crate::viewset::ViewSet;
use async_trait::async_trait;
use reinhardt_apps::{Request, Response, Result};
use reinhardt_di::{Depends, DiResult, Injectable, InjectionContext};
use std::sync::Arc;

/// ViewSet with DI support
pub struct DiViewSet<V: ViewSet + Injectable + Clone> {
    viewset: Depends<V>,
}

impl<V: ViewSet + Injectable + Clone> DiViewSet<V> {
    /// Create a new DiViewSet by resolving dependencies
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_viewsets::{DiViewSet, GenericViewSet};
    /// use reinhardt_di::{Injectable, InjectionContext, SingletonScope};
    /// use std::sync::Arc;
    /// use async_trait::async_trait;
    ///
    /// #[derive(Clone)]
    /// struct MyHandler;
    ///
    /// #[async_trait]
    /// impl Injectable for MyHandler {
    ///     async fn inject(_ctx: &InjectionContext) -> reinhardt_di::DiResult<Self> {
    ///         Ok(MyHandler)
    ///     }
    /// }
    ///
    /// type MyViewSet = GenericViewSet<MyHandler>;
    ///
    /// #[async_trait]
    /// impl Injectable for MyViewSet {
    ///     async fn inject(ctx: &InjectionContext) -> reinhardt_di::DiResult<Self> {
    ///         let handler = MyHandler::inject(ctx).await?;
    ///         Ok(GenericViewSet::new("my_resource", handler))
    ///     }
    /// }
    ///
    /// # tokio_test::block_on(async {
    /// let singleton = Arc::new(SingletonScope::new());
    /// let ctx = InjectionContext::new(singleton);
    ///
    /// let di_viewset = DiViewSet::<MyViewSet>::new(&ctx).await.unwrap();
    /// assert_eq!(di_viewset.get_basename(), "my_resource");
    /// # });
    /// ```
    pub async fn new(ctx: &InjectionContext) -> DiResult<Self> {
        let viewset = Depends::<V>::resolve(ctx, true).await?;
        Ok(Self { viewset })
    }
    /// Get the inner viewset
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_viewsets::{DiViewSet, GenericViewSet};
    /// use reinhardt_di::{Injectable, InjectionContext, SingletonScope};
    /// use std::sync::Arc;
    /// use async_trait::async_trait;
    ///
    /// #[derive(Clone)]
    /// struct MyHandler;
    ///
    /// #[async_trait]
    /// impl Injectable for MyHandler {
    ///     async fn inject(_ctx: &InjectionContext) -> reinhardt_di::DiResult<Self> {
    ///         Ok(MyHandler)
    ///     }
    /// }
    ///
    /// type MyViewSet = GenericViewSet<MyHandler>;
    ///
    /// #[async_trait]
    /// impl Injectable for MyViewSet {
    ///     async fn inject(ctx: &InjectionContext) -> reinhardt_di::DiResult<Self> {
    ///         let handler = MyHandler::inject(ctx).await?;
    ///         Ok(GenericViewSet::new("my_resource", handler))
    ///     }
    /// }
    ///
    /// # tokio_test::block_on(async {
    /// let singleton = Arc::new(SingletonScope::new());
    /// let ctx = InjectionContext::new(singleton);
    ///
    /// let di_viewset = DiViewSet::<MyViewSet>::new(&ctx).await.unwrap();
    /// let inner_viewset = di_viewset.inner();
    /// assert_eq!(inner_viewset.get_basename(), "my_resource");
    /// # });
    /// ```
    pub fn inner(&self) -> &V {
        &self.viewset
    }
}

#[async_trait]
impl<V: ViewSet + Injectable + Clone> ViewSet for DiViewSet<V> {
    fn get_basename(&self) -> &str {
        self.viewset.get_basename()
    }

    async fn dispatch(&self, request: Request, action: crate::Action) -> Result<Response> {
        self.viewset.dispatch(request, action).await
    }
}

/// Trait for creating ViewSets with dependency injection
#[async_trait]
pub trait ViewSetFactory: Send + Sync {
    type ViewSet: ViewSet;

    /// Create a new viewset instance with injected dependencies
    async fn create(&self, ctx: &InjectionContext) -> DiResult<Self::ViewSet>;
}

/// Example: Database connection as an injectable dependency
#[derive(Clone)]
pub struct DatabaseConnection {
    pub pool: Arc<String>, // Placeholder for actual DB pool
}

#[async_trait]
impl Injectable for DatabaseConnection {
    async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
        Ok(DatabaseConnection {
            pool: Arc::new("postgres://localhost/db".to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Action, GenericViewSet};
    use bytes::Bytes;
    use hyper::{HeaderMap, Method, Uri, Version};
    use reinhardt_apps::{Request, Response};
    use reinhardt_di::SingletonScope;

    #[derive(Clone)]
    struct TestHandler {
        db: DatabaseConnection,
    }

    #[async_trait]
    impl Injectable for TestHandler {
        async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
            let db = DatabaseConnection::inject(ctx).await?;
            Ok(TestHandler { db })
        }
    }

    impl TestHandler {
        async fn handle(&self, _request: Request) -> Result<Response> {
            Ok(Response::ok().with_json(&serde_json::json!({
                "db": *self.db.pool
            }))?)
        }
    }

    type TestViewSet = GenericViewSet<TestHandler>;

    #[async_trait]
    impl Injectable for TestViewSet {
        async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
            let handler = TestHandler::inject(ctx).await?;
            Ok(GenericViewSet::new("test", handler))
        }
    }

    #[tokio::test]
    async fn test_database_connection_injection() {
        let singleton = Arc::new(SingletonScope::new());
        let ctx = InjectionContext::new(singleton);

        let db = DatabaseConnection::inject(&ctx).await.unwrap();
        assert_eq!(*db.pool, "postgres://localhost/db");
    }

    #[tokio::test]
    async fn test_handler_with_injected_db() {
        let singleton = Arc::new(SingletonScope::new());
        let ctx = InjectionContext::new(singleton);

        let handler = TestHandler::inject(&ctx).await.unwrap();
        assert_eq!(*handler.db.pool, "postgres://localhost/db");
    }
}
