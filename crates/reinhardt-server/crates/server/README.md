# reinhardt-server

高性能HTTPサーバー実装

## Overview

Hyperをベースにした高性能HTTPサーバー。リクエストルーティング、WebSocket接続、GraphQLサポートを提供し、async/awaitによる並行接続を処理します。

## Features

### Implemented ✓

#### Core HTTP Server

- **HTTP/1.1サーバー**: Hyperベースの高性能HTTP/1.1サーバー実装
- **非同期リクエスト処理**: Tokioランタイムによる完全非同期処理
- **カスタムハンドラーサポート**: `Handler` traitを実装することでカスタムロジックを追加可能
- **TCP接続管理**: 効率的なTCP接続管理とタスクスポーニング
- **リクエスト/レスポンス変換**: Hyperリクエストとreinhardt-httpのRequest/Response間の自動変換
- **エラーハンドリング**: ハンドラーエラーを自動的に500エラーレスポンスに変換

#### WebSocket Support (feature = "websocket")

- **WebSocketサーバー**: tokio-tungstenitベースのWebSocketサーバー実装
- **カスタムメッセージハンドラー**: `WebSocketHandler` traitによるメッセージ処理のカスタマイズ
- **接続ライフサイクルフック**: `on_connect`と`on_disconnect`による接続イベントのハンドリング
- **テキスト/バイナリメッセージ**: テキストメッセージの処理とバイナリメッセージのエコー
- **自動接続管理**: WebSocket接続の確立、メッセージループ、クローズ処理の自動管理
- **ピア情報**: クライアントのSocketAddr情報へのアクセス

#### GraphQL Support (feature = "graphql")

- **GraphQLハンドラー**: async-graphql統合によるGraphQLエンドポイントのサポート
- **スキーマビルダー**: QueryとMutationルートからのスキーマ自動構築
- **POSTリクエスト処理**: GraphQLクエリのPOSTリクエストによる実行
- **JSON レスポンス**: GraphQL実行結果の自動JSON シリアライゼーション
- **エラーハンドリング**: GraphQLエラーの適切な処理とレスポンス
- **空のサブスクリプション**: デフォルトで`EmptySubscription`を使用

#### Convenience Functions

- **`serve()` 関数**: HTTPサーバーの簡単な起動を提供するヘルパー関数
- **`serve_websocket()` 関数**: WebSocketサーバーの簡単な起動を提供するヘルパー関数
- **`graphql_handler()` 関数**: GraphQLハンドラーのArc包装を簡略化

#### Graceful Shutdown

- **ShutdownCoordinator**: Gracefulシャットダウンの調整機構
  - シグナルハンドリング (SIGTERM, SIGINT)
  - 既存接続の完了待機
  - タイムアウト処理付きシャットダウン
  - Broadcast channelによるシャットダウン通知
- **shutdown_signal()**: OSシャットダウンシグナルのリスニング
- **listen_with_shutdown()**: Graceful shutdownサポート付きサーバー起動
- **serve_with_shutdown()**: Graceful shutdown対応の便利関数
- **with_shutdown()**: Futureにシャットダウンハンドリングを追加

#### HTTP/2 Support

- **Http2Server**: HTTP/2プロトコルサーバー実装
  - hyper-utilのHTTP/2ビルダー使用
  - 完全非同期リクエスト処理
  - Graceful shutdownサポート
  - HTTP/1.1と同じHandlerトレイトを使用
- **serve_http2()**: HTTP/2サーバーの簡単な起動を提供
- **serve_http2_with_shutdown()**: Graceful shutdown対応のHTTP/2サーバー起動

#### Request Timeouts

- **TimeoutHandler**: リクエストタイムアウトミドルウェア
  - 設定可能なタイムアウト期間
  - タイムアウト時に408 Request Timeout応答を返す
  - 任意のHandlerをラップ可能
  - 完全にテスト済み

#### Rate Limiting

- **RateLimitHandler**: レート制限ミドルウェア
  - IPアドレスベースのレート制限
  - Fixed WindowとSliding Window戦略をサポート
  - 設定可能なウィンドウ期間と最大リクエスト数
  - レート制限超過時に429 Too Many Requests応答を返す
- **RateLimitConfig**: レート制限設定
  - `per_minute()`: 分単位のレート制限
  - `per_hour()`: 時間単位のレート制限
  - カスタム設定可能

### Planned

#### Advanced HTTP Features

- **ミドルウェアパイプライン**: リクエスト/レスポンス処理のミドルウェアチェーン
- **接続プーリング**: HTTP接続の効率的なプーリング機構
- **リクエストロギング**: 構造化されたリクエストログ

#### WebSocket Advanced Features

- **ブロードキャストサポート**: 複数クライアントへのメッセージブロードキャスト
- **ルームベース管理**: クライアントをルームごとに管理
- **メッセージ圧縮**: WebSocketメッセージの圧縮サポート
- **ハートビート/Ping-Pong**: 接続の生存確認機構
- **認証/認可**: WebSocket接続の認証と認可

#### GraphQL Advanced Features

- **サブスクリプションサポート**: リアルタイムGraphQLサブスクリプション
- **DataLoader統合**: N+1問題解決のためのDataLoader
- **GraphQLプレイグラウンド**: GraphQL IDE統合
- **ファイルアップロード**: GraphQLによるファイルアップロード
- **バッチクエリ**: 複数クエリのバッチ実行

#### Testing & Monitoring

- **メトリクス**: サーバーメトリクスの収集と公開
- **ヘルスチェック**: サーバーヘルスチェックエンドポイント
- **トレーシング**: 分散トレーシングのサポート

## Usage

### Basic HTTP Server

```rust
use std::sync::Arc;
use std::net::SocketAddr;
use reinhardt_server::{HttpServer, serve};
use reinhardt_types::Handler;
use reinhardt_http::{Request, Response};

struct MyHandler;

#[async_trait::async_trait]
impl Handler for MyHandler {
    async fn handle(&self, _req: Request) -> reinhardt_exception::Result<Response> {
        Ok(Response::ok().with_body("Hello, World!"))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let handler = Arc::new(MyHandler);
    let addr: SocketAddr = "127.0.0.1:8080".parse()?;

    // Option 1: Using HttpServer directly
    let server = HttpServer::new(handler.clone());
    server.listen(addr).await?;

    // Option 2: Using convenience function
    serve(addr, handler).await?;

    Ok(())
}
```

### WebSocket Server (feature = "websocket")

```rust
use std::sync::Arc;
use std::net::SocketAddr;
use reinhardt_server::{WebSocketServer, WebSocketHandler, serve_websocket};

struct EchoHandler;

#[async_trait::async_trait]
impl WebSocketHandler for EchoHandler {
    async fn handle_message(&self, message: String) -> Result<String, String> {
        Ok(format!("Echo: {}", message))
    }

    async fn on_connect(&self) {
        println!("Client connected");
    }

    async fn on_disconnect(&self) {
        println!("Client disconnected");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let handler = Arc::new(EchoHandler);
    let addr: SocketAddr = "127.0.0.1:9001".parse()?;
    serve_websocket(addr, handler).await?;
    Ok(())
}
```

### GraphQL Server (feature = "graphql")

```rust
use std::sync::Arc;
use std::net::SocketAddr;
use reinhardt_server::{HttpServer, graphql_handler};
use async_graphql::Object;

struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn hello(&self) -> &str {
        "Hello, GraphQL!"
    }

    async fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }
}

struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn increment(&self, value: i32) -> i32 {
        value + 1
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let handler = graphql_handler(QueryRoot, MutationRoot);
    let addr: SocketAddr = "127.0.0.1:8080".parse()?;

    let server = HttpServer::new(handler);
    server.listen(addr).await?;

    Ok(())
}
```

## Feature Flags

- `websocket`: WebSocketサーバーサポートを有効化
- `graphql`: GraphQLサーバーサポートを有効化

## Dependencies

- `hyper`: HTTPサーバーの基盤
- `tokio`: 非同期ランタイム
- `tokio-tungstenite`: WebSocketサポート (optional)
- `async-graphql`: GraphQLサポート (optional)
