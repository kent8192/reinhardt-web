# Reinhardt Dispatch

ReinhardtフレームワークのためのHTTPリクエストディスパッチおよびハンドラーシステム。

## 概要

`reinhardt-dispatch`は、Djangoの`django.core.handlers`および`django.dispatch`に相当するコアリクエスト処理機能を提供します。ミドルウェアの実行、シグナルの発行、および例外処理を含む完全なリクエストライフサイクルを調整します。

## 機能

- **リクエストライフサイクル管理**: HTTPリクエストを最初から最後まで処理
- **ミドルウェアチェーン**: リクエスト/レスポンス処理のための組み合わせ可能なミドルウェア
- **シグナル統合**: ライフサイクルシグナル(`request_started`, `request_finished`)の発行
- **例外処理**: エラーを適切なHTTPレスポンスに変換
- **非同期サポート**: Tokioによる完全なasync/awaitサポート

## アーキテクチャ

```text
Request → BaseHandler → Middleware Chain → URL Resolver → View → Response
               ↓                                            ↓
          Signals                                      Signals
      (request_started)                          (request_finished)
```

## 使用方法

### 基本的なリクエスト処理

```rust
use reinhardt_dispatch::{BaseHandler, DispatchError};
use reinhardt_http::{Request, Response};

async fn handle_request(request: Request) -> Result<Response, DispatchError> {
    let handler = BaseHandler::new();
    handler.handle_request(request).await
}
```

### ミドルウェアとの組み合わせ

```rust
use reinhardt_dispatch::{BaseHandler, MiddlewareChain};
use reinhardt_types::{Handler, Middleware};
use std::sync::Arc;

async fn setup_handler() -> Arc<dyn Handler> {
    let handler = Arc::new(BaseHandler::new());

    MiddlewareChain::new(handler)
        .add_middleware(Arc::new(LoggingMiddleware))
        .add_middleware(Arc::new(AuthMiddleware))
        .build()
}
```

### 例外処理

`DefaultExceptionHandler`は自動的にエラーをHTTPレスポンスに変換します:

- `DispatchError::View` → 500 Internal Server Error
- `DispatchError::UrlResolution` → 404 Not Found
- `DispatchError::Middleware` → 500 Internal Server Error
- `DispatchError::Http` → 400 Bad Request
- `DispatchError::Internal` → 500 Internal Server Error

## コンポーネント

### BaseHandler

以下を行うコアリクエストハンドラー:

- `request_started`シグナルの発行
- リクエストの処理(URLリゾルバーとビューに委譲)
- `request_finished`シグナルの発行
- 例外の処理

### MiddlewareChain

複数のミドルウェアコンポーネントを処理パイプラインに組み合わせます:

```rust
let chain = MiddlewareChain::new(handler)
    .add_middleware(middleware1)
    .add_middleware(middleware2)
    .build();
```

ミドルウェアは逆順(LIFO)で実行されるため、最後に追加されたミドルウェアが最初にリクエストを処理します。

### Dispatcher

ハンドラーとフレームワークの他の部分との間を調整する高レベルディスパッチャー:

```rust
use reinhardt_dispatch::Dispatcher;

let dispatcher = Dispatcher::new(BaseHandler::new());
let response = dispatcher.dispatch(request).await?;
```

### 例外処理

例外モジュールは以下を提供します:

- カスタム例外処理のための`ExceptionHandler`トレイト
- 標準エラーレスポンスのための`DefaultExceptionHandler`
- `convert_exception_to_response`ヘルパー関数
- 型をHTTPレスポンスに変換する`IntoResponse`トレイト

## Djangoとの対応

| Reinhardt                 | Django                                             |
| ------------------------- | -------------------------------------------------- |
| `BaseHandler`             | `django.core.handlers.base.BaseHandler`            |
| `MiddlewareChain`         | `django.core.handlers.base.MiddlewareChain`        |
| `ExceptionHandler`        | `django.core.handlers.exception.exception_handler` |
| `request_started` signal  | `django.core.signals.request_started`              |
| `request_finished` signal | `django.core.signals.request_finished`             |

## 実装上の注意

このクレートはHTTPリクエストディスパッチに焦点を当てており、シグナルディスパッチは別の`reinhardt-signals`クレートで処理されます。この分離により以下が提供されます:

- 明確な責任境界
- HTTPコンテキストの外で使用できる独立したシグナルシステム
- ミドルウェアと例外処理を備えた特化したHTTPリクエスト処理

## ライセンス

Reinhardtプロジェクトと同じ条件でライセンスされています。