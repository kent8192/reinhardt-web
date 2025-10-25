# reinhardt-templates

Reinhardtフレームワーク用のAskamaを使用したテンプレートエンジン

## 概要

HTML レスポンスをレンダリングするためのテンプレートエンジン統合。Askama（Rust 用の Jinja ライクなテンプレートエンジン）を基盤とした柔軟なテンプレートシステムを提供し、カスタムフィルター、ファイルシステムローディング、静的ファイル処理、国際化サポートなど、Django にインスパイアされた機能を備えています。

## 機能

### 実装済み ✓

#### コアテンプレート機能

- **変数展開**: `{{ variable }}` 構文による動的コンテンツの挿入
- **制御構造**: 条件分岐とループのための `{% if %}`、`{% for %}` タグ
- **テンプレート継承**: テンプレート合成のための `{% extends %}` と `{% block %}`
- **基本フィルター**: データ変換のための組み込み Askama フィルター

#### テンプレート管理

- **TemplateLoader**: 実行時のテンプレート登録とレンダリング
  - `register()` メソッドによるテンプレート登録
  - 名前によるテンプレートの `render()`
  - `TemplateId` トレイトによる型安全なテンプレートローディング
  - スレッドセーフな並行テンプレートアクセス
- **FileSystemTemplateLoader**: ファイルシステムからのテンプレート読み込み
  - セキュリティ: ディレクトリトラバーサル防止
  - オプションのキャッシュ制御によるテンプレートキャッシング
  - ファイルパスでの Unicode と絵文字のサポート
  - 深くネストされたディレクトリのサポート
  - 並行アクセスのサポート

#### カスタムフィルター（Django 互換）

すべてのフィルターはエラーハンドリングのために `AskamaResult<T>` を返し、Askama テンプレートで使用できます:

**文字列変換**

- `upper` - 大文字に変換: `{{ "hello"|upper }}` → `HELLO`
- `lower` - 小文字に変換: `{{ "HELLO"|lower }}` → `hello`
- `capitalize` - 最初の文字を大文字に: `{{ "hello"|capitalize }}` → `Hello`
- `title` - タイトルケースに変換: `{{ "hello world"|title }}` → `Hello World`
- `trim` - 空白を削除: `{{ "  hello  "|trim }}` → `hello`
- `reverse` - 文字列を反転: `{{ "hello"|reverse }}` → `olleh`

**文字列操作**

- `truncate(length)` - 省略記号で切り詰め: `{{ "Hello World"|truncate(5) }}` → `Hello...`
- `ljust(width, fill)` - パディングで左寄せ: `{{ "42"|ljust(5, "0") }}` → `42000`
- `rjust(width, fill)` - パディングで右寄せ: `{{ "42"|rjust(5, "0") }}` → `00042`
- `replace(from, to)` - 部分文字列を置換: `{{ "hello world"|replace("world", "rust") }}` → `hello rust`

**文字列分析**

- `length` - 文字列の長さを取得: `{{ "hello"|length }}` → `5`
- `split(separator)` - 配列に分割: `{{ "a,b,c"|split(",") }}` → `["a", "b", "c"]`

**配列操作**

- `join(separator)` - 配列要素を結合: `{{ items|join(", ") }}` → `a, b, c`

**条件付きレンダリング**

- `default(value)` - 空文字列のデフォルト値を提供: `{{ ""|default("N/A") }}` → `N/A`

**HTML 処理**

- `striptags` - HTML タグを削除: `{{ "<p>Hello</p>"|striptags }}` → `Hello`

#### 静的ファイルサポート

- **static_filter**: 静的ファイルの URL を生成
  - 基本的な静的 URL 生成: `{{ "css/style.css"|static }}` → `/static/css/style.css`
  - ハッシュ化されたファイル名のマニフェストサポート（キャッシュバスティング）
  - 設定可能な静的 URL プレフィックス
  - パスの正規化（先頭のスラッシュを削除）
- **StaticConfig**: グローバル静的ファイル設定
  - カスタム静的 URL（デフォルト: `/static/`）
  - オプションのマニフェストベースのキャッシュバスティング
  - スレッドセーフな設定管理
- **static_path_join**: 動的パス構築のためのパスコンポーネント結合

#### 国際化（i18n）フィルター

プレースホルダー実装による基本的な i18n サポート:

- `get_current_language()` - 現在の言語コードを取得
- `trans(message)` - 文字列を翻訳
- `trans_with_context(context, message)` - コンテキスト付きで翻訳
- `blocktrans(message)` - ブロック翻訳
- `blocktrans_plural(singular, plural, count)` - 複数形対応の翻訳
- `localize_date_filter(date)` - 日付フォーマットのローカライズ
- `localize_number_filter(number)` - 数値フォーマットのローカライズ

### 予定

#### 高度なテンプレート機能

- **コンテキストプロセッサー**: すべてのテンプレートのグローバルコンテキスト変数
- **テンプレートタグ**: フィルター以外のカスタムテンプレートタグ
- **自動エスケープ**: セキュリティのための自動 HTML エスケープ
- **テンプレートインクルード**: テンプレート合成のための `{% include %}` タグ

#### 強化された i18n

- reinhardt-i18n クレートとの完全統合
- 実際の翻訳検索（現在は入力をそのまま返す）
- ロケール固有の日付と数値のフォーマット
- 複数言語の複数形処理
- 翻訳コンテキストサポート

#### パフォーマンス

- テンプレートコンパイルキャッシング
- プリコンパイル済みテンプレートバンドル
- 遅延テンプレートローディング
- メモリ効率の良いテンプレートストレージ

#### 開発者体験

- テンプレートデバッグツール
- 行番号付きの改善されたエラーメッセージ
- テンプレート構文検証
- 開発中のホットリロード

## 使用方法

### 基本的なテンプレート使用

```rust
use reinhardt_templates::Template;
use askama::Template as AskamaTemplate;

#[derive(Template)]
#[template(source = "Hello {{ name }}!", ext = "txt")]
struct HelloTemplate {
    name: String,
}

let tmpl = HelloTemplate { name: "World".to_string() };
assert_eq!(tmpl.render().unwrap(), "Hello World!");
```

### テンプレートローダー

```rust
use reinhardt_templates::TemplateLoader;

let mut loader = TemplateLoader::new();
loader.register("hello", || "Hello World!".to_string());

let result = loader.render("hello").unwrap();
assert_eq!(result, "Hello World!");
```

### 型安全なテンプレート

```rust
use reinhardt_templates::{TemplateLoader, TemplateId};

pub struct HomeTemplate;
impl TemplateId for HomeTemplate {
    const NAME: &'static str = "home.html";
}

let mut loader = TemplateLoader::new();
loader.register_typed::<HomeTemplate, _>(|| "<h1>Home Page</h1>".to_string());

let html = loader.render_typed::<HomeTemplate>().unwrap();
```

### ファイルシステムテンプレートローダー

```rust,no_run
use reinhardt_templates::FileSystemTemplateLoader;
use std::path::Path;

let loader = FileSystemTemplateLoader::new(Path::new("/app/templates"));
let content = loader.load("index.html").unwrap();
```

### 静的ファイル

```rust
use reinhardt_templates::static_filters::{StaticConfig, init_static_config};
use std::collections::HashMap;

// 静的ファイルを設定
init_static_config(StaticConfig {
    static_url: "/static/".to_string(),
    use_manifest: false,
    manifest: HashMap::new(),
});

// テンプレート内: {{ "css/style.css"|static }} → /static/css/style.css
```

## 依存関係

- **askama**: Rust 用の Jinja ライクなテンプレートエンジン
- **serde**: テンプレートコンテキストのシリアライゼーションサポート
- **thiserror**: エラーハンドリング
- **reinhardt-i18n**: 国際化サポート
- **reinhardt-exception**: 統一されたエラー型

## ライセンス

Apache License, Version 2.0 または MIT license のいずれかの下でライセンスされています。
