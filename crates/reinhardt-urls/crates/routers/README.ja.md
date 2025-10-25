# reinhardt-routers

Reinhardtフレームワークの自動URL routing設定

## 概要

`reinhardt-routers`は、ReinhardtアプリケーションにDjango風のURLルーティング機能を提供します。ViewSetのURLパターンを自動生成し、名前空間とバージョニングをサポートし、強力なURL逆引き機能を含みます。このクレートは、型安全性と柔軟性を維持しながら、一般的なREST API URLパターンを定義するためのボイラープレートコードを削減します。

## 実装済み ✓

### コアRouterタイプ

- **`Router`トレイト**: モジュール式ルーティングシステムを構築するための組み合わせ可能なrouterインターフェース
- **`DefaultRouter`**: ViewSetの自動URL生成を備えた多機能router実装
  - リスト/詳細エンドポイントの自動生成（`/resource/`と`/resource/{id}/`）
  - カスタムViewSetアクションのサポート（リストレベルと詳細レベルの両方）
  - パスパラメータ抽出を含むリクエストディスパッチ
  - `reinhardt-apps`の`Handler`トレイトとの統合
- **`UnifiedRouter`**: ネストされたルーティング構造をサポートする階層的router
  - 3つのAPIスタイルの統合（FastAPI風、Django風、DRF風）
  - 自動プレフィックス継承によるネストされたrouterのマウント
  - 親から子へのDIコンテキスト伝播
  - ミドルウェアスタック構成（親→子の順序）
  - 名前空間ベースのルート構成
  - 深さ優先ルート解決アルゴリズム

### ルート定義

- **`Route`**: パスパターンとハンドラーの組み合わせ
  - オプションの名前空間付き名前付きルート（`namespace:name`形式）
  - 逆URLルックアップのための完全名解決
  - 名前空間パターンからのバージョン抽出
  - バージョニング戦略のための名前空間パターンマッチング

### URLパターンマッチング

- **`PathPattern`**: Django風のURLパターンパーサー
  - パラメータ抽出構文（`/users/{id}/`）
  - 名前付きグループによる正規表現ベースのパターンマッチング
  - パラメータ名の検証と追跡
  - `is_match()`: パスがパターンに一致するかテスト
  - `extract_params()`: 一致したパスからパスパラメータを抽出
- **`PathMatcher`**: 効率的なパスマッチングエンジン
  - 複数のパターン登録
  - パスパラメータの抽出とマッピング
  - 最初にマッチした方が優先のルーティング戦略

### ヘルパー関数（Django風API）

- **`path()`**: シンプルなパラメータ構文でルートを作成
  - Djangoの`path()`関数と同様
  - URLパターンを定義するためのクリーンな構文
- **`re_path()`**: 正規表現パターンを使用してルートを作成
  - Djangoの`re_path()`関数と同様
  - Django風の正規表現パターン（`(?P<name>pattern)`）をReinhardt形式に変換
  - 複雑な正規表現処理のための`nom`を使用した高度なパーサー
  - ネストされたグループとエスケープ文字をサポート
- **`include_routes()`**: プレフィックスと名前空間でルートコレクションをインクルード
  - Djangoの`include()`関数と同様
  - 組織的な階層のための名前空間サポート

### URL逆引き（Django風reverse()）

#### ランタイム文字列ベース逆引き

- **`UrlReverser`**: 名前からURLへの解決エンジン
  - 名前によるルート登録とルックアップ
  - URLパターン内のパラメータ置換
  - 名前空間対応URL解決
  - ヘルパーメソッド（`reverse()`、`reverse_with()`）
  - ルート存在チェックと名前列挙
- **`reverse()`関数**: URL逆引きのためのスタンドアロン便利関数

#### コンパイル時型安全逆引き

- **`UrlPattern`トレイト**: コンパイル時に型安全なURLパターンを定義
  - 定数のパターンと名前定義
  - URL定義のためのゼロコスト抽象化
- **`UrlPatternWithParams`トレイト**: パラメータ付き型安全パターン
  - コンパイル時パラメータ名検証
  - パラメータ要件の強制
- **`reverse_typed()`**: シンプルなURL（パラメータなし）の型安全逆引き
- **`reverse_typed_with_params()`**: パラメータ検証付き型安全逆引き
- **`UrlParams<T>`ビルダー**: 型安全なURLを構築するための流暢なAPI
  - チェーン可能なパラメータ追加
  - コンパイル時パターンチェック
  - ランタイムパラメータ検証

### ViewSet統合

- **自動エンドポイント生成**: ViewSetから標準RESTエンドポイントを生成
  - リストエンドポイント（`GET /resource/`）
  - 作成エンドポイント（`POST /resource/`）
  - 詳細エンドポイント（`GET /resource/{id}/`）
  - 更新エンドポイント（`PUT/PATCH /resource/{id}/`）
  - 削除エンドポイント（`DELETE /resource/{id}/`）
- **カスタムアクションサポート**: カスタムViewSetアクションの登録とルーティング
  - リストレベルと詳細レベルの両方のアクション
  - カスタムURLパスと名前の設定
  - 自動アクションハンドラーラッピング
- **アクションURLマッピング**: ViewSetアクションのURLマップを生成
  - API検出性のためのヘルパーメソッド`get_action_url_map()`

### バージョニングサポート

- **名前空間ベースバージョニング**: URL名前空間を使用したAPIのバージョニング
  - パスパターンからのバージョン抽出（`/v{version}/`）
  - 名前空間パターンによるルートフィルタリング
  - 利用可能なバージョンの列挙
- **パターンベースバージョン検出**: URLからバージョン番号を抽出
  - 異なるバージョニングスキームのための柔軟なパターンマッチング
  - カスタムバージョン形式のサポート

### エラーハンドリング

- **`ReverseError`**: URL逆引き用の包括的なエラータイプ
  - `NotFound`: ルート名が登録されていない
  - `MissingParameter`: 必須パラメータが提供されていない
  - `Validation`: パターンパースまたはパラメータ検証エラー
- **`ReverseResult<T>`**: 逆引き操作の型エイリアス

### 階層的ルーティング（UnifiedRouter）

- **ネストされたrouterマウント**: 階層的ルーティング構造を構築
  - `mount(prefix, child)`: 自動プレフィックス継承で子routerをマウント
  - `mount_mut(&mut self, prefix, child)`: 可変参照バージョン
  - `group(namespace)`: 組織化のための名前空間グループを作成
- **ビルダーパターン設定**:
  - `with_prefix(prefix)`: routerのURLプレフィックスを設定
  - `with_namespace(namespace)`: ルート命名のための名前空間を設定
  - `with_di_context(context)`: 依存性注入コンテキストをアタッチ
  - `with_middleware(middleware)`: routerにミドルウェアを追加
- **自動継承**:
  - 親から子へDIコンテキストを継承
  - ミドルウェアスタックの蓄積（親→子の順序）
  - ネストされたパスのプレフィックス連結
- **ルート解決**:
  - 深さ優先探索アルゴリズム
  - 親自身のルートの前に子routerをチェック
  - 完全なミドルウェアとDIコンテキストの伝播

## 予定

### 高度な機能

- **`SimpleRouter`**: 基本的なルーティングニーズのための軽量router
  - シンプルなアプリケーション向けの最小限のオーバーヘッド
  - DefaultRouter機能のサブセット
- **名前空間ベースURL逆引き**（フェーズ2 - 進行中）:
  - 階層的ルート命名（`"v1:users:detail"`）
  - ネストされた名前空間解決
  - 名前空間サポート付きURL逆引き
- **ルートキャッシング**: 大規模ルートテーブルのパフォーマンス最適化
- **カスタムコンバーター**: 型固有のパスパラメータコンバーター
  - Integer、UUID、slugコンバーター
  - カスタム検証ルール

### 開発者エクスペリエンス

- **ルート内省**: ランタイムルート分析とデバッグ
- **OpenAPI統合**: ルートからの自動OpenAPIスキーマ生成
- **ルート可視化**: ドキュメント用のルートマップ生成

## 使用例

### DefaultRouter（従来型）

```rust
use reinhardt_routers::{DefaultRouter, Router, path, include_routes};
use reinhardt_viewsets::ViewSet;
use std::sync::Arc;

// routerを作成
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

### UnifiedRouter（階層型）

```rust
use reinhardt_routers::UnifiedRouter;
use reinhardt_di::InjectionContext;
use reinhardt_middleware::AuthMiddleware;
use std::sync::Arc;

// メインrouterを作成
let app = UnifiedRouter::new()
    .with_middleware(Arc::new(LoggingMiddleware));

// API v1 routerを作成
let api_v1 = UnifiedRouter::new()
    .with_namespace("v1")
    .with_middleware(Arc::new(AuthMiddleware));

// ユーザーrouterを作成
let users_router = UnifiedRouter::new()
    .viewset("users", Arc::new(UserViewSet::new()));

// 投稿routerを作成
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
// GET  /api/v1/users/       -> ユーザー一覧
// POST /api/v1/users/       -> ユーザー作成
// GET  /api/v1/users/{id}/  -> ユーザー取得
// PUT  /api/v1/users/{id}/  -> ユーザー更新
// DELETE /api/v1/users/{id}/ -> ユーザー削除
// （/api/v1/posts/も同様）

// /api/v1/users/のミドルウェアスタック:
// 1. LoggingMiddleware（appから）
// 2. AuthMiddleware（api_v1から）
```

### UnifiedRouterによる混在APIスタイル

```rust
use reinhardt_routers::UnifiedRouter;
use hyper::Method;
use std::sync::Arc;

let router = UnifiedRouter::new()
    // FastAPI風: 関数ベースエンドポイント
    .function("/health", Method::GET, health_check)

    // DRF風: 自動CRUDを備えたViewSet
    .viewset("users", Arc::new(UserViewSet::new()))

    // Django風: クラスベースビュー
    .view("/about", Arc::new(AboutView));

// 3つのスタイルすべてがシームレスに連携します！
```

## 依存関係

- `reinhardt-apps`: Handlerトレイトとリクエスト/レスポンスタイプ
- `reinhardt-viewsets`: ViewSetトレイトとアクション定義
- `reinhardt-exception`: エラータイプと結果ハンドリング
- `reinhardt-di`（オプション、`unified-router`機能使用時）: 依存性注入サポート
- `reinhardt-middleware`（オプション、`unified-router`機能使用時）: ミドルウェアシステム
- `regex`: パターンマッチングエンジン
- `nom`: 正規表現変換のためのパーサーコンビネーターライブラリ
- `async-trait`: 非同期トレイトサポート
- `hyper`: HTTPタイプ（Method、Uriなど）

## ライセンス

以下のいずれかでライセンスされています:

- Apache License, Version 2.0 または
- MIT license

お好みの方を選択してください。
