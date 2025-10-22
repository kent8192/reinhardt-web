//! Integration tests for reinhardt-di

use reinhardt_di::{Depends, DiResult, Injectable, InjectionContext, SingletonScope};
use std::sync::Arc;

// Test structures
#[derive(Clone, Debug, PartialEq)]
struct Database {
    connection_string: String,
}

#[derive(Clone, Debug, PartialEq)]
struct UserRepository {
    db: Arc<Database>,
}

#[derive(Clone, Debug, PartialEq)]
struct UserService {
    repo: Arc<UserRepository>,
}

// Injectable implementations
#[async_trait::async_trait]
impl Injectable for Database {
    async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
        Ok(Database {
            connection_string: "postgres://localhost/test".to_string(),
        })
    }
}

#[async_trait::async_trait]
impl Injectable for UserRepository {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        let db = Database::inject(ctx).await?;
        Ok(UserRepository { db: Arc::new(db) })
    }
}

#[async_trait::async_trait]
impl Injectable for UserService {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        let repo = UserRepository::inject(ctx).await?;
        Ok(UserService {
            repo: Arc::new(repo),
        })
    }
}

#[tokio::test]
async fn test_basic_injection() {
    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    let db = Database::inject(&ctx).await.unwrap();
    assert_eq!(db.connection_string, "postgres://localhost/test");
}

#[tokio::test]
async fn test_nested_injection() {
    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    let service = UserService::inject(&ctx).await.unwrap();
    assert_eq!(
        service.repo.db.connection_string,
        "postgres://localhost/test"
    );
}

#[tokio::test]
async fn test_depends_wrapper() {
    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    let db = Depends::<Database>::new().resolve(&ctx).await.unwrap();
    assert_eq!(db.connection_string, "postgres://localhost/test");
}

#[tokio::test]
async fn test_request_scope_isolation() {
    let singleton = Arc::new(SingletonScope::new());

    // Create two separate request contexts
    let ctx1 = InjectionContext::new(Arc::clone(&singleton));
    let ctx2 = InjectionContext::new(Arc::clone(&singleton));

    // Set different values in each request scope
    ctx1.set_request("request1".to_string());
    ctx2.set_request("request2".to_string());

    // Verify isolation
    let val1: Option<Arc<String>> = ctx1.get_request();
    let val2: Option<Arc<String>> = ctx2.get_request();

    assert_eq!(*val1.unwrap(), "request1");
    assert_eq!(*val2.unwrap(), "request2");
}

#[tokio::test]
async fn test_singleton_scope_sharing() {
    let singleton = Arc::new(SingletonScope::new());

    // Set value in singleton scope
    singleton.set("shared_value".to_string());

    // Create two contexts sharing the same singleton
    let ctx1 = InjectionContext::new(Arc::clone(&singleton));
    let ctx2 = InjectionContext::new(Arc::clone(&singleton));

    // Both should see the same value
    let val1: Option<Arc<String>> = ctx1.get_singleton();
    let val2: Option<Arc<String>> = ctx2.get_singleton();

    assert_eq!(*val1.unwrap(), "shared_value");
    assert_eq!(*val2.unwrap(), "shared_value");
}

#[tokio::test]
async fn test_concurrent_request_scopes() {
    use tokio::task;

    let singleton = Arc::new(SingletonScope::new());

    let mut handles = vec![];

    for i in 0..10 {
        let singleton_clone = Arc::clone(&singleton);
        let handle = task::spawn(async move {
            let ctx = InjectionContext::new(singleton_clone);
            ctx.set_request(i);

            // Small delay to ensure concurrency
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            let val: Option<Arc<i32>> = ctx.get_request();
            val.map(|v| *v)
        });
        handles.push(handle);
    }

    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // Each task should have its own isolated value
    for (i, result) in results.iter().enumerate() {
        assert_eq!(result.unwrap(), i as i32);
    }
}

#[tokio::test]
async fn test_di_integration_depends_clone() {
    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    let db1 = Depends::<Database>::new().resolve(&ctx).await.unwrap();
    let db2 = db1.clone();

    // Both should point to the same underlying data
    assert_eq!(db1.connection_string, db2.connection_string);
}

#[tokio::test]
async fn test_mixed_scopes() {
    let singleton = Arc::new(SingletonScope::new());

    // Set singleton value
    singleton.set(100i32);

    // Create request context
    let ctx = InjectionContext::new(Arc::clone(&singleton));
    ctx.set_request(200i32);

    // Verify both scopes work
    let singleton_val: Option<Arc<i32>> = ctx.get_singleton();
    let request_val: Option<Arc<i32>> = ctx.get_request();

    assert_eq!(*singleton_val.unwrap(), 100);
    assert_eq!(*request_val.unwrap(), 200);
}
