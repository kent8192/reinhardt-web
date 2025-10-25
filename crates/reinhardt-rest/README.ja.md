# reinhardt-rest

**エクスポート専用の統合レイヤー**: Reinhardt REST APIフレームワーク用

## 概要

このクレートは、複数のReinhardtクレートを単一のインポートにまとめる**利便性レイヤー**として機能します。独自の実装やテストは含まれておらず、すべての機能は基盤となる特化したクレートによって提供されます。

## 目的

- **統一されたインターフェース**: REST API機能のための単一のインポートポイント
- **再エクスポートレイヤー**: 認証、ルーティング、ブラウザブルAPI、レスポンス処理を統合
- **実装なし**: 純粋なエクスポート/集約クレート
- **テストなし**: すべての機能は特化したクレートでテストされています

## 機能

### 実装済み ✓

#### 認証 (`reinhardt-auth`より)

- **JWT認証**: JSON Web Tokensを使用したステートレス認証
  - `JwtAuth` - JWT認証バックエンド
  - `Claims` - JWT claimsの構造
- **ユーザー型**:
  - `User` - 基本ユーザートレイト
  - `SimpleUser` - シンプルなユーザー実装
  - `AnonymousUser` - 未認証ユーザーの表現
- **パーミッションクラス**:
  - `AllowAny` - すべてのユーザーを許可(認証済み・未認証問わず)
  - `IsAuthenticated` - 認証を必須とする
  - `IsAuthenticatedOrReadOnly` - 未認証は読み取り専用、認証済みはフルアクセス
  - `IsAdminUser` - 管理者権限を必須とする
- **REST固有のユーティリティ**:
  - `AuthResult<U>` - 認証操作のための結果型
  - `AuthBackend` - 認証バックエンドトレイト

#### ルーティング (`reinhardt-routers`より)

- **ルーター型**:
  - `DefaultRouter` - ViewSetの自動URL生成を備えたデフォルトルーター
  - `Router` - 基本ルータートレイト
- **URLパターン**:
  - `Route` - 個別のルート定義
  - `UrlPattern` - URLパターンマッチング

#### ブラウザブルAPI (`reinhardt-browsable-api`より)

- **HTMLインターフェース**: 開発とテストのためのインタラクティブなAPIエクスプローラー
- **自動ドキュメンテーション**: 自己文書化されたAPIエンドポイント

#### レスポンス処理

- **レスポンス型**:
  - `ApiResponse<T>` - DRF形式のAPIレスポンスラッパー
    - 成功レスポンス(`success`, `success_with_status`)
    - エラーレスポンス(`error`, `validation_error`)
    - 標準HTTPレスポンス(`not_found`, `unauthorized`, `forbidden`)
  - `ResponseBuilder<T>` - APIレスポンスのための流暢なビルダー
- **ユーティリティ**:
  - `IntoApiResponse<T>` - 型をAPIレスポンスに変換するためのトレイト
  - `PaginatedResponse` - ページネーションされたレスポンスラッパー(`reinhardt-pagination`より)

#### スキーマ生成 (`reinhardt-openapi`より)

- **OpenAPI/Swagger**:
  - `OpenApiSchema` - OpenAPI 3.0スキーマ生成
  - `Components` - 再利用可能なスキーマコンポーネント
  - `Operation` - API操作定義
  - `Parameter` - リクエストパラメーター定義
  - `Server` - サーバー設定
  - Rust型からの自動スキーマ生成
  - `SwaggerUI` - インタラクティブなAPIドキュメンテーション

#### ページネーション (`reinhardt-pagination`より)

- **ページネーション戦略**:
  - `PageNumberPagination` - ページベースのページネーション
  - `LimitOffsetPagination` - オフセットベースのページネーション
  - `CursorPagination` - カーソルベースのページネーション

#### フィルタリング (`reinhardt-filters`より)

- **フィルターバックエンド**:
  - `SearchFilter` - 複数フィールドにわたる検索
  - `OrderingFilter` - フィールドによる結果のソート
  - `QueryFilter` - 型安全なクエリフィルタリング
  - `MultiTermSearch` - 複数用語の検索操作

#### スロットリング/レート制限 (`reinhardt-throttling`より)

- **スロットリングクラス**:
  - `AnonRateThrottle` - 未認証ユーザーのレート制限
  - `UserRateThrottle` - 認証済みユーザーのレート制限
  - `ScopedRateThrottle` - エンドポイントごとのレート制限

#### シグナル/フック (`reinhardt-signals`より)

- **モデルシグナル**:
  - `pre_save`, `post_save` - モデル保存シグナル
  - `pre_delete`, `post_delete` - モデル削除シグナル
  - `m2m_changed` - 多対多リレーションシップシグナル

### 予定

現在、このクレートの計画されているすべての機能は実装されています。

## テスト

このクレートにはテストが含まれていません。すべての機能は、基盤となる特化したクレートでテストされています:

- 認証テスト: `reinhardt-auth/tests/`
- ルーターテスト: `reinhardt-routers/tests/`
- ブラウザブルAPIテスト: `reinhardt-browsable-api/tests/`
- レスポンス処理テスト: `src/response.rs`のドキュメンテーションテスト
- 統合テスト: `tests/integration/`

## 使用例

```rust
use reinhardt_rest::{
    // 認証
    JwtAuth, IsAuthenticated, AllowAny, User, SimpleUser,

    // ルーティング
    DefaultRouter, Router, Route,

    // レスポンス処理
    ApiResponse, ResponseBuilder, IntoApiResponse,

    // ページネーション
    PaginatedResponse,
};

// 成功レスポンスの作成
let user = SimpleUser::new(1, "Alice");
let response = ApiResponse::success(user);

// カスタムレスポンスのビルド
let response = ResponseBuilder::new()
    .data("Success")
    .status(201)
    .message("Resource created")
    .build();

// ResultをApiResponseに変換
let result: Result<String, String> = Ok("data".to_string());
let response = result.into_api_response();
```
