# reinhardt-renderers

Django REST Frameworkにインスパイアされた、Reinhardtフレームワークのためのレスポンスレンダラー。

## 概要

レスポンスデータを様々な形式に変換するためのレンダラー。JSON形式のJSONRenderer、HTML インターフェースのBrowsableAPIRenderer、カスタムレンダラーのサポートを含みます。Acceptヘッダーに基づくコンテンツネゴシエーションを処理します。

## 実装済み ✓

### コアレンダラー

#### JSONRenderer

設定可能なフォーマットオプションでレスポンスをJSONとしてレンダリングします。

**機能:**

- 標準JSON出力
- `.pretty(true)`による整形出力のサポート
- `.ensure_ascii(true/false)`によるASCIIエンコーディング制御
- 設定可能なフォーマットオプション

**使用例:**

```rust
use reinhardt_renderers::{JSONRenderer, Renderer};
use serde_json::json;

let renderer = JSONRenderer::new()
    .pretty(true)
    .ensure_ascii(false);

let data = json!({"name": "test", "value": 123});
let result = renderer.render(&data, None).await?;
```

#### XMLRenderer

カスタマイズ可能なルート要素でレスポンスをXMLとしてレンダリングします。

**機能:**

- JSONからXMLへの自動変換
- 設定可能なルート要素名
- XML宣言を含む
- 適切なインデントとフォーマット

**使用例:**

```rust
use reinhardt_renderers::XMLRenderer;

let renderer = XMLRenderer::new()
    .root_name("data");

let result = renderer.render(&data, None).await?;
```

#### BrowsableAPIRenderer

HTMLの自己文書化APIインターフェース（`reinhardt-browsable-api`から再エクスポート）。

**機能:**

- API探索のためのインタラクティブなWebインターフェース
- フォームベースのAPIテスト
- ブラウザでの認証サポート
- レスポンスのシンタックスハイライト
- 人間に優しいHTMLレンダリング

### 専用レンダラー

#### AdminRenderer

リソース管理のためのDjangoライクな管理インターフェースレンダラー。

**機能:**

- 管理スタイルのHTMLインターフェース
- データから自動テーブル生成
- リソース作成時の確認メッセージ
- 設定可能なベースURL
- オブジェクトと配列データの両方をサポート
- 詳細URLの自動生成

**使用例:**

```rust
use reinhardt_renderers::AdminRenderer;

let renderer = AdminRenderer::new()
    .base_url("/custom-admin");
```

#### StaticHTMLRenderer

入力データを無視して、事前定義された静的HTMLコンテンツを返します。

**機能:**

- 静的HTMLコンテンツの提供
- データに依存しないレンダリング
- 静的ページやテンプレートに有用
- シンプルなコンテンツ設定

**使用例:**

```rust
use reinhardt_renderers::StaticHTMLRenderer;

let content = "<html><body><h1>Hello</h1></body></html>";
let renderer = StaticHTMLRenderer::new(content);
```

#### DocumentationRenderer

OpenAPIスキーマからAPIドキュメントをレンダリングします。

**機能:**

- HTMLドキュメント生成
- Markdownドキュメント生成
- OpenAPIスキーマのパース
- メソッドと説明を含むエンドポイントのリスト
- 設定可能な出力形式（HTMLまたはMarkdown）

**使用例:**

```rust
use reinhardt_renderers::DocumentationRenderer;

// HTML形式（デフォルト）
let renderer = DocumentationRenderer::new();

// Markdown形式
let renderer = DocumentationRenderer::new()
    .format_type("markdown");
```

#### SchemaJSRenderer

OpenAPIスキーマをSchema.jsライブラリのためのJavaScriptとしてレンダリングします。

**機能:**

- OpenAPIからJavaScriptへの変換
- ヘルパー関数生成（`getEndpoint`、`getAllPaths`）
- CommonJSモジュールエクスポートのサポート
- 適切なJavaScriptオブジェクト表記
- 有効な識別子の処理

**使用例:**

```rust
use reinhardt_renderers::SchemaJSRenderer;

let renderer = SchemaJSRenderer::new();
let js_output = renderer.render(&openapi_schema, None).await?;
```

#### CSVRenderer

カスタマイズ可能なオプションでテーブルデータをCSV形式にレンダリングします。

**機能:**

- オブジェクトの配列からCSVへの変換
- カスタマイズ可能な区切り文字（デフォルト: `,`）
- オプションのヘッダー行制御
- 自動型処理（String、Number、Bool、Null）
- 適切なCSVエスケープとクォート

**使用例:**

```rust
use reinhardt_renderers::CSVRenderer;
use serde_json::json;

let renderer = CSVRenderer::new()
    .delimiter(b';')
    .include_header(true);

let data = json!([
    {"name": "Alice", "age": 30},
    {"name": "Bob", "age": 25}
]);

let result = renderer.render(&data, None).await?;
```

#### YAMLRenderer

データをYAML形式にレンダリングします。

**機能:**

- JSONからYAMLへの変換
- クリーンで人間が読みやすい出力
- 適切なYAML構文
- 複雑なネスト構造のサポート

**使用例:**

```rust
use reinhardt_renderers::YAMLRenderer;
use serde_json::json;

let renderer = YAMLRenderer::new();
let data = json!({"key": "value", "nested": {"foo": "bar"}});
let result = renderer.render(&data, None).await?;
```

#### OpenAPIRenderer

JSONまたはYAML形式でOpenAPI 3.0仕様をレンダリングします。

**機能:**

- JSON形式出力（デフォルト）
- `.format("yaml")`によるYAML形式出力
- `.pretty(true)`による整形出力のサポート
- 完全なOpenAPI 3.0スキーマのサポート

**使用例:**

```rust
use reinhardt_renderers::OpenAPIRenderer;

// JSON形式（デフォルト）
let json_renderer = OpenAPIRenderer::new()
    .pretty(true);

// YAML形式
let yaml_renderer = OpenAPIRenderer::new()
    .format("yaml");

let openapi_spec = json!({"openapi": "3.0.0", ...});
let result = json_renderer.render(&openapi_spec, None).await?;
```

### コアトレイト

#### Rendererトレイト

非同期サポートを持つすべてのレンダラーのためのベーストレイト。

**メソッド:**

- `media_types()` - サポートするMIMEタイプを返す
- `render()` - データをバイト列に非同期レンダリング
- `format()` - オプションの形式識別子

#### RendererContext

レンダリング中にレンダラーに渡されるコンテキスト情報。

## 予定

### 追加レンダラー

- **TemplateRenderer** - テンプレートエンジン統合によるテンプレートベースのHTMLレンダリング

### コンテンツネゴシエーション

- Acceptヘッダーに基づく自動レンダラー選択
- 複数のレンダラー管理のためのレンダラーレジストリ
- 品質値（q-factor）のサポート
- 形式サフィックスの処理（例: `/api/users.json`）

### 高度な機能

- カスタムレンダラーミドルウェア
- レンダラーのチェーン
- レスポンスのキャッシュ
- 大きなレスポンスのためのストリーミングサポート
- 圧縮サポート（gzip、brotli）

## 依存関係

- `serde_json` - JSONシリアライゼーション
- `serde_yaml` - YAMLサポート
- `quick-xml` - XML生成
- `csv` - CSV出力サポート
- `utoipa` - OpenAPIスキーマサポート
- `bytes` - 効率的なバイトバッファの処理
- `async-trait` - 非同期トレイトのサポート

## 関連クレート

- `reinhardt-browsable-api` - ブラウザブルAPIインターフェースの実装
- `reinhardt-exception` - エラー処理
- `reinhardt-apps` - アプリケーションフレームワーク
