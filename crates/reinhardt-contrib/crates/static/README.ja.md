# reinhardt-static

Reinhardt用の静的ファイル配信と本番環境ユーティリティ

## 概要

CSS、JavaScript、画像、その他の静的アセットを配信するための静的ファイル処理機能を提供します。ファイル収集、URL生成、ストレージバックエンド、ヘルスチェック、本番デプロイのためのメトリクス収集が含まれます。

## 機能

### コア機能

#### ✓ 実装済み

- **静的ファイル設定** (`StaticFilesConfig`)
  - 収集されたファイルの静的ルートディレクトリの設定
  - 検証機能付き静的URLパス設定
  - `STATICFILES_DIRS`による複数のソースディレクトリのサポート
  - メディアURL設定と競合検出

- **ストレージバックエンド** (`Storage`トレイト)
  - `FileSystemStorage` - ローカルファイルシステムストレージ
  - `MemoryStorage` - テスト用インメモリストレージ
  - 拡張可能なストレージバックエンドシステム

- **静的ファイル検索** (`StaticFilesFinder`)
  - 複数の静的ディレクトリからファイルを検索
  - 様々なソースからのファイル収集をサポート
  - `find_all()` - 設定されたディレクトリ全体から再帰的にすべての静的ファイルを検出
  - 適切なエラーハンドリングを備えた効率的なディレクトリツリー走査

- **ハッシュ化ファイルストレージ** (`HashedFileStorage`)
  - キャッシュバスティングのためのファイルハッシュ化
  - 設定可能なハッシュアルゴリズム（MD5、SHA-256）
  - 自動ハッシュ計算とファイル名生成
  - マニフェストシステムとの統合

- **マニフェストシステム** (`ManifestStaticFilesStorage`)
  - 元のファイル名とハッシュ化されたバージョンのマッピングのためのJSONマニフェスト
  - バージョン管理されたマニフェスト形式（現在V1）
  - 本番環境での効率的な静的ファイル検索を実現
  - 事前収集されたアセットを使用したデプロイワークフローをサポート

- **メディアアセット管理** (`Media`、`HasMedia`)
  - フォームとウィジェットのためのCSSおよびJavaScript依存関係の宣言
  - メディアタイプの整理（例："all"、"screen"、"print"）
  - `<link>`および`<script>`タグのHTML生成
  - 重複防止機能付き依存関係のマージ
  - コンポーネントがアセットを宣言するためのトレイトベースシステム

- **静的ファイルハンドラ** (`StaticFileHandler`)
  - 静的ファイルのHTTPリクエスト処理
  - `mime_guess`によるMIMEタイプ検出
  - `StaticError`および`StaticResult`型によるエラーハンドリング
  - 適切なコンテンツタイプでのファイル配信
  - 自動インデックスファイル検出機能付きディレクトリ配信
  - `with_index_files()`による設定可能なインデックスファイル（デフォルト：`["index.html"]`）
  - ディレクトリに直接アクセスした際にindex.htmlを配信
  - **ETags サポート**: 条件付きリクエストのためのコンテンツベースETag生成
    - ファイルコンテンツのハッシュを使用した自動ETag生成
    - `If-None-Match`ヘッダーのサポート
    - キャッシュされたリソースに対する304 Not Modifiedレスポンス
    - `handler.rs`（`StaticFile::etag()`メソッド）で実装

- **設定の検証** (`checks`モジュール)
  - Django風の静的ファイル設定のシステムチェック
  - 複数のチェックレベル：デバッグ、情報、警告、エラー、クリティカル
  - 包括的な検証ルール：
    - `static.E001` - STATIC_ROOTが設定されていない
    - `static.E002` - STATIC_ROOTがSTATICFILES_DIRS内にある
    - `static.E003` - STATIC_URLが空
    - `static.E004` - STATICFILES_DIRSのエントリがディレクトリではない
    - `static.W001` - STATIC_ROOTがSTATICFILES_DIRSのサブディレクトリである
    - `static.W002` - STATIC_URLが'/'で始まらない
    - `static.W003` - STATIC_URLが'/'で終わらない
    - `static.W004` - STATICFILES_DIRSが空
    - `static.W005` - ディレクトリが存在しない
    - `static.W006` - STATICFILES_DIRSに重複エントリがある
    - `static.W007` - MEDIA_URLが'/'で始まらない
    - `static.W008` - MEDIA_URLが'/'で終わらない
    - `static.W009` - MEDIA_URLプレフィックスがSTATIC_URLと競合している
  - 設定問題の修正のための有用なヒント

- **ヘルスチェックシステム** (`health`モジュール)
  - ヘルスステータス監視（Healthy、Degraded、Unhealthy）
  - `async_trait`を使用した非同期ヘルスチェックトレイト
  - 一元監視のためのヘルスチェックマネージャー
  - メタデータサポート付き詳細ヘルスレポート
  - 特化したチェックのためのマーカートレイト：
    - `CacheHealthCheck` - キャッシュ関連のヘルスチェック
    - `DatabaseHealthCheck` - データベース関連のヘルスチェック
  - コンポーネントレベルのヘルスステータス追跡
  - 本番環境対応の監視統合

- **メトリクス収集** (`metrics`モジュール)
  - パフォーマンスメトリクスの追跡
  - リクエストのタイミングとプロファイリング（`RequestTimer`）
  - リクエスト固有のメトリクス（`RequestMetrics`）
  - 一元化されたメトリクス収集（`MetricsCollector`）
  - カスタム測定のための汎用メトリクスタイプ

- **ミドルウェア** (`StaticFilesMiddleware`)
  - 静的ファイルのリクエスト/レスポンス処理
  - HTTPパイプラインとの統合
  - 開発環境での自動静的ファイル配信

- **依存関係の解決** (`DependencyGraph`)
  - 静的アセット間の依存関係追跡
  - アセットの読み込み順序の解決
  - 複雑なアセット依存関係チェーンのサポート

#### 関連クレートで実装済み

- **collectstaticコマンド** （`reinhardt-commands`で実装）
  - ✓ すべてのソースから静的ファイルを収集するCLIコマンド
  - ✓ オプションの処理付きでSTATIC_ROOTにファイルをコピー
  - ✓ デプロイワークフローとの統合
  - ✓ 進捗レポートと詳細出力
  - 詳細は[reinhardt-commands](../../commands/README.md)を参照

- **GZIP圧縮** （`reinhardt-middleware`で実装）
  - ✓ 帯域幅最適化のためのレスポンス圧縮
  - ✓ 設定可能な圧縮レベル（0-9）
  - ✓ 最小サイズ閾値設定
  - ✓ コンテンツタイプフィルタリング（text/\*、application/jsonなど）
  - ✓ 自動Accept-Encoding検出
  - ✓ 有益な場合のみ圧縮（サイズチェック）
  - 詳細は[reinhardt-middleware](../../../reinhardt-middleware/README.md)を参照

- **Brotli圧縮** （`reinhardt-middleware`で実装）
  - ✓ gzipより優れた圧縮率を持つ高度な圧縮
  - ✓ 設定可能な品質レベル（Fast、Balanced、Best）
  - ✓ ウィンドウサイズ設定（10-24）
  - ✓ コンテンツタイプフィルタリング
  - ✓ 自動Accept-Encoding: br検出
  - ✓ インテリジェント圧縮（有益な場合のみ）
  - 詳細は[reinhardt-middleware](../../../reinhardt-middleware/README.md)を参照

- **Cache-Controlヘッダー管理**
  - ✓ ファイルタイプごとの設定可能なキャッシュポリシー
  - ✓ 静的アセット（CSS、JS、フォント、画像）の長期キャッシング
  - ✓ HTMLファイルの短期キャッシング
  - ✓ 柔軟なキャッシュディレクティブ（public、private、no-cache、immutableなど）
  - ✓ max-ageとs-maxageの設定
  - ✓ Varyヘッダーサポート
  - ✓ パターンベースのキャッシュポリシー

- **CDN統合**
  - ✓ マルチプロバイダーサポート（CloudFront、Fastly、Cloudflare、カスタム）
  - ✓ パスプレフィックス付きCDN URL生成
  - ✓ バージョン付きURL生成
  - ✓ HTTPS/HTTP設定
  - ✓ カスタムヘッダーサポート
  - ✓ キャッシュ無効化リクエストヘルパー
  - ✓ ワイルドカードパージサポート

- **高度なストレージバックエンド** （`storage`モジュール）
  - `S3Storage` - S3互換ストレージバックエンド（AWS S3、MinIO、LocalStack）
    - 設定可能な認証情報（アクセスキー、シークレットキー）
    - S3互換サービス用のカスタムエンドポイントサポート
    - パススタイルアドレッシング設定
    - バケット内のパスプレフィックスサポート
  - `AzureBlobStorage` - Azure Blob Storageバックエンド
    - 共有キーおよびSASトークン認証
    - Azureエミュレータ用のカスタムエンドポイントサポート
    - コンテナとパスプレフィックスの設定
  - `GcsStorage` - Google Cloud Storageバックエンド
    - サービスアカウント認証情報（JSONまたはファイルパス）
    - GCSエミュレータ用のカスタムエンドポイントサポート
    - プロジェクトIDとバケットの設定
  - `StorageRegistry` - カスタムストレージバックエンド登録システム
    - ストレージバックエンドの動的登録
    - ストレージインスタンス作成のためのファクトリーパターン
    - バックエンドライフサイクル管理（登録、登録解除、クリア）

- **テンプレート統合** （`template_integration`モジュール）
  - テンプレート内の静的ファイルURLのための`reinhardt-templates`との統合
  - `TemplateStaticConfig` - テンプレート静的ファイル生成の設定
  - `init_template_static_config()` - `StaticFilesConfig`から初期化
  - `init_template_static_config_with_manifest()` - マニフェストサポート付きで初期化
  - マニフェストを使用した自動ハッシュ化ファイル名の解決
  - Askamaの`{{ "path/to/file.css"|static }}`フィルタ構文と連携
  - カスタム静的URL（CDNなど）をサポート
  - 機能フラグ：`templates-integration`（オプション）

- **ファイル処理パイプライン** （`processing`モジュール）
  - CSS/JavaScriptのミニファイ（基本的な空白とコメントの除去）
  - 依存関係解決付きアセットバンドリング
  - 処理パイプラインマネージャー
  - 設定可能な最適化レベル
  - 機能フラグ：`processing`（デフォルト：無効）

- **開発サーバー機能** （`dev_server`モジュール）
  - `notify`クレートを使用したファイルシステム監視
  - ブロードキャストチャネルを使用した自動リロード通知システム
  - 詳細なデバッグ情報付き開発エラーページ
  - WebSocketベースのリロード通知（デフォルトはポート35729）
  - スマートなリロード戦略：
    - CSSファイル：ページ全体のリフレッシュなしでリロード
    - その他のファイル：ページ全体のリロード
  - 複数パスの監視サポート
  - クライアント接続の追跡
  - 機能フラグ：`dev-server`（デフォルト：無効）

- **高度なファイル処理**
  - 画像最適化（PNG、JPEG、WebP） - 機能フラグ：`image-optimization`
  - ソースマップ生成 - 機能フラグ：`source-maps`
  - アセット圧縮（gzip、brotli） - 機能フラグ：`compression`
  - CSSおよびJavaScriptのミニファイ
  - 依存関係解決付きアセットバンドリング

- **高度なミニファイ** （OXC駆動）
  - 変数の名前変更（マングリング） - 機能フラグ：`advanced-minification`
  - デッドコード除去
  - 本番グレード圧縮
  - console.log削除オプション
  - debuggerステートメント削除

## アーキテクチャ

### ストレージシステム

ストレージシステムは`Storage`トレイトを中心に構築され、複数のバックエンド実装を可能にします：

**ローカルストレージ**：

- **FileSystemStorage**：ローカルファイルシステムを使用するデフォルトストレージ
- **MemoryStorage**：テスト用のインメモリストレージ
- **HashedFileStorage**：コンテンツベースのハッシュ化を追加するために他のストレージバックエンドをラップ
- **ManifestStaticFilesStorage**：効率的な検索のためのマニフェスト付き本番ストレージ

**クラウドストレージ** （オプション、機能ゲート付き）：

- **S3Storage**：Amazon S3およびS3互換サービス（MinIO、LocalStack）
- **AzureBlobStorage**：Microsoft Azure Blob Storage
- **GcsStorage**：Google Cloud Storage

**拡張性**：

- **StorageRegistry**：カスタムストレージバックエンドを動的に登録・管理

### ヘルスチェック

ヘルスチェックシステムは以下を提供します：

- 非同期ヘルスチェック実行
- コンポーネントレベルのステータス追跡
- 集約されたヘルスレポート
- 拡張可能なチェック登録
- 監視システムとの統合

### メトリクス

メトリクスシステムは以下を可能にします：

- リクエストレベルのタイミング
- カスタムメトリクス収集
- パフォーマンスプロファイリング
- 本番監視統合

## 使用例

### 基本設定

```rust
use reinhardt_static::StaticFilesConfig;
use std::path::PathBuf;

let config = StaticFilesConfig {
    static_root: PathBuf::from("/var/www/static"),
    static_url: "/static/".to_string(),
    staticfiles_dirs: vec![
        PathBuf::from("app/static"),
        PathBuf::from("vendor/static"),
    ],
    media_url: Some("/media/".to_string()),
};
```

### 設定の検証

```rust
use reinhardt_static::checks::check_static_files_config;

let messages = check_static_files_config(&config);
for message in messages {
    println!("[{}] {}", message.id, message.message);
    if let Some(hint) = message.hint {
        println!("  Hint: {}", hint);
    }
}
```

### すべての静的ファイルの検索

```rust
use reinhardt_static::StaticFilesFinder;
use std::path::PathBuf;

let mut finder = StaticFilesFinder::new();
finder.add_directory(PathBuf::from("app/static"));
finder.add_directory(PathBuf::from("vendor/static"));

// 再帰的にすべての静的ファイルを検索
let all_files = finder.find_all();
for file in all_files {
    println!("Found: {}", file);
}
```

### インデックスファイル付きディレクトリ配信

```rust
use reinhardt_static::StaticFileHandler;
use std::path::PathBuf;

let handler = StaticFileHandler::new(PathBuf::from("/var/www/static"))
    .with_index_files(vec![
        "index.html".to_string(),
        "index.htm".to_string(),
        "default.html".to_string(),
    ]);

// /docs/ にアクセスすると、存在する場合 /docs/index.html を配信
```

### フォーム用メディアアセット

```rust
use reinhardt_static::media::{Media, HasMedia};

let mut media = Media::new();
media.add_css("all", "/static/css/forms.css");
media.add_js("/static/js/widgets.js");

// テンプレートでレンダリング
let css_html = media.render_css();
let js_html = media.render_js();
```

### ヘルスチェック

```rust
use reinhardt_static::health::{HealthCheckManager, HealthCheck, HealthCheckResult};
use async_trait::async_trait;
use std::sync::Arc;

struct StaticFilesHealthCheck;

#[async_trait]
impl HealthCheck for StaticFilesHealthCheck {
    async fn check(&self) -> HealthCheckResult {
        // 静的ファイルがアクセス可能かチェック
        HealthCheckResult::healthy("static_files")
            .with_metadata("static_root_exists", "true")
    }
}

let mut manager = HealthCheckManager::new();
manager.register("static", Arc::new(StaticFilesHealthCheck));

let report = manager.run_checks().await;
if report.is_healthy() {
    println!("All systems operational");
}
```

### テンプレート統合

**機能フラグ**：`templates-integration`

```toml
[dependencies]
reinhardt-static = { version = "0.1.0", features = ["templates-integration"] }
reinhardt-templates = "0.1.0"
```

#### 基本的なテンプレート統合

```rust
use reinhardt_static::{StaticFilesConfig, init_template_static_config};
use std::path::PathBuf;

// 静的ファイル設定を初期化
let config = StaticFilesConfig {
    static_root: PathBuf::from("/var/www/static"),
    static_url: "/static/".to_string(),
    staticfiles_dirs: vec![],
    media_url: None,
};

// テンプレート静的設定を初期化
init_template_static_config(&config);
```

これで、Askamaテンプレートで`static`フィルタを使用できるようになります：

```html
<!DOCTYPE html>
<html>
<head>
    <link rel="stylesheet" href="{{ "css/style.css"|static }}">
    <script src="{{ "js/app.js"|static }}"></script>
</head>
<body>
    <img src="{{ "images/logo.png"|static }}" alt="Logo">
</body>
</html>
```

これにより以下が生成されます：

```html
<!DOCTYPE html>
<html>
<head>
    <link rel="stylesheet" href="/static/css/style.css">
    <script src="/static/js/app.js"></script>
</head>
<body>
    <img src="/static/images/logo.png" alt="Logo">
</body>
</html>
```

#### マニフェスト付きテンプレート統合（ハッシュ化ファイル名）

```rust
use reinhardt_static::{ManifestStaticFilesStorage, init_template_static_config_with_manifest};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // マニフェストストレージを作成
    let storage = ManifestStaticFilesStorage::new(
        PathBuf::from("/var/www/static"),
        "/static/"
    );

    // マニフェストサポート付きでテンプレート設定を初期化
    init_template_static_config_with_manifest(&storage).await?;

    Ok(())
}
```

マニフェストファイル（`staticfiles.json`）を使用：

```json
{
  "css/style.css": "css/style.abc123def.css",
  "js/app.js": "js/app.456789abc.js",
  "images/logo.png": "images/logo.xyz987uvw.png"
}
```

同じテンプレートが、キャッシュバスティングのためのハッシュ化されたURLを生成します：

```html
<!DOCTYPE html>
<html>
<head>
    <link rel="stylesheet" href="/static/css/style.abc123def.css">
    <script src="/static/js/app.456789abc.js"></script>
</head>
<body>
    <img src="/static/images/logo.xyz987uvw.png" alt="Logo">
</body>
</html>
```

#### CDN統合

```rust
use reinhardt_static::TemplateStaticConfig;
use std::collections::HashMap;

// CDN URLで設定
let config = TemplateStaticConfig::new(
    "https://cdn.example.com/assets/".to_string()
);

// reinhardt_templates::StaticConfigに変換
let static_config = reinhardt_templates::StaticConfig::from(config);
reinhardt_templates::init_static_config(static_config);
```

テンプレートはCDN URLを生成するようになります：

```html
<link rel="stylesheet" href="https://cdn.example.com/assets/css/style.css">
```

#### 高度な使用法：カスタムマニフェスト読み込み

```rust
use reinhardt_static::TemplateStaticConfig;
use std::collections::HashMap;

// カスタムマニフェストマッピングを作成
let mut manifest = HashMap::new();
manifest.insert("app.js".to_string(), "app.v1.2.3.js".to_string());
manifest.insert("main.css".to_string(), "main.v1.2.3.css".to_string());

// カスタムマニフェストで設定
let config = TemplateStaticConfig::new("/static/".to_string())
    .with_manifest(manifest);

let static_config = reinhardt_templates::StaticConfig::from(config);
reinhardt_templates::init_static_config(static_config);
```

### ファイル処理パイプライン

**機能フラグ**：`processing`

```toml
[dependencies]
reinhardt-static = { version = "0.1.0", features = ["processing"] }
```

#### 基本的なCSS/JSミニファイ

```rust
use reinhardt_static::processing::{ProcessingPipeline, ProcessingConfig};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 処理設定を作成
    let config = ProcessingConfig::new(PathBuf::from("dist"))
        .with_minification(true)
        .with_image_optimization(false);

    // パイプラインを作成
    let pipeline = ProcessingPipeline::new(config);

    // CSSファイルを処理
    let css_content = b"body { color: red; }";
    let minified = pipeline
        .process_file(css_content, &PathBuf::from("style.css"))
        .await?;

    // JavaScriptファイルを処理
    let js_content = b"const x = 1; // comment";
    let minified_js = pipeline
        .process_file(js_content, &PathBuf::from("app.js"))
        .await?;

    Ok(())
}
```

#### アセットバンドリング

```rust
use reinhardt_static::processing::bundle::{AssetBundler, BundleConfig};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // バンドラーを作成
    let mut bundler = AssetBundler::new();

    // ファイルを追加
    bundler.add_file(
        PathBuf::from("utils.js"),
        b"export const add = (a, b) => a + b;".to_vec(),
    );
    bundler.add_file(
        PathBuf::from("main.js"),
        b"import { add } from './utils.js'; console.log(add(1, 2));".to_vec(),
    );

    // 依存関係を定義（mainはutilsに依存）
    bundler.add_dependency(
        PathBuf::from("main.js"),
        PathBuf::from("utils.js"),
    );

    // 依存順序でバンドル
    let bundle = bundler.bundle()?;

    // utils.jsがmain.jsの前に含まれる
    println!("{}", String::from_utf8_lossy(&bundle));

    Ok(())
}
```

#### 依存関係グラフ

```rust
use reinhardt_static::DependencyGraph;

let mut graph = DependencyGraph::new();

// ファイルと依存関係を追加
graph.add_dependency("app.js".to_string(), "config.js".to_string());
graph.add_dependency("app.js".to_string(), "utils.js".to_string());
graph.add_dependency("config.js".to_string(), "constants.js".to_string());

// 処理順序を解決
let order = graph.resolve_order();
// 結果: ["constants.js", "utils.js", "config.js", "app.js"]
// （依存関係が最初）
```

#### カスタムバンドル設定

```rust
use reinhardt_static::processing::bundle::{AssetBundler, BundleConfig};
use std::path::PathBuf;

let mut bundler = AssetBundler::new();
bundler.add_file(PathBuf::from("a.js"), b"const a = 1;".to_vec());
bundler.add_file(PathBuf::from("b.js"), b"const b = 2;".to_vec());
bundler.add_file(PathBuf::from("c.js"), b"const c = 3;".to_vec());

// カスタム順序でバンドル（依存関係を無視）
let bundle = bundler.bundle_files(&[
    PathBuf::from("c.js"),
    PathBuf::from("a.js"),
    PathBuf::from("b.js"),
])?;
```

#### ストレージ統合による処理

```rust,ignore
use reinhardt_static::processing::{ProcessingPipeline, ProcessingConfig};
use reinhardt_static::ManifestStaticFilesStorage;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // パイプラインを作成
    let config = ProcessingConfig::new(PathBuf::from("dist"))
        .with_minification(true);
    let pipeline = ProcessingPipeline::new(config);

    // ストレージを作成
    let storage = ManifestStaticFilesStorage::new(
        PathBuf::from("dist"),
        "/static/"
    );

    // ファイルを処理して保存
    let css_content = tokio::fs::read("src/style.css").await?;
    let minified = pipeline
        .process_file(&css_content, &PathBuf::from("style.css"))
        .await?;

    // ハッシュ化されたファイル名で保存
    storage.save("style.css", &minified).await?;

    Ok(())
}
```

#### 圧縮（GzipとBrotli）

**機能フラグ**：`compression`

```toml
[dependencies]
reinhardt-static = { version = "0.1.0", features = ["compression"] }
```

```rust
use reinhardt_static::processing::compress::{GzipCompressor, BrotliCompressor, CompressionConfig};
use reinhardt_static::processing::Processor;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Gzip圧縮
    let gzip = GzipCompressor::with_level(9);
    let input = b"Hello, World! This is test data.";
    let compressed = gzip.process(input, &PathBuf::from("test.txt")).await?;
    println!("Original: {} bytes, Compressed: {} bytes", input.len(), compressed.len());

    // Brotli圧縮（より良い圧縮率）
    let brotli = BrotliCompressor::new();
    let compressed_br = brotli.process(input, &PathBuf::from("test.txt")).await?;

    // 圧縮設定
    let config = CompressionConfig::new()
        .with_gzip(true)
        .with_brotli(true)
        .with_min_size(1024)  // 1KB以上のファイルのみ圧縮
        .add_extension("txt".to_string());

    // ファイルを圧縮すべきかチェック
    if config.should_compress(&PathBuf::from("large.js"), 5000) {
        println!("File will be compressed");
    }

    Ok(())
}
```

#### ソースマップ

**機能フラグ**：`source-maps`

```toml
[dependencies]
reinhardt-static = { version = "0.1.0", features = ["source-maps"] }
```

```rust
use reinhardt_static::processing::sourcemap::{SourceMap, SourceMapGenerator, SourceMapMerger};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ソースマップを生成
    let generator = SourceMapGenerator::new()
        .with_inline_sources(true)
        .with_source_root("/src".to_string());

    let map = generator.generate_for_file(
        &PathBuf::from("dist/app.min.js"),
        &PathBuf::from("src/app.js"),
        "const x = 1; const y = 2;"
    );

    // JSONとして保存
    let map_json = map.to_json_pretty()?;
    tokio::fs::write("dist/app.min.js.map", map_json).await?;

    // ソースマップコメントを生成
    let comment = generator.generate_comment("app.min.js.map");
    println!("Add to minified file: {}", comment);

    // 複数のソースマップをマージ
    let mut merger = SourceMapMerger::new();

    let mut map1 = SourceMap::new("file1.min.js".to_string());
    map1.add_source("src/file1.js".to_string());
    merger.add_map(map1);

    let mut map2 = SourceMap::new("file2.min.js".to_string());
    map2.add_source("src/file2.js".to_string());
    merger.add_map(map2);

    let merged = merger.merge("bundle.min.js".to_string());
    println!("Merged map has {} sources", merged.sources.len());

    Ok(())
}
```

#### 画像最適化

**機能フラグ**：`image-optimization`

```toml
[dependencies]
reinhardt-static = { version = "0.1.0", features = ["image-optimization"] }
```

```rust
use reinhardt_static::processing::images::ImageOptimizer;
use reinhardt_static::processing::Processor;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 品質設定（1-100）でオプティマイザを作成
    let optimizer = ImageOptimizer::new(85);

    // PNGを最適化
    let png_data = tokio::fs::read("image.png").await?;
    let optimized = optimizer.process(&png_data, &PathBuf::from("image.png")).await?;
    tokio::fs::write("image.optimized.png", optimized).await?;

    // JPEGを最適化
    let jpg_data = tokio::fs::read("photo.jpg").await?;
    let optimized_jpg = optimizer.process(&jpg_data, &PathBuf::from("photo.jpg")).await?;

    // カスタム設定
    let optimizer_lossless = ImageOptimizer::with_settings(100, false);

    Ok(())
}
```

#### 高度なミニファイ（本番グレード）

**機能フラグ**：`advanced-minification`

```toml
[dependencies]
reinhardt-static = { version = "0.1.0", features = ["advanced-minification"] }
```

OXCを使用した高度なミニファイは、変数の名前変更、デッドコード除去、高度な圧縮を含む本番グレードの最適化を提供します。

```rust
use reinhardt_static::processing::advanced_minify::{AdvancedJsMinifier, AdvancedMinifyConfig};
use reinhardt_static::processing::Processor;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 本番ミニファイア（本番ビルドに推奨）
    let minifier = AdvancedJsMinifier::production();

    let input = br#"
        function calculateSum(a, b) {
            console.log('Calculating sum');
            debugger;
            const result = a + b;
            return result;
        }

        const unusedVariable = 42;
    "#;

    let minified = minifier.process(input, &PathBuf::from("app.js")).await?;
    let output = String::from_utf8(minified)?;

    // 出力：変数名変更、console.log削除、debugger削除でミニファイ化
    println!("Minified: {}", output);

    Ok(())
}
```

**カスタム設定**：

```rust
use reinhardt_static::processing::advanced_minify::{AdvancedJsMinifier, AdvancedMinifyConfig};

// カスタム設定
let config = AdvancedMinifyConfig::new()
    .with_mangle(true)              // 変数名の変更を有効化
    .with_compress(true)            // 圧縮を有効化
    .with_drop_console(true)        // console.*呼び出しを削除
    .with_drop_debugger(true)       // debuggerステートメントを削除
    .with_toplevel(false)           // トップレベル変数をマングルしない
    .with_keep_fnames(false)        // 関数名を変更
    .with_keep_classnames(false);   // クラス名を変更

let minifier = AdvancedJsMinifier::with_config(config);
```

**開発モード**（最小限のミニファイ）：

```rust
use reinhardt_static::processing::advanced_minify::AdvancedJsMinifier;

// 開発ミニファイア（可読性を保持）
let minifier = AdvancedJsMinifier::development();
```

**設定プリセット**：

| プリセット | マングル | 圧縮 | Console削除 | Debugger削除 | 用途 |
|--------|--------|----------|--------------|---------------|----------|
| `production()` | ✓ | ✓ | ✓ | ✓ | 本番ビルド |
| `development()` | ✗ | ✗ | ✗ | ✗ | 開発ビルド |
| `new()`（デフォルト） | ✓ | ✓ | ✗ | ✓ | 一般的な用途 |

**パフォーマンス上の利点**：

- **ファイルサイズ削減**：基本的なミニファイと比較して40-60%
- **変数名の変更**：`myLongVariableName` → `a`
- **デッドコード削除**：到達不可能なコードを除去
- **Console削除**：デバッグステートメントを削除
- **ASTベース**：正規表現ベースのミニファイより安全

**処理パイプラインとの統合**：

```rust
use reinhardt_static::processing::{ProcessingPipeline, ProcessingConfig};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 高度なミニファイ付きパイプライン
    let config = ProcessingConfig::new(PathBuf::from("dist"))
        .with_minification(true)        // 基本的なミニファイ
        .with_advanced_minification(true); // 高度なミニファイ（機能が必要）

    let pipeline = ProcessingPipeline::new(config);

    let js_content = tokio::fs::read("src/app.js").await?;
    let optimized = pipeline.process_file(&js_content, &PathBuf::from("app.js")).await?;

    tokio::fs::write("dist/app.min.js", optimized).await?;

    Ok(())
}
```

### 開発サーバー機能

**機能フラグ**：`dev-server`

```toml
[dependencies]
reinhardt-static = { version = "0.1.0", features = ["dev-server"] }
```

#### ファイル監視と自動リロード

```rust,no_run
use reinhardt_static::{DevServerConfig, FileWatcher, AutoReload};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 開発サーバー設定を作成
    let config = DevServerConfig::new()
        .with_watch_path(PathBuf::from("./static"))
        .with_watch_path(PathBuf::from("./templates"))
        .with_auto_reload(true)
        .with_reload_port(35729);

    // ファイルウォッチャーを作成
    let paths = vec![
        PathBuf::from("./static"),
        PathBuf::from("./templates"),
    ];
    let mut watcher = FileWatcher::new(&paths)?;

    // 自動リロードシステムを作成
    let reload = AutoReload::new();

    // ファイル変更をリッスン
    loop {
        if let Some(event) = watcher.next_event().await {
            println!("File change detected: {:?}", event);
            reload.handle_watch_event(event);
        }
    }
}
```

#### ブロードキャストチャネルによる自動リロード

```rust
use reinhardt_static::{AutoReload, ReloadEvent};

#[tokio::main]
async fn main() {
    let reload = AutoReload::new();

    // クライアントがリロードイベントを購読
    let mut rx = reload.subscribe();

    // ファイル変更をシミュレート
    reload.trigger_reload();

    // イベントを受信
    if let Ok(event) = rx.try_recv() {
        match event {
            ReloadEvent::Reload => println!("Full page reload"),
            ReloadEvent::ReloadFile(path) => println!("Reload file: {}", path),
            ReloadEvent::ClearCache => println!("Clear cache"),
        }
    }
}
```

#### スマートなCSSリロード

```rust
use reinhardt_static::{AutoReload, WatchEvent};
use std::path::PathBuf;

let reload = AutoReload::new();

// CSSファイルが変更 - ページ全体のリロード不要
let event = WatchEvent::Modified(PathBuf::from("./static/css/main.css"));
reload.handle_watch_event(event);
// ReloadEvent::ReloadFile("/static/css/main.css")を送信

// JavaScriptファイルが変更 - ページ全体のリロード
let event = WatchEvent::Modified(PathBuf::from("./static/js/app.js"));
reload.handle_watch_event(event);
// ReloadEvent::Reloadを送信
```

#### 開発エラーページ

```rust
use reinhardt_static::DevelopmentErrorHandler;
use std::io;

let handler = DevelopmentErrorHandler::new()
    .with_stack_trace(true)
    .with_source_context(true)
    .with_context_lines(5);

let error = io::Error::new(io::ErrorKind::NotFound, "File not found");

// HTMLエラーページを生成
let html = handler.format_error(&error);
// 以下を含む詳細なエラーページを返す：
// - エラーメッセージ
// - スタックトレース
// - エラーチェーン
// - 有用なスタイリング

// またはプレーンテキストを生成
let text = handler.format_error_text(&error);
```

#### クライアント接続の追跡

```rust
use reinhardt_static::AutoReload;

#[tokio::main]
async fn main() {
    let reload = AutoReload::new();

    // クライアントが接続したとき
    reload.add_client().await;
    println!("Connected clients: {}", reload.client_count().await);

    // クライアントが切断したとき
    reload.remove_client().await;
    println!("Connected clients: {}", reload.client_count().await);
}
```

## ライセンス

Apache License, Version 2.0またはMITライセンスのいずれかで配布されています。
