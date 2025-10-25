# reinhardt-core

Reinhardtフレームワークのコアコンポーネント

## 概要

`reinhardt-core` は、Reinhardtフレームワークの基本的な構成要素を提供します。他のクレートが依存する重要な型、トレイト、エラーハンドリング、シグナル、セキュリティプリミティブ、バリデータ、バックエンド抽象化を含んでいます。

このクレートは、Reinhardtエコシステム全体の基盤として機能し、フレームワーク全体で使用されるコア抽象化とユーティリティを提供します。

## 機能

### 実装済み ✓

この親クレートは、以下のサブクレートから機能を再エクスポートします:

- **Types** (`reinhardt-types`): コア型定義
  - リクエスト処理のためのHandlerトレイト
  - リクエスト/レスポンスパイプラインのためのMiddlewareトレイト
  - 合成可能なミドルウェアのためのMiddlewareChain
  - 型エイリアスと非同期トレイトサポート

- **Exception** (`reinhardt-exception`): 例外ハンドリングとエラー型
  - Django スタイルの例外階層
  - HTTPステータスコード例外（401、403、404、500など）
  - バリデーションエラーハンドリング
  - データベース例外型
  - カスタムエラー型（ImproperlyConfigured、ParseErrorなど）

- **Signals** (`reinhardt-signals`): ライフサイクルイベントのためのイベント駆動フック
  - 疎結合通信のための型安全なシグナルシステム
  - モデル、マイグレーション、リクエストのライフサイクルシグナル
  - 非同期および同期シグナルディスパッチパターン
  - シグナル合成とミドルウェア
  - パフォーマンスモニタリング

- **Macros** (`reinhardt-macros`): コード生成のための手続き型マクロ
  - エンドポイント定義のための`#[handler]`マクロ
  - ミドルウェア実装のための`#[middleware]`マクロ
  - 依存性注入のための`#[injectable]`マクロ

- **Security** (`reinhardt-security`): セキュリティプリミティブとユーティリティ
  - パスワードハッシュ化と検証
  - CSRF保護
  - XSS防止
  - セキュアな乱数生成
  - 定数時間比較

- **Validators** (`reinhardt-validators`): データバリデーションユーティリティ
  - メールバリデーション
  - URLバリデーション
  - 長さバリデータ
  - 範囲バリデータ
  - カスタムバリデータサポート

- **Backends** (`reinhardt-backends`): バックエンド抽象化
  - キャッシュバックエンドトレイト
  - セッションバックエンドトレイト
  - メモリバックエンド実装
  - Redisバックエンド実装

### 予定

- 追加のミドルウェア型
- 強化されたセキュリティ機能
- より多くのバリデータ型
- 追加のバックエンド実装

## インストール

`Cargo.toml` に以下を追加してください:

```toml
[dependencies]
reinhardt-core = "0.1.0"
```

### オプション機能

ニーズに応じて特定のサブクレートを有効にできます:

```toml
[dependencies]
reinhardt-core = { version = "0.1.0", features = ["signals", "macros", "security"] }
```

利用可能な機能:

- `types` (デフォルト): コア型定義
- `exception` (デフォルト): エラーハンドリング
- `signals` (デフォルト): イベントシステム
- `macros` (デフォルト): 手続き型マクロ
- `security` (デフォルト): セキュリティプリミティブ
- `validators` (デフォルト): データバリデーション
- `backends` (デフォルト): バックエンド抽象化
- `redis-backend`: Redisバックエンド実装

## 使用方法

### ハンドラとミドルウェア

```rust
use reinhardt_core::{Handler, Middleware, Request, Response, Result};

// ハンドラを定義
async fn my_handler(req: Request) -> Result<Response> {
    Response::ok().with_body("Hello, world!")
}

// ミドルウェアを定義
struct LoggingMiddleware;

#[async_trait]
impl Middleware for LoggingMiddleware {
    async fn process_request(&self, req: Request) -> Result<Request> {
        println!("Processing request: {:?}", req.uri());
        Ok(req)
    }
}
```

### エラーハンドリング

```rust
use reinhardt_core::{Error, Result};

fn validate_user() -> Result<()> {
    if !authenticated {
        return Err(Error::Authentication("Invalid credentials".into()));
    }
    if !authorized {
        return Err(Error::Authorization("Permission denied".into()));
    }
    Ok(())
}
```

### シグナル

```rust
use reinhardt_core::{Signal, SignalDispatcher};

// シグナルを定義
static USER_CREATED: Signal<User> = Signal::new();

// レシーバーを接続
USER_CREATED.connect(|user| {
    println!("User created: {}", user.name);
});

// シグナルを送信
USER_CREATED.send(user)?;
```

## サブクレート

この親クレートには以下のサブクレートが含まれています:

```
reinhardt-core/
├── Cargo.toml          # 親クレート定義
├── src/
│   └── lib.rs          # サブクレートからの再エクスポート
└── crates/
    ├── types/          # コア型定義
    ├── exception/      # エラーハンドリング
    ├── signals/        # イベントシステム
    ├── macros/         # 手続き型マクロ
    ├── security/       # セキュリティプリミティブ
    ├── validators/     # データバリデーション
    └── backends/       # バックエンド抽象化
```

## ライセンス

Apache License, Version 2.0 または MIT ライセンスのいずれかの条件の下でライセンスされています。