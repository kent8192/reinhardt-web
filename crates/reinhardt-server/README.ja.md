# reinhardt-server

ReinhardtフレームワークのHTTPサーバー実装

## 概要

`reinhardt-server`は、WebSocketとGraphQLサポートを備えたReinhardtアプリケーション向けの高性能HTTPサーバー実装を提供します。Hyperをベースに構築されており、サーバー関連機能を統合する親クレートとして機能します。

## 機能

### 実装済み ✓

この親クレートは`server`サブクレートから機能を再エクスポートしています:

- **コアHTTPサーバー**: 高性能HTTP/1.1サーバー
  - Tokioランタイムによる非同期リクエスト処理
  - Handlerトレイトによるカスタムハンドラーサポート
  - 効率的なTCP接続管理
  - 自動リクエスト/レスポンス変換
  - 組み込みエラーハンドリング

- **WebSocketサポート** (feature = "websocket"): WebSocketサーバー実装
  - tokio-tungstenitベースのWebSocketサーバー
  - カスタムメッセージハンドラーサポート
  - 接続ライフサイクルフック (on_connect, on_disconnect)
  - テキストおよびバイナリメッセージ処理
  - 自動接続管理

- **GraphQLサポート** (feature = "graphql"): GraphQLエンドポイント統合
  - async-graphql統合
  - QueryとMutationルート用のスキーマビルダー
  - GraphQLクエリのPOSTリクエスト処理
  - JSONレスポンスシリアライゼーション
  - GraphQLエラーのエラーハンドリング

### 予定

- HTTP/2サポート
- ミドルウェアパイプライン
- コネクションプーリング
- グレースフルシャットダウン
- リクエストタイムアウト
- レート制限

## インストール

`Cargo.toml`に以下を追加してください:

```toml
[dependencies]
reinhardt-server = "0.1.0"
```

### オプション機能

必要に応じて特定の機能を有効にします:

```toml
[dependencies]
reinhardt-server = { version = "0.1.0", features = ["websocket", "graphql"] }
```

利用可能な機能:

- `server` (デフォルト): コアHTTPサーバー
- `websocket`: WebSocketサポート
- `graphql`: GraphQLエンドポイントサポート

## 使い方

### 基本的なHTTPサーバー

```rust
use reinhardt_server::{serve, HttpServer};
use reinhardt_http::{Request, Response};
use std::sync::Arc;

async fn my_handler(req: Request) -> Result<Response, Error> {
    Response::ok().with_body("Hello, world!")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let handler = Arc::new(my_handler);
    serve("127.0.0.1:8000", handler).await?;
    Ok(())
}
```

### WebSocketサーバー

```rust
use reinhardt_server::{serve_websocket, WebSocketHandler};

struct MyWebSocketHandler;

impl WebSocketHandler for MyWebSocketHandler {
    async fn on_connect(&self, peer: SocketAddr) {
        println!("Client connected: {}", peer);
    }

    async fn on_text(&self, peer: SocketAddr, text: String) -> Option<String> {
        Some(format!("Echo: {}", text))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    serve_websocket("127.0.0.1:8080", MyWebSocketHandler).await?;
    Ok(())
}
```

### GraphQLサーバー

```rust
use reinhardt_server::graphql_handler;
use async_graphql::{Object, Schema, EmptyMutation, EmptySubscription};

struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn hello(&self) -> String {
        "Hello, GraphQL!".to_string()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .finish();

    let handler = graphql_handler(schema);
    serve("127.0.0.1:8000", handler).await?;
    Ok(())
}
```

## サブクレート

この親クレートには以下のサブクレートが含まれています:

```
reinhardt-server/
├── Cargo.toml          # 親クレート定義
├── src/
│   └── lib.rs          # サブクレートからの再エクスポート
└── crates/
    └── server/         # HTTPサーバー実装
```

## ライセンス

Apache License, Version 2.0またはMITライセンスのいずれかの条件の下でライセンスされています。
