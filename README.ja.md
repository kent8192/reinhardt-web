<div align="center">
  <img src="branding/logo.png" alt="Reinhardt Logo" width="200"/>

  <h1>Reinhardt</h1>

  <h3>🦀 Polylithic Batteries Included</h3>

  <p><strong>Rust用のモジュラーフルスタックAPIフレームワーク</strong></p>
  <p>Djangoのバッテリーインクルード哲学の全ての力と、<br/>
  必要なものだけを含める柔軟性を兼ね備えています。</p>

  [![Crates.io](https://img.shields.io/crates/v/reinhardt.svg)](https://crates.io/crates/reinhardt)
  [![Documentation](https://docs.rs/reinhardt/badge.svg)](https://docs.rs/reinhardt)
  [![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

</div>

---

## 📍 クイックナビゲーション

お探しの情報:

- 🚀 [クイックスタート](#クイックスタート) - 5分で始める
- 📦 [インストールオプション](#インストール) - Micro、Standard、Fullから選択
- 📚 [Getting Started Guide](docs/GETTING_STARTED.md) - ステップバイステップチュートリアル
- 🎛️ [Feature Flags](docs/FEATURE_FLAGS.md) - ビルドを細かく調整
- 📖 [API Documentation](https://docs.rs/reinhardt) - 完全なAPIリファレンス
- 💬 [コミュニティ & サポート](#ヘルプを得る) - コミュニティからサポートを受ける

## なぜReinhardtなのか？

Reinhardtは3つの世界のベストを統合しています:

| インスピレーション | 何を借用したか | 何を改善したか |
|------------|------------------|------------------|
| 🐍 **Django** | バッテリーインクルード哲学、ORM設計、管理画面 | モジュラービルドのための機能フラグ、Rustの型安全性 |
| 🎯 **Django REST** | シリアライザ、ビューセット、パーミッション | コンパイル時検証、ゼロコスト抽象化 |
| ⚡ **FastAPI** | DIシステム、自動OpenAPI | ネイティブRustパフォーマンス、ランタイムオーバーヘッドなし |
| 🗄️ **SQLAlchemy** | QuerySetパターン、リレーション処理 | 型安全なクエリビルダー、コンパイル時検証 |

**結果**: Python開発者にとって馴染み深いフレームワークでありながら、Rustのパフォーマンスと安全性の保証を備えています。

## ✨ 特徴

### 🎯 コアフレームワーク
- **型安全なORM**: コンパイル時クエリ検証を備えたQuerySet API
- **強力なシリアライザ**: 自動バリデーションと変換
- **柔軟なビューセット**: CRUD操作のためのDRY原則
- **スマートルーティング**: ViewSetからの自動URL構成
- **マルチ認証サポート**: JWT、Token、Session、Basic認証

### 🚀 FastAPIインスパイアの人間工学
- **パラメータ抽出**: 型安全な`Path<T>`、`Query<T>`、`Header<T>`、`Cookie<T>`、`Json<T>`、`Form<T>`抽出器
- **Dependency Injection**: `Depends<T>`、リクエストスコープ、キャッシングを備えたFastAPIスタイルのDIシステム
- **自動OpenAPI**: `#[derive(Schema)]`でRustの型からOpenAPI 3.0スキーマを生成
- **関数ベースのエンドポイント**: APIルートを定義するための人間工学的な`#[endpoint]`マクロ (近日公開)
- **バックグラウンドタスク**: シンプルな非同期タスク実行

### 🔋 バッテリーインクルード
- **管理画面**: Djangoスタイルの自動生成管理インターフェース (近日公開)
- **ミドルウェアシステム**: リクエスト/レスポンス処理パイプライン
- **管理コマンド**: マイグレーション、静的ファイルなどのためのCLIツール
- **ページネーション**: PageNumber、LimitOffset、Cursor戦略
- **フィルタリング & 検索**: クエリセット用の組み込みSearchFilterとOrderingFilter
- **レート制限**: 柔軟なスロットリング (AnonRateThrottle、UserRateThrottle、ScopedRateThrottle)
- **シグナル**: イベント駆動フック (pre_save、post_save、pre_delete、post_delete、m2m_changed)

### 🌍 高度な機能
- **GraphQLサポート**: RESTと並行してGraphQL APIを構築 (近日公開)
- **WebSocketサポート**: リアルタイム双方向通信 (近日公開)
- **国際化**: 多言語サポート
- **静的ファイル**: CDN統合、ハッシュストレージ、圧縮
- **ブラウザブルAPI**: API探索用のHTMLインターフェース

## インストール

Reinhardtは、プロジェクトの規模に合わせて3つのフレーバーを提供しています:

### Reinhardt Micro - マイクロサービス向け

軽量で高速、シンプルなAPIやマイクロサービスに最適:

```toml
[dependencies]
reinhardt-micro = "0.1.0"
```

### Reinhardt Standard - バランス型

ほとんどのプロジェクトに適したデフォルト構成:

```toml
[dependencies]
reinhardt = "0.1.0"
# 以下と同等: reinhardt = { version = "0.1.0", features = ["standard"] }
```

### Reinhardt Full - 全機能搭載

全ての機能を有効化、Djangoスタイルのバッテリーインクルード:

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = ["full"] }
```

### カスタム構成

必要に応じて機能を組み合わせる:

```toml
[dependencies]
# ルーティングとパラメータのみの最小構成
reinhardt = { version = "0.1.0", default-features = false, features = ["minimal"] }

# データベースサポートを追加
reinhardt = { version = "0.1.0", default-features = false, features = ["minimal", "database"] }

# Standardに追加機能
reinhardt = { version = "0.1.0", features = ["standard", "websockets", "graphql"] }
```

## クイックスタート

### 基本的なCRUD API

```rust
use reinhardt::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: i64,
    name: String,
    email: String,
}

#[derive(Debug, Clone)]
struct UserSerializer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a router
    let mut router = DefaultRouter::new();

    // Create and register a ViewSet for CRUD operations
    let user_viewset: Arc<ModelViewSet<User, UserSerializer>> =
        Arc::new(ModelViewSet::new("users"));
    router.register_viewset("users", user_viewset);

    // Start the server
    println!("Server running on http://127.0.0.1:8000");
    reinhardt::serve("127.0.0.1:8000", router).await?;

    Ok(())
}
```

これにより、以下のエンドポイントを持つ完全なCRUD APIが作成されます:

- `GET /users/` - 全ユーザーの一覧取得
- `POST /users/` - 新規ユーザーの作成
- `GET /users/{id}/` - ユーザーの取得
- `PUT /users/{id}/` - ユーザーの更新
- `DELETE /users/{id}/` - ユーザーの削除

## 🎓 実例で学ぶ

### データベースを使用

```rust
use reinhardt::prelude::*;

#[derive(Model, Serialize, Deserialize)]
#[reinhardt(table_name = "users")]
struct User {
    #[reinhardt(primary_key)]
    id: i64,
    email: String,
    name: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let db = Database::connect("postgres://localhost/mydb").await?;
    let router = DefaultRouter::new().with_database(db);

    reinhardt::serve("127.0.0.1:8000", router).await?;
    Ok(())
}
```

### 認証を使用

```rust
use reinhardt::prelude::*;

#[endpoint(GET, "/profile")]
async fn get_profile(
    user: Authenticated<User>,
) -> Json<UserProfile> {
    Json(user.to_profile())
}
```

### Dependency Injectionを使用

```rust
use reinhardt::prelude::*;

async fn get_db() -> Database {
    Database::from_env()
}

#[endpoint(GET, "/users/{id}")]
async fn get_user(
    Path(id): Path<i64>,
    Depends(db): Depends<Database, get_db>,
) -> Result<Json<User>> {
    let user = User::find_by_id(id, &db).await?;
    Ok(Json(user))
}
```

### シリアライザとバリデーションを使用

```rust
use reinhardt::prelude::*;

#[derive(Serialize, Deserialize, Validate)]
struct CreateUserRequest {
    #[validate(email)]
    email: String,
    #[validate(length(min = 3, max = 50))]
    name: String,
}

#[derive(Serializer)]
#[serializer(model = "User")]
struct UserSerializer {
    id: i64,
    email: String,
    name: String,
}

#[endpoint(POST, "/users")]
async fn create_user(
    Json(req): Json<CreateUserRequest>,
    db: Depends<Database>,
) -> Result<Json<UserSerializer>> {
    req.validate()?;
    let user = User::create(&req, &db).await?;
    Ok(Json(UserSerializer::from(user)))
}
```

## 適切なフレーバーの選択

| 機能                 | Micro      | Standard  | Full    |
| -------------------- | ---------- | --------- | ------- |
| バイナリサイズ       | ~5-10 MB   | ~20-30 MB | ~50+ MB |
| コンパイル時間       | 高速       | 中程度    | 低速    |
| **コア機能**         |
| ルーティング         | ✅         | ✅        | ✅      |
| パラメータ抽出       | ✅         | ✅        | ✅      |
| Dependency Injection | ✅         | ✅        | ✅      |
| **標準機能**         |
| ORM                  | オプション | ✅        | ✅      |
| シリアライザ         | ❌         | ✅        | ✅      |
| ビューセット         | ❌         | ✅        | ✅      |
| 認証                 | ❌         | ✅        | ✅      |
| ページネーション     | ❌         | ✅        | ✅      |
| **高度な機能**       |
| 管理画面             | ❌         | ❌        | ✅      |
| GraphQL              | ❌         | ❌        | ✅      |
| WebSocket            | ❌         | ❌        | ✅      |
| 国際化               | ❌         | ❌        | ✅      |
| **ユースケース**     |
| マイクロサービス     | ✅         | ⚠️        | ❌      |
| REST API             | ✅         | ✅        | ✅      |
| フルアプリケーション | ❌         | ✅        | ✅      |
| 複雑なシステム       | ❌         | ⚠️        | ✅      |

**凡例**: ✅ 推奨 • ⚠️ 可能だが最適ではない • ❌ 非推奨

## コンポーネント

Reinhardtには以下のコアコンポーネントが含まれています:

### コアフレームワーク

- **ORM**: QuerySet APIを備えたデータベース抽象化レイヤー
- **シリアライザ**: 型安全なデータシリアライゼーションとバリデーション
- **ビューセット**: APIエンドポイント用の合成可能なビュー
- **ルーター**: 自動URL構成
- **認証**: JWT認証とパーミッションシステム
- **ミドルウェア**: リクエスト/レスポンス処理パイプライン
- **管理コマンド**: プロジェクト管理のためのDjangoスタイルCLI (`reinhardt-commands`)

### REST API機能 (reinhardt-rest)

- **認証**: JWT、Token、Session、Basic認証
- **ルーティング**: ViewSetの自動URLルーティング
- **ブラウザブルAPI**: API探索用のHTMLインターフェース
- **スキーマ生成**: OpenAPI/Swaggerドキュメント
- **ページネーション**: PageNumber、LimitOffset、Cursorページネーション
- **フィルタリング**: クエリセット用のSearchFilterとOrderingFilter
- **スロットリング**: レート制限 (AnonRateThrottle、UserRateThrottle、ScopedRateThrottle)
- **シグナル**: イベント駆動フック (pre_save、post_save、pre_delete、post_delete、m2m_changed)

### FastAPIインスパイアの機能

- **パラメータ抽出**: 型安全な`Path<T>`、`Query<T>`、`Header<T>`、`Cookie<T>`、`Json<T>`、`Form<T>`抽出器
- **Dependency Injection**: `Depends<T>`、リクエストスコープ、キャッシングを備えたFastAPIスタイルのDIシステム
- **自動スキーマ生成**: `#[derive(Schema)]`でRustの型からOpenAPIスキーマを導出
- **関数ベースのエンドポイント**: APIルートを定義するための人間工学的な`#[endpoint]`マクロ (近日公開)
- **バックグラウンドタスク**: シンプルなバックグラウンドタスク実行

## ドキュメント

- 📚 [Getting Started Guide](docs/GETTING_STARTED.md) - 初心者向けステップバイステップチュートリアル
- 🎛️ [Feature Flags Guide](docs/FEATURE_FLAGS.md) - きめ細かい機能制御でビルドを最適化
- 📖 [API Reference](https://docs.rs/reinhardt) (近日公開予定)
- 📝 [Tutorials](docs/tutorials/) - 実際のアプリケーションを構築しながら学ぶ

## 💬 ヘルプを得る

Reinhardtはコミュニティ駆動のプロジェクトです。サポートを受けられる場所:

- 💬 **Discord**: リアルタイムチャット用のDiscordサーバーに参加 (近日公開)
- 💭 **GitHub Discussions**: [質問したりアイデアを共有](https://github.com/yourusername/reinhardt/discussions)
- 🐛 **Issues**: [バグを報告](https://github.com/yourusername/reinhardt/issues)
- 📖 **Documentation**: [ガイドを読む](docs/)

質問する前に、以下を確認してください:
- ✅ [Getting Started Guide](docs/GETTING_STARTED.md)
- ✅ [Examples](examples/)
- ✅ 既存のGitHub IssuesとDiscussions

## 🤝 コントリビューション

コントリビューションを歓迎します！開始するには[Contributing Guide](CONTRIBUTING.ja.md)をお読みください。

**クイックリンク**:
- [開発環境のセットアップ](CONTRIBUTING.ja.md#開発環境のセットアップ)
- [テストガイドライン](CONTRIBUTING.ja.md#テストガイドライン)
- [コミットガイドライン](CONTRIBUTING.ja.md#コミットガイドライン)

## ライセンス

以下のいずれかのライセンスで提供されます:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) または http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) または http://opensource.org/licenses/MIT)

お好きな方をお選びください。

### サードパーティの帰属表示

このプロジェクトは以下にインスパイアされています:

- [Django](https://www.djangoproject.com/) (BSD 3-Clause License)
- [Django REST Framework](https://www.django-rest-framework.org/) (BSD 3-Clause License)
- [FastAPI](https://fastapi.tiangolo.com/) (MIT License)
- [SQLAlchemy](https://www.sqlalchemy.org/) (MIT License)

詳細な帰属表示については[THIRD-PARTY-NOTICES](THIRD-PARTY-NOTICES)を参照してください。

**注意:** このプロジェクトはDjango Software Foundation、Encode OSS Ltd.、Sebastián Ramírez氏(FastAPI作者)、またはMichael Bayer氏(SQLAlchemy作者)と提携または承認されたものではありません。
