# reinhardt-apps

Reinhardtフレームワーク向けのDjangoインスパイアなアプリケーション設定とレジストリシステムです。

## 概要

`reinhardt-apps`は、Reinhardtプロジェクト内でDjango風のアプリケーションを管理するためのコアインフラストラクチャを提供します。ランタイムにおける文字列ベースの仕組みと、コンパイル時における型安全なアプリケーションレジストリメカニズムの両方を実装し、必須のフレームワークコンポーネントの包括的な再エクスポートも提供します。

このクレートは高レベルの統合ポイントとして機能し、HTTP処理、設定管理、エラーハンドリング、サーバー機能を統一されたAPIにまとめます。

## 機能

### 実装済み ✓

#### アプリケーションレジストリシステム

- **AppConfig**: バリデーション付きアプリケーション設定
  - 名前とラベルの管理
  - 詳細名(verbose name)のサポート
  - デフォルト自動フィールドの設定
  - パス管理
  - ラベルバリデーション(Rust識別子ルール)
- **Apps Registry**: グローバルアプリケーションレジストリ
  - アプリケーションの登録と検索
  - インストール済みアプリの追跡
  - 重複検出(ラベルと名前)
  - 準備状態の管理(apps_ready, models_ready, ready)
  - テスト用のキャッシュクリア
- **グローバルレジストリ関数**:
  - `get_apps()`: グローバルシングルトンレジストリへのアクセス
  - `init_apps()`: 文字列リストによる初期化
  - `init_apps_checked()`: コンパイル時検証付きリストによる初期化

#### 型安全なアプリケーションレジストリ

- **AppLabel Trait**: コンパイル時のアプリケーションアイデンティティ
  - 定数ラベル定義
  - 型安全なアプリケーション参照
- **型安全なメソッド**:
  - `get_app_config_typed::<A>()`: 型安全な設定の検索
  - `is_installed_typed::<A>()`: 型安全なインストールチェック
- **利点**: アプリケーション名のコンパイル時検証

#### HTTPリクエスト/レスポンス

- **Request**: 包括的なHTTPリクエストハンドリング
  - クエリパラメータの解析とアクセス
  - パスパラメータの保存
  - JSONデシリアライゼーション
  - HTTPメソッドサポート(GET, POST, PUT, DELETE, PATCH)
  - HTTPバージョン追跡(HTTP/1.1, HTTP/2)
  - ヘッダー管理
  - ボディハンドリング
  - パス抽出
- **Response**: HTTPレスポンス用のビルダーパターン
  - ステータスコードヘルパー(ok, created, no_content, bad_request, unauthorized, forbidden, not_found, internal_server_error)
  - JSONシリアライゼーション(`with_json`)
  - ボディ設定(`with_body`)
  - ヘッダー管理(`with_header`)
  - メソッドチェーンのサポート
- **ステータスコードユーティリティ**: カテゴリチェック関数
  - 情報レスポンス(1xx)
  - 成功(2xx)
  - リダイレクト(3xx)
  - クライアントエラー(4xx)
  - サーバーエラー(5xx)

#### 国際化(i18n)サポート

- **Accept-Languageヘッダーのパース**:
  - 品質値(q値)のパース
  - 複数言語のサポート
  - 言語コードのバリデーション(BCP 47準拠)
  - 品質ベースのソート
  - 最大長のバリデーション(255文字)
  - 無効なコードのフィルタリング(ワイルドカード、不正な形式のコード)
- **言語Cookieハンドリング**:
  - Cookieベースの言語抽出
  - 言語コードのバリデーション
  - カスタムCookie名のサポート
- **ヘルパーメソッド**:
  - `get_accepted_languages()`: Accept-Languageヘッダーのパースとソート
  - `get_preferred_language()`: 最も高い品質値の言語を取得
  - `get_language_from_cookie()`: Cookieから言語を抽出

#### 設定管理

- **Settings Struct**: Django風の完全な設定
  - ベースディレクトリとシークレットキー
  - デバッグモード
  - 許可ホスト
  - インストール済みアプリリスト
  - ミドルウェア設定
  - データベース設定(複数データベースのサポート)
  - テンプレート設定
  - 静的ファイル(URLとルート)
  - メディアファイル(URLとルート)
  - 国際化設定(language_code, time_zone, use_i18n, use_tz)
  - デフォルト自動フィールド
  - ルートURLconf
- **データベース設定**:
  - SQLiteサポート(`DatabaseConfig::sqlite`)
  - PostgreSQLサポート(`DatabaseConfig::postgresql`)
  - MySQLサポート(`DatabaseConfig::mysql`)
  - カスタムデータベースエンジン
- **テンプレート設定**:
  - バックエンド選択
  - テンプレートディレクトリ
  - アプリディレクトリのサポート
  - コンテキストプロセッサ
  - オプション管理
- **ミドルウェア設定**:
  - パスベースのミドルウェア指定
  - ミドルウェアごとのカスタムオプション
  - デフォルトミドルウェアスタック
- **ビルダーパターン**:
  - `with_validated_apps()`: コンパイル時検証付きアプリの追加
  - `with_root_urlconf()`: URL設定の設定
  - `add_installed_app()`: 単一アプリの追加
  - `add_middleware()`: 単一ミドルウェアの追加

#### エラーハンドリング

- **エラー型**:
  - Http (400)
  - Database (500)
  - Serialization (400)
  - Validation (400)
  - Authentication (401)
  - Authorization (403)
  - NotFound (404)
  - Internal (500)
  - Other (anyhow統合, 500)
- **エラー変換**:
  - `From<anyhow::Error>`実装
  - `Into<Response>`実装(JSONボディ付き)
- **ステータスコードマッピング**: 自動HTTPステータスコード割り当て
- **表示フォーマット**: ユーザーフレンドリーなエラーメッセージ
- **Result型**: フレームワーク全体で使用する`Result<T>`エイリアス

#### 再エクスポート

- **HTTP**: `Request`, `Response`, `StreamBody`, `StreamingResponse` (reinhardt-httpより)
- **Settings**: `Settings`, `DatabaseConfig`, `MiddlewareConfig`, `TemplateConfig` (reinhardt-settingsより)
- **Errors**: `Error`, `Result` (reinhardt-exceptionより)
- **Server**: `serve`, `HttpServer` (reinhardt-serverより)
- **Types**: `Handler`, `Middleware`, `MiddlewareChain` (reinhardt-typesより)
- **Apps**: `AppConfig`, `AppError`, `AppResult`, `Apps`, `get_apps`, `init_apps`, `init_apps_checked`
- **Builder**: `Application`, `ApplicationBuilder`, `ApplicationDatabaseConfig`, `BuildError`, `BuildResult`, `RouteConfig`

#### アプリケーションビルダーシステム

- **ApplicationBuilder**: アプリケーション設定用の流暢なビルダーパターン
  - `add_app()`と`add_apps()`でアプリケーションを追加
  - `add_middleware()`と`add_middlewares()`でミドルウェアを追加
  - `add_url_pattern()`と`add_url_patterns()`でURLパターンを追加
  - データベース設定のサポート
  - カスタム設定の管理
  - 設定のバリデーション(重複チェック)
  - メソッドチェーンのサポート
- **RouteConfig**: メタデータを持つルート定義
  - パスとハンドラー名の設定
  - オプションのルート名
  - オプションの名前空間のサポート
  - フルネーム生成(namespace:name)
- **ApplicationDatabaseConfig**: データベース設定
  - URLベースの設定
  - 接続プールサイズの設定
  - 最大オーバーフロー接続数
  - 接続タイムアウトの設定
- **Application**: 完全な設定アクセスを持つビルド済みアプリケーション
  - 登録済みアプリ、ミドルウェア、URLパターンへのアクセス
  - データベース設定の取得
  - カスタム設定へのアクセス
  - Appsレジストリ統合
  - 準備状態の検証

### 予定

#### アプリケーションレジストリの機能強化

- モデルの検出と登録
- 逆方向リレーションの構築
- Readyフック(AppConfig.ready())
- アプリライフサイクルイベント用のシグナル統合
- マイグレーション検出

#### 高度な設定機能

- 環境変数の統合
- 設定のバリデーション
- 設定の継承(ベース設定 + 環境固有)
- セキュアな設定(シークレットキー生成、機密データハンドリング)
- 設定の凍結(初期化後に不変)

#### 強化されたエラーハンドリング

- エラーコードシステム
- ローカライズされたエラーメッセージ
- エラー詳細とコンテキスト
- 構造化ロギング統合
- エラー集約

#### リクエストの機能強化

- フォームデータのパース
- マルチパートファイルアップロードのサポート
- クエリパラメータのURLデコード
- リクエストコンテキスト管理
- セッション統合
- 認証/ユーザー統合
- CSRFトークンハンドリング

#### レスポンスの機能強化

- レスポンス圧縮(gzip, brotli)
- ストリーミングレスポンスヘルパー
- リダイレクトヘルパー(permanent, temporary)
- コンテンツネゴシエーション
- ETagサポート
- キャッシュコントロールヘッダー

#### テストユーティリティ

- テストクライアント
- モックリクエスト/レスポンスビルダー
- アプリケーションレジストリフィクスチャ
- データベーステスト分離

## 使用方法

### アプリケーションレジストリ

```rust
use reinhardt_apps::{AppConfig, Apps, get_apps, init_apps_checked};

// アプリケーションを定義
let app1 = AppConfig::new("myapp", "myapp")
    .with_verbose_name("My Application")
    .with_default_auto_field("BigAutoField");

// アプリケーションを登録
let apps = Apps::new(vec!["myapp".to_string()]);
apps.register(app1)?;

// インストールをチェック
assert!(apps.is_installed("myapp"));

// 設定を取得
let config = apps.get_app_config("myapp")?;

// グローバルレジストリを初期化
init_apps_checked(|| vec!["myapp".to_string()])?;
let global_apps = get_apps();
```

### 型安全なアプリケーションレジストリ

```rust
use reinhardt_apps::{Apps, AppLabel};

// 型安全なアプリケーションを定義
struct AuthApp;
impl AppLabel for AuthApp {
    const LABEL: &'static str = "auth";
}

let apps = Apps::new(vec!["auth".to_string()]);

// 型安全なチェック(コンパイル時検証済み)
assert!(apps.is_installed_typed::<AuthApp>());
let config = apps.get_app_config_typed::<AuthApp>()?;
```

### リクエストハンドリング

```rust
use reinhardt_apps::Request;
use hyper::{Method, Uri, HeaderMap, Version};
use bytes::Bytes;

// リクエストを作成
let request = Request::new(
    Method::GET,
    Uri::from_static("/api/users?page=1"),
    Version::HTTP_11,
    HeaderMap::new(),
    Bytes::new(),
);

// クエリパラメータにアクセス
assert_eq!(request.query_params.get("page"), Some(&"1".to_string()));

// JSONボディをパース
#[derive(Deserialize)]
struct User { name: String }
let user: User = request.json()?;
```

### レスポンスビルディング

```rust
use reinhardt_apps::Response;
use serde_json::json;

// シンプルなレスポンス
let response = Response::ok().with_body("Hello, world!");

// JSONレスポンス
let data = json!({"message": "Success"});
let response = Response::ok().with_json(&data)?;

// カスタムヘッダー
let response = Response::created()
    .with_json(&data)?
    .with_header(
        hyper::header::LOCATION,
        hyper::header::HeaderValue::from_static("/api/users/1")
    );
```

### 国際化

```rust
use reinhardt_apps::Request;

// Accept-Languageヘッダーをパース
let languages = request.get_accepted_languages();
for (lang, quality) in languages {
    println!("{}: {}", lang, quality);
}

// 優先言語を取得
if let Some(lang) = request.get_preferred_language() {
    println!("User prefers: {}", lang);
}

// Cookieから言語を取得
if let Some(lang) = request.get_language_from_cookie("django_language") {
    println!("Cookie language: {}", lang);
}
```

### 設定管理

```rust
use reinhardt_apps::{Settings, DatabaseConfig, TemplateConfig};
use std::path::PathBuf;

// 設定を作成
let settings = Settings::new(
    PathBuf::from("/project"),
    "secret-key".to_string()
)
.with_validated_apps(|| vec!["myapp".to_string()])
.with_root_urlconf("config.urls");

// データベース設定
let db = DatabaseConfig::postgresql(
    "mydb",
    "user",
    "password",
    "localhost",
    5432
);

// テンプレート設定
let template = TemplateConfig::default()
    .add_dir("/templates")
    .add_dir("/other_templates");
```

### エラーハンドリング

```rust
use reinhardt_apps::{Error, Result, Response};

fn handle_request() -> Result<Response> {
    // 特定のエラー型を返す
    if !authenticated {
        return Err(Error::Authentication("Invalid token".into()));
    }

    if !authorized {
        return Err(Error::Authorization("Permission denied".into()));
    }

    // HTTPレスポンスへの自動変換
    Ok(Response::ok().with_body("Success"))
}

// エラーは自動的に適切なHTTPレスポンスに変換される
let response: Response = handle_request()
    .unwrap_or_else(|err| err.into());
```

### アプリケーションビルダー

```rust
use reinhardt_apps::{
    ApplicationBuilder, ApplicationDatabaseConfig, AppConfig, RouteConfig
};

// 完全なアプリケーションを構築
let app = ApplicationBuilder::new()
    // アプリケーションを追加
    .add_app(AppConfig::new("myapp", "myapp").with_verbose_name("My Application"))
    .add_app(AppConfig::new("auth", "auth"))

    // ミドルウェアスタックを追加
    .add_middleware("CorsMiddleware")
    .add_middleware("AuthMiddleware")

    // ルートを設定
    .add_url_pattern(
        RouteConfig::new("/api/users/", "UserListHandler")
            .with_namespace("api")
            .with_name("user-list")
    )
    .add_url_pattern(
        RouteConfig::new("/api/posts/", "PostListHandler")
            .with_namespace("api")
            .with_name("post-list")
    )

    // データベースを設定
    .database(
        ApplicationDatabaseConfig::new("postgresql://localhost/mydb")
            .with_pool_size(10)
            .with_max_overflow(5)
            .with_timeout(30)
    )

    // カスタム設定を追加
    .add_setting("DEBUG", "true")
    .add_setting("SECRET_KEY", "super-secret")

    // アプリケーションをビルド
    .build()
    .expect("Failed to build application");

// 設定にアクセス
assert!(app.apps_registry().is_installed("myapp"));
assert_eq!(app.middleware().len(), 2);
assert_eq!(app.url_patterns().len(), 2);
assert!(app.database_config().is_some());
assert_eq!(app.settings().get("DEBUG"), Some(&"true".to_string()));
```

## 他のクレートとの統合

このクレートは以下のReinhardtコンポーネントを統合します:

- `reinhardt-http`: HTTPリクエスト/レスポンス抽象化
- `reinhardt-settings`: 設定管理
- `reinhardt-exception`: エラー型とハンドリング
- `reinhardt-server`: HTTPサーバー実装
- `reinhardt-types`: コアトレイトと型定義

## テスト

このクレートには包括的なテストカバレッジが含まれています:

- `src/apps.rs`のユニットテスト(アプリケーションレジストリ)
- `tests/`ディレクトリの統合テスト:
  - `installed_apps_integration.rs`: レジストリ統合
  - `test_settings.rs`: 設定の構成
  - `test_request.rs`: リクエストハンドリング
  - `test_response.rs`: レスポンスビルディングとステータスコード
  - `test_error.rs`: エラーハンドリングと変換
  - `i18n_http_tests.rs`: 国際化機能

## ライセンス

Apache License, Version 2.0またはMITライセンスのいずれかでライセンスされています。
