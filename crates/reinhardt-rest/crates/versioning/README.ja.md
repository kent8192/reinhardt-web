# reinhardt-versioning

Reinhardtフレームワーク向けのAPIバージョニング戦略。Django REST Frameworkにインスパイアされています。

## ステータス

✅ **実装完了** - すべての機能が実装され、テスト済みです

## 機能

### ✅ 実装済み

#### コアバージョニング戦略

1. **URLPathVersioning** - URLパスからのバージョン検出
   - パスセグメントからバージョンを抽出（例: `/v1/users/`, `/v2.0/api/`）
   - 柔軟なバージョン抽出のためのカスタマイズ可能な正規表現パターン
   - `with_pattern()`メソッドによるパターン設定
   - デフォルトフォールバックバージョンのサポート
   - 許可されたバージョンの検証
   - 例: `/v1/`, `/v2.0/`, `/api/v3/`

2. **AcceptHeaderVersioning** - Acceptヘッダーからのバージョン検出
   - Acceptヘッダーのメディアタイプパラメータを解析（例: `Accept: application/json; version=2.0`）
   - 設定可能なバージョンパラメータ名
   - 厳格なバージョン検証
   - デフォルトバージョンへのフォールバック
   - 標準的なメディアタイプネゴシエーションとの互換性
   - 例: `Accept: application/json; version=1.0`

3. **QueryParameterVersioning** - クエリパラメータからのバージョン検出
   - クエリ文字列からバージョンを抽出（例: `?version=1.0`, `?v=2.0`）
   - `with_version_param()`によるカスタマイズ可能なパラメータ名
   - 複数パラメータのサポート
   - デフォルトバージョンへのフォールバック
   - 例: `?version=1.0`, `?v=2.0`

4. **HostNameVersioning** - サブドメインからのバージョン検出
   - ホスト名のサブドメインからバージョンを抽出（例: `v1.api.example.com`）
   - ホスト名解析のためのカスタマイズ可能な正規表現パターン
   - `with_host_format()`によるホストフォーマット設定
   - `with_hostname_pattern()`によるホスト名パターンマッピング
   - ポート処理のサポート
   - 例: `v1.api.example.com`, `api-v2.example.com`

5. **NamespaceVersioning** - URL名前空間からのバージョン検出
   - バージョン抽出のためのルーター名前空間統合
   - 設定可能な名前空間パターン（例: `/v{version}/`）
   - `with_namespace_prefix()`による名前空間プレフィックスのサポート
   - ルーター統合メソッド:
     - `extract_version_from_router()` - ルーターパスからバージョンを抽出
     - `get_available_versions_from_router()` - 登録されているすべてのバージョンを取得
   - パターンベースのバージョン抽出
   - 例: `/v1/users/`, `/api/v2/posts/`

#### ミドルウェアシステム

6. **VersioningMiddleware** - 自動バージョン検出と注入
   - 任意の`BaseVersioning`戦略と統合
   - リクエストからの自動バージョン抽出
   - リクエストエクステンションにバージョンを保存
   - 無効なバージョンのエラーハンドリング
   - ミドルウェア合成のためのクローンサポート
   - バージョニング戦略に対するゼロコスト抽象化

7. **RequestVersionExt** - リクエストからの型安全なバージョンアクセス
   - `version()` - バージョンを`Option<String>`として取得
   - `version_or()` - フォールバックデフォルトを持つバージョンを取得
   - リクエストエクステンションとのシームレスな統合
   - 型安全なバージョン取得

8. **ApiVersion** - バージョンデータ型
   - `as_str()` - バージョンを文字列スライスとして取得
   - `to_string()` - バージョンを所有String型として取得
   - `new()` - 新しいバージョンインスタンスを作成
   - CloneとDebugのサポート

#### ハンドラー統合

9. **VersionedHandler** - バージョン対応ハンドラーのトレイト
   - `handle_versioned()` - バージョンコンテキストを使用してリクエストを処理
   - `supported_versions()` - サポートされているバージョンのリストを取得
   - `supports_version()` - バージョンサポートを確認

10. **VersionedHandlerWrapper** - ハンドラートレイトアダプタ
    - `VersionedHandler`を標準の`Handler`トレイトと互換性を持たせる
    - 自動バージョン決定
    - 処理前のバージョン検証
    - サポートされていないバージョンのエラーハンドリング

11. **SimpleVersionedHandler** - シンプルなバージョン-レスポンスマッパー
    - バージョンを静的レスポンスにマッピング
    - `with_version_response()` - バージョン固有のレスポンスを追加
    - `with_default_response()` - フォールバックレスポンスを設定
    - HashMapベースのレスポンス検索

12. **ConfigurableVersionedHandler** - 高度なハンドラー設定
    - バージョンを異なるハンドラー実装にマッピング
    - `with_version_handler()` - バージョン固有のハンドラーを追加
    - `with_default_handler()` - フォールバックハンドラーを設定
    - バージョンに基づく動的ハンドラーディスパッチ

13. **VersionedHandlerBuilder** - ハンドラーのビルダーパターン
    - ハンドラー構築のための流暢なAPI
    - バージョン-ハンドラーマッピング
    - デフォルトハンドラー設定
    - 自動ラッパー統合

14. **VersionResponseBuilder** - バージョンメタデータ付きレスポンスビルダー
    - `with_data()` - レスポンスデータを追加
    - `with_field()` - 個別フィールドを追加
    - `with_version_info()` - バージョンメタデータを追加
    - `version()` - 現在のバージョンを取得
    - JSONシリアライゼーションのサポート

15. **versioned_handler!** - ハンドラー作成を簡単にするマクロ
    - バージョンマッピングのための宣言的構文
    - オプションのデフォルトハンドラー
    - コンパイル時バージョンチェック

#### 設定システム

16. **VersioningConfig** - グローバル設定
    - 一元化されたバージョニング設定
    - 戦略設定
    - デフォルトおよび許可されたバージョン
    - 厳格モードの強制
    - バージョンパラメータのカスタマイズ
    - ホスト名パターンマッピング
    - ビルダーパターンAPI
    - シリアライゼーション/デシリアライゼーションのサポート

17. **VersioningStrategy** - 戦略列挙型
    - 5つの戦略バリアント:
      - `AcceptHeader` - Acceptヘッダーバージョニング
      - `URLPath { pattern }` - カスタムパターンを持つURLパス
      - `QueryParameter { param_name }` - カスタム名を持つクエリパラメータ
      - `HostName { patterns }` - パターンマッピングを持つホスト名
      - `Namespace { pattern }` - カスタムパターンを持つ名前空間
    - 設定ファイルのためのSerdeサポート
    - JSON/YAML互換

18. **VersioningManager** - 設定管理
    - 設定からバージョニングインスタンスを作成
    - 動的設定更新
    - `config()` - 現在の設定を取得
    - `versioning()` - バージョニングインスタンスを取得
    - `update_config()` - 実行時に設定を更新
    - `from_env()`による環境変数サポート

19. **環境設定** - 環境変数のサポート
    - `REINHARDT_VERSIONING_DEFAULT_VERSION` - デフォルトバージョン
    - `REINHARDT_VERSIONING_ALLOWED_VERSIONS` - カンマ区切りの許可されたバージョン
    - `REINHARDT_VERSIONING_STRATEGY` - 戦略タイプ
    - `REINHARDT_VERSIONING_STRICT_MODE` - 厳格モードの有効/無効

#### URLリバースシステム

20. **VersionedUrlBuilder** - バージョン付きURL構築
    - 適切な場所にバージョンを含むURLを構築
    - 戦略を意識したURL生成
    - `build()` - デフォルトバージョンでURLを構築
    - `build_with_version()` - 特定のバージョンでURLを構築
    - `build_all_versions()` - すべての許可されたバージョンでURLを構築
    - 5つのバージョニング戦略すべてをサポート

21. **UrlReverseManager** - 複数ビルダー管理
    - 名前付きビルダー登録
    - デフォルトビルダーサポート
    - `add_builder()` - 名前付きビルダーを登録
    - `with_default_builder()` - デフォルトビルダーを設定
    - `build_url()` - 名前付きビルダーでURLを構築
    - `build_default_url()` - デフォルトビルダーでURLを構築
    - `build_all_urls()` - すべてのビルダーでURLを構築

22. **ApiDocUrlBuilder** - APIドキュメントURLビルダー
    - OpenAPIスキーマURL
    - Swagger UI URL
    - ReDoc URL
    - カスタムフォーマットサポート
    - バージョン固有のドキュメントパス
    - 例:
      - `/v1.0/openapi.json`
      - `/v2.0/swagger-ui/`
      - `/v1.0/redoc/`

23. **ApiDocFormat** - ドキュメントフォーマット列挙型
    - `OpenApi` - OpenAPI 3.0 JSON
    - `Swagger` - Swagger UI
    - `ReDoc` - ReDocドキュメント
    - `Custom(String)` - カスタムフォーマット

24. **versioned_url!** - URL構築のためのマクロ
    - URL構築のためのシンプルな構文
    - バージョンオーバーライドのサポート
    - 型安全なURL生成

#### テストと品質

25. **包括的なテストカバレッジ**
    - すべてのモジュールにわたる29以上のユニットテスト
    - 11以上の統合テスト
    - すべてのバージョニング戦略のテスト
    - ミドルウェア統合テスト
    - ハンドラーシステムのテスト
    - URL構築テスト
    - 設定シリアライゼーションテスト

26. **テストユーティリティ** - `test_utils`モジュール
    - `create_test_request()` - テスト用のモックリクエストを作成
    - ヘッダーのカスタマイズ
    - URIのカスタマイズ
    - テストスイート全体で再利用可能

27. **完全なドキュメント**
    - 包括的なrustdocコメント
    - すべての公開APIのコード例
    - 各モジュールの使用例
    - 統合例

#### エラーハンドリング

28. **VersioningError** - 包括的なエラー型
    - `InvalidAcceptHeader` - 不正なAcceptヘッダー
    - `InvalidURLPath` - 無効なURLパス形式
    - `InvalidNamespace` - 無効な名前空間形式
    - `InvalidHostname` - 無効なホスト名形式
    - `InvalidQueryParameter` - 無効なクエリパラメータ
    - `VersionNotAllowed` - 許可されたリストにないバージョン
    - `reinhardt_apps::Error`との統合

#### トレイトと抽象化

29. **BaseVersioning** - コアバージョニングトレイト
    - `determine_version()` - リクエストからバージョンを抽出
    - `default_version()` - デフォルトバージョンを取得
    - `allowed_versions()` - 許可されたバージョンを取得
    - `is_allowed_version()` - バージョンの有効性を確認
    - `version_param()` - バージョンパラメータ名を取得
    - 非同期バージョン検出のための非同期トレイト
    - スレッドセーフのためのSend + Sync

### 予定

なし - すべての機能が実装済みです

## クイックスタート

## 基本的な使い方

```rust
use reinhardt_versioning::{URLPathVersioning, VersioningMiddleware, RequestVersionExt};

// バージョニング戦略を作成
let versioning = URLPathVersioning::new()
    .with_default_version("1.0")
    .with_allowed_versions(vec!["1.0", "2.0"]);

// ミドルウェアとして使用
let middleware = VersioningMiddleware::new(versioning);

// ハンドラー内でアクセス
async fn handler(request: Request) -> Result<Response> {
    let version = request.version().unwrap_or_else(|| "1.0".to_string());
    // ここにバージョン固有のロジック
}
```

## グローバル設定

```rust
use reinhardt_versioning::{VersioningConfig, VersioningManager, VersioningStrategy};

// バージョニングをグローバルに設定
let config = VersioningConfig {
    default_version: "1.0".to_string(),
    allowed_versions: vec!["1.0".to_string(), "2.0".to_string()],
    strategy: VersioningStrategy::URLPath {
        default_version: Some("1.0".to_string()),
        allowed_versions: Some(vec!["1.0".to_string(), "2.0".to_string()]),
        pattern: Some("/v{version}/".to_string()),
    },
};

let manager = VersioningManager::new(config);
```

## ハンドラー統合

```rust
use reinhardt_versioning::{VersionedHandlerBuilder, SimpleVersionedHandler};

// バージョン付きハンドラーを作成
let v1_handler = Arc::new(
    SimpleVersionedHandler::new()
        .with_version_response("1.0", r#"{"version": "1.0"}"#)
);

let v2_handler = Arc::new(
    SimpleVersionedHandler::new()
        .with_version_response("2.0", r#"{"version": "2.0"}"#)
);

// バージョン付きハンドラーを構築
let handler = VersionedHandlerBuilder::new(versioning)
    .with_version_handler("1.0", v1_handler)
    .with_version_handler("2.0", v2_handler)
    .build();
```

## URLリバースサポート

```rust
use reinhardt_versioning::{VersionedUrlBuilder, VersioningStrategy};

// URLビルダーを作成
let url_builder = VersionedUrlBuilder::with_strategy(
    versioning,
    "https://api.example.com",
    VersioningStrategy::URLPath,
);

// バージョン付きURLを生成
let v1_url = url_builder.with_version("1.0").build("/users");
// 結果: "https://api.example.com/v1.0/users"

let v2_url = url_builder.with_version("2.0").build("/users");
// 結果: "https://api.example.com/v2.0/users"
```

## ドキュメント

詳細なAPI使用方法については、インラインドキュメントを参照してください。

## ライセンス

MIT OR Apache-2.0
