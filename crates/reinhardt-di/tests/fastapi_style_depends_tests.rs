//! Tests for FastAPI-style Depends functionality

use reinhardt_di::{Depends, Injectable, InjectionContext, SingletonScope};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Clone, Default, Debug, PartialEq)]
struct CommonQueryParams {
    q: Option<String>,
    skip: usize,
    limit: usize,
}

#[derive(Clone, Default)]
struct Database {
    connection_count: usize,
}

// カスタムInjectableでインスタンスカウンタ（AtomicUsizeで安全に）
static INSTANCE_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone)]
struct CountedService {
    instance_id: usize,
}

#[async_trait::async_trait]
impl Injectable for CountedService {
    async fn inject(_ctx: &InjectionContext) -> reinhardt_di::DiResult<Self> {
        let instance_id = INSTANCE_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(CountedService { instance_id })
    }
}

#[tokio::test]
async fn test_depends_with_cache_default() {
    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    // デフォルトではキャッシュが有効
    let params1 = Depends::<CommonQueryParams>::new()
        .resolve(&ctx)
        .await
        .unwrap();
    let params2 = Depends::<CommonQueryParams>::new()
        .resolve(&ctx)
        .await
        .unwrap();

    // 同じインスタンスを返す
    assert_eq!(*params1, *params2);
}

#[tokio::test]
async fn test_depends_no_cache() {
    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    // キャッシュ無効
    let service1 = Depends::<CountedService>::no_cache()
        .resolve(&ctx)
        .await
        .unwrap();
    let service2 = Depends::<CountedService>::no_cache()
        .resolve(&ctx)
        .await
        .unwrap();

    // 異なるインスタンスが作成される (IDは連続している)
    assert_ne!(service1.instance_id, service2.instance_id);
    assert_eq!(service1.instance_id + 1, service2.instance_id);
}

#[tokio::test]
async fn test_depends_with_cache_enabled() {
    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    // キャッシュ有効（デフォルト）
    let service1 = Depends::<CountedService>::new()
        .resolve(&ctx)
        .await
        .unwrap();
    let service2 = Depends::<CountedService>::new()
        .resolve(&ctx)
        .await
        .unwrap();

    // 同じインスタンスを返す（IDが同じ）
    assert_eq!(service1.instance_id, service2.instance_id);
}

#[tokio::test]
async fn test_depends_from_value() {
    let db = Database {
        connection_count: 10,
    };
    let depends = Depends::from_value(db);

    assert_eq!(depends.connection_count, 10);
}

#[tokio::test]
async fn test_depends_deref() {
    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    let params = Depends::<CommonQueryParams>::new()
        .resolve(&ctx)
        .await
        .unwrap();

    // Derefで直接フィールドにアクセス可能
    assert_eq!(params.skip, 0);
    assert_eq!(params.limit, 0);
}

#[tokio::test]
async fn test_fastapi_depends_clone() {
    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    let params1 = Depends::<CommonQueryParams>::new()
        .resolve(&ctx)
        .await
        .unwrap();
    let params2 = params1.clone();

    // Cloneは参照をコピー（Arc::clone）
    assert_eq!(*params1, *params2);
}

// FastAPIスタイルの使用例
#[tokio::test]
async fn test_fastapi_style_usage() {
    #[derive(Clone, Default)]
    struct Config {
        api_key: String,
    }

    async fn endpoint_handler(
        config: Depends<Config>,
        params: Depends<CommonQueryParams>,
    ) -> String {
        format!("API Key: {}, Skip: {}", config.api_key, params.skip)
    }

    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    // エンドポイントでの使用をシミュレート
    let config = Depends::<Config>::new().resolve(&ctx).await.unwrap();
    let params = Depends::<CommonQueryParams>::new()
        .resolve(&ctx)
        .await
        .unwrap();

    let result = endpoint_handler(config, params).await;
    assert_eq!(result, "API Key: , Skip: 0");
}
