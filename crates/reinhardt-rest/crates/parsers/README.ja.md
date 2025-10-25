# reinhardt-parsers

Django REST Frameworkのパーサーにインスパイアされた、Reinhardtフレームワーク用のリクエストボディパーサー。

## 概要

Webアプリケーションで異なるリクエストコンテンツタイプを処理するための包括的なパーサーセットを提供します。各パーサーは`Parser`トレイトを実装し、`ParserRegistry`を通じた自動的なContent-Typeネゴシエーションで特定のMIMEタイプを処理できます。

## 実装済み ✓

### コアパーサーシステム

#### `Parser`トレイト

- **非同期トレイト** - Content-Typeネゴシエーション機能を持つリクエストボディのパース
- `media_types()` - サポートするMIMEタイプのリストを返す
- `parse()` - リクエストボディを`ParsedData`に非同期パース
- `can_parse()` - ワイルドカードサポートによる自動Content-Typeマッチング

#### `ParserRegistry`

- **中央レジストリ** - 複数のパーサーを管理
- リクエストのContent-Typeヘッダーに基づく自動パーサー選択
- パーサー登録のためのビルダーパターン
- カスタムパーサー実装のサポート

#### `MediaType`

- **Content-Typeのパースと操作**
- MIMEタイプパラメータのサポート（例: `charset=utf-8`）
- ワイルドカードマッチング（`application/*`、`*/json`、`*/*`）
- Content-TypeヘッダーからのRFC準拠のパース

#### `ParsedData`列挙型

パースされたリクエストデータの統一表現:

- `Json(Value)` - `serde_json`でパースされたJSONデータ
- `Form(HashMap<String, String>)` - URLエンコードされたフォームデータ
- `MultiPart { fields, files }` - ファイルアップロードを含むマルチパートフォームデータ
- `File(UploadedFile)` - 生のファイルアップロード

#### `UploadedFile`

ファイルアップロードの表現:

- フィールド名とオプションのファイル名
- Content-Type検出
- ファイルサイズのトラッキング
- `Bytes`を使用したバイナリデータの保存

### JSONパーサー（`JSONParser`）

#### 基本機能

- **Content-Type**: `application/json`、`application/*+json`
- `serde_json`を使用したJSONリクエストボディのパース
- 柔軟なデータ処理のための`ParsedData::Json(Value)`を返す

#### 高度なオプション

- **空ボディの処理** - `allow_empty()`で設定可能
  - デフォルト: 空ボディを拒否
  - オプション: 空リクエストに対して`null`を返す
- **厳格モード** - `strict()`で設定可能
  - デフォルト: 有効（Django REST Frameworkの動作）
  - 非有限浮動小数点（`Infinity`、`-Infinity`、`NaN`）を拒否
  - 寛容なパースのために無効化可能

#### バリデーション

- ネストされた構造（オブジェクトと配列）の再帰的バリデーション
- 不正なJSON向けの詳細なエラーメッセージ

### フォームパーサー（`FormParser`）

#### 基本機能

- **Content-Type**: `application/x-www-form-urlencoded`
- `serde_urlencoded`を使用したHTMLフォームデータのパース
- `ParsedData::Form(HashMap<String, String>)`を返す

#### URLエンコーディングのサポート

- フォーム値の自動パーセントデコーディング
- 特殊文字とスペースを正しく処理
- 空ボディは空のHashMapを返す（エラーではない）

### マルチパートパーサー（`MultiPartParser`）

#### 基本機能

- **Content-Type**: `multipart/form-data`
- 単一のリクエストでファイルアップロードとフォームフィールドを処理
- `ParsedData::MultiPart { fields, files }`を返す
- 堅牢なパースのために`multer`クレートをベースに構築

#### ファイルアップロード機能

- **複数ファイルのアップロード** - 単一のリクエストで
- フォームフィールドとファイルフィールドの分離処理
- ファイルごとのContent-Type検出
- 元のファイル名の保持

#### サイズ制限

- **ファイルごとのサイズ制限** - `max_file_size()`
- **総アップロードサイズ制限** - `max_total_size()`
- 制限超過時の詳細なエラーメッセージ

#### バウンダリーのパース

- Content-Typeヘッダーからの自動抽出
- RFC準拠のマルチパートバウンダリー処理

### ファイルアップロードパーサー（`FileUploadParser`）

#### 基本機能

- **Content-Type**: `application/octet-stream`、`*/*`
- マルチパートのオーバーヘッドなしの生ファイルアップロード
- `ParsedData::File(UploadedFile)`を返す
- 設定可能なフィールド名

#### ファイル名の抽出

- **標準ファイル名** - `Content-Disposition`ヘッダーからのパース
- **RFC2231エンコードされたファイル名** - 国際文字のサポート
  - フォーマット: `filename*=utf-8''%encoded_name`
  - 言語タグのサポート
  - 標準ファイル名より優先
- エンコードされたファイル名の自動URLデコーディング

#### サイズコントロール

- **最大ファイルサイズ** - `max_file_size()`で設定可能
- 制限超過時の詳細なエラーレポート

### エラー処理

- `reinhardt_exception::Error`を使用した統一エラー型
- 型エイリアス: `ParseError`と`ParseResult<T>`
- デバッグ用の詳細なエラーメッセージ
- フレームワーク例外システムとの統合

## 予定

### 追加パーサー

- **XMLパーサー** - `application/xml`と`text/xml`用
- **YAMLパーサー** - `application/x-yaml`用
- **MessagePackパーサー** - バイナリメッセージフォーマット用
- **Protobufパーサー** - Protocol Buffers用

### 拡張機能

- **ストリーミングパース** - メモリにボディ全体をロードせずに大きなファイルアップロードに対応
- **コンテンツネゴシエーション** - Acceptヘッダーに基づく自動パーサー選択
- **カスタムバリデーター** - パーサーごとのバリデーションフック
- **スキーマバリデーション** - JSON Schema、XML Schemaのサポート
- **圧縮サポート** - Gzip、Brotli、Deflateの展開

### パフォーマンス最適化

- **ゼロコピーパース** - 現在のパーサー実装で可能な場合
- **並列マルチパート処理** - 複数のファイルを同時にパース
- **メモリプーリング** - 繰り返しのパース操作のためのバッファ再利用

## 使用例

```rust
use bytes::Bytes;
use reinhardt_parsers::{
    JSONParser, FormParser, MultiPartParser, FileUploadParser,
    ParserRegistry,
};

// すべてのパーサーを含むレジストリを作成
let registry = ParserRegistry::new()
    .register(JSONParser::new())
    .register(FormParser::new())
    .register(MultiPartParser::new().max_file_size(10 * 1024 * 1024))
    .register(FileUploadParser::new("upload"));

// JSONリクエストをパース
let json_body = Bytes::from(r#"{"name": "test"}"#);
let parsed = registry
    .parse(Some("application/json"), json_body)
    .await?;

// フォームリクエストをパース
let form_body = Bytes::from("name=test&value=123");
let parsed = registry
    .parse(Some("application/x-www-form-urlencoded"), form_body)
    .await?;
```

## ライセンス

以下のいずれかのライセンスの下でライセンスされています:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE))
- MIT license ([LICENSE-MIT](../../LICENSE-MIT))

お好きな方を選択してください。
