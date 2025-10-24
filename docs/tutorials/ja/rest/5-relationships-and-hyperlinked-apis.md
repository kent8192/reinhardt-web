# チュートリアル 5: リレーションシップとハイパーリンクAPI

APIのリレーションシップを表現する方法と、URLの逆引きについて学びます。

## URL逆引き (Reverse Routing)

Reinhardtは名前付きルートからURLを生成する機能を提供します。

### reverseの使用

```rust
use reinhardt_routers::{DefaultRouter, Router, path};
use reinhardt_apps::Handler;
use std::sync::Arc;
use std::collections::HashMap;

let mut router = DefaultRouter::new();

// 名前付きルートを登録
router.add_route(
    path("/snippets/{id}/", handler)
        .with_name("detail")
        .with_namespace("snippet")
);

// URLを逆引き
let mut params = HashMap::new();
params.insert("id".to_string(), "123".to_string());

let url = router.reverse("snippet:detail", &params)?;
// 結果: "/snippets/123/"
```

### パラメータ付きURL逆引き

```rust
use reinhardt_routers::DefaultRouter;

let router = DefaultRouter::new();

// 簡易パラメータ渡し
let url = router.reverse_with("user-detail", &[("id", "42")])?;
// 結果: "/users/42/"
```

## ハイパーリンク付きシリアライザ

リレーションシップにURLを使用することで、APIがより発見可能になります。

### 基本的なハイパーリンク

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
struct SnippetSerializer {
    pub id: i64,
    pub url: String,         // 自身へのURL
    pub code: String,
    pub owner_url: String,   // オーナーへのURL
}

impl SnippetSerializer {
    fn new(snippet: &Snippet, base_url: &str) -> Self {
        Self {
            id: snippet.id,
            url: format!("{}/snippets/{}/", base_url, snippet.id),
            code: snippet.code.clone(),
            owner_url: format!("{}/users/{}/", base_url, snippet.owner_id),
        }
    }
}
```

### ルーターを使用したURL生成

```rust
use reinhardt_routers::DefaultRouter;
use std::collections::HashMap;

fn build_snippet_url(router: &DefaultRouter, id: i64) -> String {
    let mut params = HashMap::new();
    params.insert("id".to_string(), id.to_string());

    router.reverse("snippet-detail", &params)
        .unwrap_or_else(|_| format!("/snippets/{}/", id))
}
```

## APIルート

APIのエントリポイントを提供:

```rust
use reinhardt_core::{Request, Response, Result};
use serde_json::json;

async fn api_root(request: Request) -> Result<Response> {
    let base_url = "http://127.0.0.1:8000";

    let root = json!({
        "snippets": format!("{}/snippets/", base_url),
        "users": format!("{}/users/", base_url),
    });

    Response::ok().with_json(&root)
}
```

## ネストされたシリアライザ

関連オブジェクトを埋め込む:

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
struct UserSerializer {
    pub id: i64,
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SnippetWithOwner {
    pub id: i64,
    pub code: String,
    pub owner: UserSerializer,  // ネストされたオブジェクト
}
```

## まとめ

このチュートリアルで学んだこと:

1. URL逆引きの使用方法
2. ハイパーリンク付きシリアライザの作成
3. APIルートの実装
4. ネストされたリレーションシップの表現

次のチュートリアル: [チュートリアル 6: ViewSetとRouter](6-viewsets-and-routers.md)
