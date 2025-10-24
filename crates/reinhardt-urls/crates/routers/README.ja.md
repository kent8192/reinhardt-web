# reinhardt-routers

Reinhardtフレームワーク用の自動URL ルーティング設定

## 概要

`reinhardt-routers` は、ReinhardtアプリケーションにDjangoスタイルのURLルーティング機能を提供します。ViewSet用のURLパターンを自動生成し、名前空間とバージョニングをサポートし、強力なURL逆引き機能を含みます。このクレートは、型安全性と柔軟性を維持しながら、一般的なREST API URLパターンを定義するためのボイラープレートコードを排除します。

## 実装済み ✓

### コアルータータイプ

- **`Router` トレイト**: モジュラーなルーティングシステムを構築するための組み合わせ可能なルーターインターフェース
- **`DefaultRouter`**: ViewSetのURL自動生成を含むフル機能のルーター実装
  - リスト/詳細エンドポイントの自動生成 (`/resource/` および `/resource/{id}/`)
  - カスタムViewSetアクションのサポート（リストレベルと詳細レベルの両方）
  - パスパラメータ抽出を含むリクエストディスパッチ
  - `reinhardt-apps`からの`Handler`トレイトとの統合
- **`UnifiedRouter`**: ネストされたルーティング構造をサポートする階層的ルーター
  - 3つのAPIスタイルの統一（FastAPIスタイル、Djangoスタイル、DRFスタイル）
  - 自動プレフィックス継承を伴うネストルーターマウント
  - 親から子へのDIコンテキスト伝播
  - ミドルウェアスタックの合成（親 → 子の順序）
  - 名前空間ベースのルート編成
  - 深さ優先のルート解決アルゴリズム

### ルート定義

- **`Route`**: パスパターンとハンドラーの合成
  - オプションの名前空間付き名前付きルート（`namespace:name`形式）
  - 逆URLルックアップのための完全な名前解決
  - 名前空間パターンからのバージョン抽出
  - バージョニング戦略のための名前空間パターンマッチング

### URLパターンマッチング

- **`PathPattern`**: Djangoスタイルのパターンパーサー
  - パラメータ抽出構文 (`/users/{id}/`)
  - 名前付きグループを使用したRegexベースのパターンマッチング
  - パラメータ名の検証と追跡
  - `is_match()`: パスがパターンに一致するかテスト
  - `extract_params()`: マッチしたパスからパスパラメータを抽出
- **`PathMatcher`**: 効率的なパスマッチングエンジン
  - 複数パターンの登録
  - パスパラメータの抽出とマッピング
  - 最初にマッチしたものが優先されるルーティング戦略

### ヘルパー関数（Djangoスタイル API）

- **`path()`**: シンプルなパラメータ構文でルートを作成
  - Djangoの`path()`関数に類似
  - URLパターン定義のためのクリーンな構文
- **`re_path()`**: 正規表現パターンを使用してルートを作成
  - Djangoの`re_path()`関数に類似
  - Djangoスタイルの正規表現パターン (`(?P<name>pattern)`) をReinhardt形式に変換
  - 複雑な正規表現処理のための`nom`を使用した高度なパーサー
  - ネストされたグループとエスケープ文字のサポート
- **`include_routes()`**: プレフィックスと名前空間でルートコレクションをインクルード
  - Djangoの`include()`関数に類似
  - 組織階層のための名前空間サポート

### URL逆引き（Djangoスタイル reverse()）

#### ランタイム文字列ベース逆引き

- **`UrlReverser`**: 名前からURLへの解決エンジン
  - 名前によるルートの登録とルックアップ
  - URLパターン内のパラメータ置換
  - 名前空間を意識したURL解決
  - ヘルパーメソッド (`reverse()`, `reverse_with()`)
  - ルートの存在チェックと名前列挙
- **`reverse()` 関数**: URL逆引き用のスタンドアロン便利関数

#### コンパイル時型安全逆引き

- **`UrlPattern` トレイト**: コンパイル時に型安全なURLパターンを定義
  - 定数パターンと名前定義
  - URL定義のゼロコスト抽象化
- **`UrlPatternWithParams` トレイト**: パラメータ付き型安全パターン
  - コンパイル時パラメータ名検証
  - パラメータ要件の強制
- **`reverse_typed()`**: シンプルなURL（パラメータなし）の型安全逆引き
- **`reverse_typed_with_params()`**: パラメータ検証付き型安全逆引き
- **`UrlParams<T>` ビルダー**: 型安全なURLを構築するための流暢なAPI
  - チェーン可能なパラメータ追加
  - コンパイル時パターンチェック
  - ランタイムパラメータ検証

### ViewSet統合

- **自動エンドポイント生成**: ViewSetから標準RESTエンドポイントを生成
  - リストエンドポイント (`GET /resource/`)
  - 作成エンドポイント (`POST /resource/`)
  - 詳細エンドポイント (`GET /resource/{id}/`)
  - 更新エンドポイント (`PUT/PATCH /resource/{id}/`)
  - 削除エンドポイント (`DELETE /resource/{id}/`)
- **カスタムアクションサポート**: カスタムViewSetアクションの登録とルーティング
  - リストレベルと詳細レベルの両方のアクション
  - カスタムURLパスと名前の設定
  - 自動アクションハンドラーラッピング
- **アクションURLマッピング**: ViewSetアクション用のURLマップを生成
  - API検出可能性のためのヘルパーメソッド `get_action_url_map()`

### バージョニングサポート

- **名前空間ベースのバージョニング**: URL名前空間を使用したAPIのバージョニング
  - パスパターンからのバージョン抽出 (`/v{version}/`)
  - 名前空間パターンによるルートフィルタリング
  - 利用可能なバージョンの列挙
- **パターンベースのバージョン検出**: URLからのバージョン番号抽出
  - 異なるバージョニングスキームのための柔軟なパターンマッチング
  - カスタムバージョン形式のサポート

### エラーハンドリング

- **`ReverseError`**: URL逆引き用の包括的なエラータイプ
  - `NotFound`: ルート名が登録されていない
  - `MissingParameter`: 必要なパラメータが提供されていない
  - `Validation`: パターン解析またはパラメータ検証エラー
- **`ReverseResult<T>`**: 逆引き操作用の型エイリアス

### 階層的ルーティング（UnifiedRouter）

- **ネストされたルーターマウント**: 階層的ルーティング構造の構築
  - `mount(prefix, child)`: 自動プレフィックス継承で子ルーターをマウント
  - `mount_mut(&mut self, prefix, child)`: 可変参照版
  - `group(namespace)`: 編成のための名前空間グループを作成
- **ビルダーパターン設定**:
  - `with_prefix(prefix)`: ルーターのURLプレフィックスを設定
  - `with_namespace(namespace)`: ルート命名の名前空間を設定
  - `with_di_context(context)`: 依存性注入コンテキストをアタッチ
  - `with_middleware(middleware)`: ルーターにミドルウェアを追加
- **自動継承**:
  - 親から子へのDIコンテキスト継承
  - ミドルウェアスタックの累積（親 → 子の順序）
  - ネストされたパスのプレフィックス連結
- **ルート解決**:
  - 深さ優先探索アルゴリズム
  - 親自身のルートより前に子ルーターをチェック
  - 完全なミドルウェアとDIコンテキスト伝播

## 計画中

### 高度な機能

- **`SimpleRouter`**: 基本的なルーティングニーズのための軽量ルーター
  - シンプルなアプリケーションのための最小限のオーバーヘッド
  - DefaultRouterの機能のサブセット
- **名前空間ベースのURL逆引き** (Phase 2 - 進行中):
  - 階層的ルート命名 (`"v1:users:detail"`)
  - ネストされた名前空間解決
  - 名前空間サポート付きURL逆引き
- **ルートキャッシング**: 大規模ルートテーブルのパフォーマンス最適化
- **カスタムコンバーター**: 型固有のパスパラメータコンバーター
  - 整数、UUID、スラッグコンバーター
  - カスタム検証ルール

### 開発者体験

- **ルートイントロスペクション**: ランタイムルート分析とデバッグ
- **OpenAPI統合**: ルートからの自動OpenAPIスキーマ生成
- **ルート可視化**: ドキュメント用のルートマップ生成

## 使用例

### DefaultRouter（従来型）

```rust
use reinhardt_routers::{DefaultRouter, Router, path, include_routes};
use reinhardt_viewsets::ViewSet;
use std::sync::Arc;

// ルーターを作成
let mut router = DefaultRouter::new();

// ViewSetを登録（自動エンドポイント生成）
let user_viewset = Arc::new(UserViewSet::new());
router.register_viewset("users", user_viewset);

// カスタムルートを追加
router.add_route(
    path("/health/", Arc::new(HealthHandler))
        .with_name("health")
);

// 名前空間付きでルートをインクルード
let api_routes = vec![/* routes */];
router.include("/api/v1", api_routes, Some("v1".to_string()));

// URL逆引き
let user_url = router.reverse_with("users-detail", &[("id", "123")]).unwrap();
// => "/users/123/"
```

### UnifiedRouter（階層的）

```rust
use reinhardt_routers::UnifiedRouter;
use reinhardt_di::InjectionContext;
use reinhardt_middleware::AuthMiddleware;
use std::sync::Arc;

// メインルーターを作成
let app = UnifiedRouter::new()
    .with_middleware(Arc::new(LoggingMiddleware));

// API v1 ルーターを作成
let api_v1 = UnifiedRouter::new()
    .with_namespace("v1")
    .with_middleware(Arc::new(AuthMiddleware));

// ユーザールーターを作成
let users_router = UnifiedRouter::new()
    .viewset("users", Arc::new(UserViewSet::new()));

// 投稿ルーターを作成
let posts_router = UnifiedRouter::new()
    .viewset("posts", Arc::new(PostViewSet::new()));

// 階層を構築
let app = app
    .mount("/api/v1",
        api_v1
            .mount("/users", users_router)
            .mount("/posts", posts_router)
    );

// 結果のURL構造:
// GET  /api/v1/users/       -> ユーザーをリスト
// POST /api/v1/users/       -> ユーザーを作成
// GET  /api/v1/users/{id}/  -> ユーザーを取得
// PUT  /api/v1/users/{id}/  -> ユーザーを更新
// DELETE /api/v1/users/{id}/ -> ユーザーを削除
// (/api/v1/posts/ も同様)

// /api/v1/users/ のミドルウェアスタック:
// 1. LoggingMiddleware (appから)
// 2. AuthMiddleware (api_v1から)
```

### UnifiedRouterによる混合APIスタイル

```rust
use reinhardt_routers::UnifiedRouter;
use hyper::Method;
use std::sync::Arc;

let router = UnifiedRouter::new()
    // FastAPIスタイル: 関数ベースエンドポイント
    .function("/health", Method::GET, health_check)

    // DRFスタイル: 自動CRUDを含むViewSet
    .viewset("users", Arc::new(UserViewSet::new()))

    // Djangoスタイル: クラスベースビュー
    .view("/about", Arc::new(AboutView));

// 3つのスタイルがシームレスに連携!
```

## 依存関係

- `reinhardt-apps`: Handlerトレイトとリクエスト/レスポンスタイプ
- `reinhardt-viewsets`: ViewSetトレイトとアクション定義
- `reinhardt-exception`: エラータイプと結果処理
- `reinhardt-di` (オプション、`unified-router` フィーチャーで): 依存性注入サポート
- `reinhardt-middleware` (オプション、`unified-router` フィーチャーで): ミドルウェアシステム
- `regex`: パターンマッチングエンジン
- `nom`: 正規表現変換用のパーサーコンビネーターライブラリ
- `async-trait`: 非同期トレイトサポート
- `hyper`: HTTPタイプ (Method, Uri, など)

## ライセンス

Apache License, Version 2.0 または MIT license のいずれかの条件でライセンスされています。
