# reinhardt-openapi

OpenAPIスキーマ生成とSwagger UI統合

## 概要

APIエンドポイント、シリアライザー、ビューセットから自動的にOpenAPI 3.0スキーマを生成します。リクエスト/レスポンススキーマ、認証要件、パラメータ説明を含む完全なAPIドキュメントを生成します。`utoipa-swagger-ui`を使用した組み込みSwagger UI統合を含みます。

## 機能

### 実装済み ✓

#### OpenAPI 3.0コア型

- **完全なOpenAPI 3.0仕様**: `utoipa`の再エクスポートによるOpenAPI 3.0型の完全サポート
  - Info、Contact、Licenseメタデータ
  - Paths、PathItem、Operation定義
  - パラメータ定義（Query、Header、Path、Cookieの位置）
  - リクエスト/レスポンスボディスキーマ
  - コンポーネントと再利用可能なスキーマ
  - セキュリティスキーム（HTTP、ApiKey、OAuth2）
  - 変数を持つサーバー定義
  - API整理のためのタグ定義

#### スキーマ生成

- **SchemaGenerator**: OpenAPIスキーマを作成するためのビルダーパターン
  - タイトル、バージョン、説明を設定するための流暢なAPI
  - `OpenApiSchema`（utoipaの`OpenApi`型）への直接生成

#### ドキュメントUI

- **Swagger UI統合**: `utoipa-swagger-ui`と`askama`テンプレートによる組み込みSwagger UI
  - カスタマイズ可能なタイトルと仕様URLを持つHTMLレンダリング
  - Swagger UIページを提供するためのリクエストハンドラー
  - `/api-docs/openapi.json`での自動OpenAPI仕様提供
  - スキーマJSON出力機能
- **Redoc UIサポート**: 代替ドキュメントインターフェース
  - RedocのためのHTMLレンダリング
  - Redocページを提供するためのリクエストハンドラー
  - 同じOpenAPI仕様エンドポイントを使用

#### フォーマット出力

- **JSON出力**: OpenAPIスキーマをJSON形式にシリアライズ
- **YAML出力**: `serde_yaml`依存関係によるサポート（依存関係に機能が存在）

#### utoipa互換性レイヤー

- **双方向型変換**: Reinhardtとutoipa型間の完全な変換ユーティリティ
  - スキーマ型変換（Object、Array、プリミティブ）
  - パラメータおよびリクエスト/レスポンスボディ変換
  - セキュリティスキーム変換（HTTP、ApiKey、OAuth2）
  - サーバーおよびタグ変換
  - フォーマットおよびスキーマ型マッピング
  - 包括的なテストカバレッジ

#### 自動スキーマ導出

- **ToSchemaトレイト**: OpenAPIスキーマを生成できる型のコアトレイト
  - `schema()`メソッドはOpenAPIスキーマ表現を返す
  - `schema_name()`メソッドはオプションのスキーマ識別子を返す
  - すべてのRustプリミティブ型に実装済み（i8-i64、u8-u64、f32-f64、bool、String）
  - `Option<T>`と`Vec<T>`のジェネリック実装

- **Schemaマクロの導出**: 自動スキーマ生成のための`#[derive(Schema)]`手続き型マクロ
  - 自動フィールドメタデータ抽出（型、必須、nullable）
  - 名前付きフィールドを持つstruct型のサポート
  - 自動必須フィールド検出（`Option<T>`フィールドはオプション）
  - フィールド説明のためのドキュメントコメント抽出
  - 文字列バリアント生成を持つenum型のサポート
  - utoipa 5.4のObjectBuilderパターンと互換性あり

### 予定

#### 拡張自動スキーマ機能

- **属性マクロサポート**: 高度なスキーマカスタマイズ
  - フィールド設定: `#[schema(example = "...", description = "...")]`
  - `$ref`参照を持つネストされたスキーマ生成
  - 高度なenum処理（タグ付き、隣接タグ付き、タグなし）
  - serde属性との統合（`#[serde(rename)]`、`#[serde(skip)]`）
  - コンポーネント再利用のためのスキーマレジストリ
  - 検証制約の反映（min、max、pattern）
  - サンプル値の生成
- **HashMapサポート**: `HashMap<K,V>`スキーマ生成
- **タプル構造体サポート**: タプル構造体のスキーマ生成

#### ViewSet統合

- **ViewSetインスペクター**: ViewSetからの自動スキーマ抽出
  - ViewSetメソッドとシリアライザーの内省
  - ViewSet定義からのパスとオペレーションの生成
  - メソッドシグネチャからのパラメータ情報抽出
  - シリアライザーからの自動リクエスト/レスポンススキーマ生成

## 使い方

### Schemaマクロの導出

```rust
use reinhardt_macros::Schema;
use reinhardt_openapi::ToSchema;

#[derive(Schema)]
struct User {
    /// User's unique identifier
    id: i64,
    /// User's username
    name: String,
    /// Optional email address
    email: Option<String>,
}

#[derive(Schema)]
enum Status {
    Active,
    Inactive,
    Pending,
}

fn main() {
    // Generate schema for User struct
    let user_schema = User::schema();
    let user_name = User::schema_name(); // Some("User")

    // Generate schema for Status enum
    let status_schema = Status::schema();
    let status_name = Status::schema_name(); // Some("Status")
}
```

**主な機能:**

- ドキュメントコメント（`///`）は自動的にフィールド説明として抽出される
- `Option<T>`フィールドは自動的にオプション（必須でない）としてマークされる
- 非オプションフィールドは自動的に必須としてマークされる
- Enumバリアントは列挙値を持つ文字列スキーマに変換される
- utoipa 5.4以降と互換性あり

### 基本的なスキーマ生成

```rust
use reinhardt_openapi::{SchemaGenerator, OpenApiSchema};

// Generate schema from ViewSets
let generator = SchemaGenerator::new("My API", "1.0.0")
    .description("API documentation");

let schema = generator.generate()?;
let json = schema.to_json()?;
```

## Swagger UI統合

```rust
use reinhardt_openapi::{SchemaGenerator, SwaggerUI};
use reinhardt_apps::{Request, Response};

// Create schema
let schema = SchemaGenerator::new("My API", "1.0.0")
    .description("API documentation")
    .generate()?;

// Create Swagger UI handler
let swagger_ui = SwaggerUI::new(schema);

// Handle requests
async fn handle_swagger_request(request: Request) -> Result<Response> {
    swagger_ui.handle(request).await
}
```

## Redoc UI統合

```rust
use reinhardt_openapi::{SchemaGenerator, RedocUI};

// Create schema
let schema = SchemaGenerator::new("My API", "1.0.0")
    .description("API documentation")
    .generate()?;

// Create Redoc UI handler
let redoc_ui = RedocUI::new(schema);

// Generate HTML
let html = redoc_ui.render_html()?;
```

## APIエンドポイント

SwaggerUIを使用する場合、以下のエンドポイントが自動的に利用可能になります:

- `/swagger-ui/` - Swagger UI HTMLインターフェース
- `/swagger-ui/swagger-ui-init.js` - Swagger UI初期化スクリプト
- `/swagger-ui/swagger-ui.css` - Swagger UIスタイル
- `/swagger-ui/swagger-ui-bundle.js` - Swagger UI JavaScriptバンドル
- `/api-docs/openapi.json` - JSON形式のOpenAPI仕様

## 以前のバージョンからの移行

このバージョンはカスタムテンプレートの代わりに`utoipa-swagger-ui`を使用します。APIはほぼ互換性がありますが、いくつかの内部実装の詳細が変更されました:

- テンプレートは使用されなくなりました（askama依存関係が削除されました）
- Swagger UIアセットは`utoipa-swagger-ui`から直接提供されます
- OpenAPIスキーマは内部的に`utoipa`フォーマットに変換されます
- すべての既存のパブリックAPIは変更されていません