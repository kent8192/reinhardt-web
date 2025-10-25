# reinhardt-template

Reinhardtフレームワーク用のテンプレートシステム

## 概要

`reinhardt-template` は、Reinhardtアプリケーション向けの包括的なテンプレートシステムを提供します。テンプレートエンジン、レスポンスレンダラー、テンプレートマクロが含まれています。Askama (Jinjaライクなテンプレートエンジン) とDjangoにインスパイアされた機能を統合し、柔軟なHTMLレンダリングとコンテンツネゴシエーションを実現します。

このクレートは、複数のテンプレート関連サブクレートを統合し、統一されたテンプレート体験を提供する親クレートとして機能します。

## 機能

### 実装済み ✓

この親クレートは、以下のサブクレートから機能を再エクスポートします:

- **テンプレート** (`reinhardt-templates`): Askama統合を持つテンプレートエンジン
  - `{{ variable }}` 構文による変数置換
  - 制御構造: `{% if %}`、`{% for %}` タグ
  - テンプレート継承: `{% extends %}` と `{% block %}`
  - ランタイムテンプレート登録のためのTemplateLoader
  - ディスクから読み込むためのFileSystemTemplateLoader
  - セキュリティ: ディレクトリトラバーサル防止
  - キャッシュ制御を持つテンプレートキャッシング (デフォルトで有効、無効化可能)
  - カスタムフィルタ: upper, lower, trim, reverse, truncate, join, default, capitalize, title, length, ljust, rjust, replace, split, striptags
  - 静的ファイルのURLを生成するための静的ファイルフィルタ
  - 翻訳とローカライゼーションのためのi18nフィルタ

- **テンプレートマクロ** (`reinhardt-templates-macros`): テンプレート用の手続きマクロ
  - テンプレート構造体のための `#[derive(Template)]`
  - コンパイル時テンプレート検証
  - 型安全なテンプレートレンダリング

- **レンダラー** (`reinhardt-renderers`): 異なる形式のレスポンスレンダラー
  - JSON レスポンスのための JSONRenderer
  - HTML インターフェースのための BrowsableAPIRenderer
  - XML レスポンスのための XMLRenderer
  - YAML レスポンスのための YAMLRenderer
  - CSV 表形式データのための CSVRenderer
  - OpenAPI 3.0 仕様のための OpenAPIRenderer
  - Django ライクな管理インターフェースのための AdminRenderer
  - 静的 HTML コンテンツのための StaticHTMLRenderer
  - API ドキュメンテーションのための DocumentationRenderer
  - JavaScript スキーマのための SchemaJSRenderer
  - テンプレートベースの HTML レンダリングのための TemplateHTMLRenderer
  - Accept ヘッダーに基づくコンテンツネゴシエーション
  - カスタムレンダラーのサポート
  - 整形表示とフォーマットオプション

### 予定

- 現在のセット (upper, lower, trim, reverse, truncate, join, default, capitalize, title, length, ljust, rjust, replace, split, striptags) を超える追加のテンプレートフィルタ
- より良い診断を備えた強化されたエラーメッセージ
- テンプレートデバッグツールと開発モード

## インストール

`Cargo.toml` に以下を追加してください:

```toml
[dependencies]
reinhardt-template = "0.1.0"
```

### オプション機能

必要に応じて特定の機能を有効化してください:

```toml
[dependencies]
reinhardt-template = { version = "0.1.0", features = ["templates", "renderers"] }
```

利用可能な機能:

- `templates` (デフォルト): テンプレートエンジン機能
- `templates-macros` (デフォルト): テンプレートマクロ
- `renderers` (デフォルト): レスポンスレンダラー
- `full`: すべての機能を有効化

## 使用方法

### テンプレートレンダリング

```rust
use reinhardt_template::{Template, TemplateLoader};

// テンプレートを定義
#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    title: String,
    user: String,
}

// テンプレートをレンダリング
let tmpl = IndexTemplate {
    title: "Welcome".to_string(),
    user: "John".to_string(),
};

let html = tmpl.render()?;
```

### ファイルシステムテンプレート

```rust
use reinhardt_template::FileSystemTemplateLoader;
use std::path::PathBuf;

// ローダーを作成
let loader = FileSystemTemplateLoader::new(PathBuf::from("./templates"));

// テンプレートを読み込んでレンダリング
let html = loader.load_and_render("index.html", &context)?;
```

### JSONレンダリング

```rust
use reinhardt_template::{JSONRenderer, Renderer};
use serde_json::json;

let renderer = JSONRenderer::new()
    .pretty(true)
    .ensure_ascii(false);

let data = json!({
    "message": "Hello, world!",
    "status": "success"
});

let response = renderer.render(&data)?;
```

### コンテンツネゴシエーション

```rust
use reinhardt_template::{ContentNegotiation, JSONRenderer, BrowsableAPIRenderer};

let negotiation = ContentNegotiation::new()
    .add_renderer(Box::new(JSONRenderer::new()))
    .add_renderer(Box::new(BrowsableAPIRenderer::new()));

let renderer = negotiation.select_renderer(&accept_header)?;
let response = renderer.render(&data)?;
```

## サブクレート

この親クレートには以下のサブクレートが含まれています:

```
reinhardt-template/
├── Cargo.toml          # 親クレート定義
├── src/
│   └── lib.rs          # サブクレートからの再エクスポート
└── crates/
    ├── templates/      # テンプレートエンジン
    ├── templates-macros/ # テンプレートマクロ
    └── renderers/      # レスポンスレンダラー
```

## ライセンス

Apache License, Version 2.0 または MIT license のいずれかの条件の下でライセンスされています。