# reinhardt-params

FastAPI-inspired parameter extraction system for Reinhardt.

## Features

### Implemented ✓

#### Core Extraction System

- **`FromRequest` trait**: 非同期パラメータ抽出のコア抽象化
- **`ParamContext`**: パスパラメータとヘッダー/クッキー名の管理
- **型安全なパラメータ抽出**: コンパイル時の型チェック付きリクエストからの抽出
- **エラーハンドリング**: `ParamError`を用いた詳細なエラーメッセージ

#### Path Parameters (`path.rs`)

- **`Path<T>`**: URLパスから単一の値を抽出
  - 全てのプリミティブ型のサポート: `i8`, `i16`, `i32`, `i64`, `i128`, `u8`, `u16`, `u32`, `u64`, `u128`, `f32`, `f64`, `bool`, `String`
  - `Deref`による透過的なアクセス: `*path` または `path.0`
  - `into_inner()`メソッドによる値の取り出し
- **`PathStruct<T>`**: 複数のパスパラメータを構造体に抽出
  - `DeserializeOwned`を実装した任意の構造体をサポート
  - URL-encodedフォーマットを用いた自動型変換 (`"42"` → `42`)

#### Query Parameters (`query.rs`)

- **`Query<T>`**: URLクエリ文字列からパラメータを抽出
  - `serde`を用いた柔軟な逆シリアル化
  - オプショナルフィールド (`Option<T>`) のサポート
- **マルチ値クエリパラメータ** (`multi-value-arrays` feature):
  - `?q=5&q=6` → `Vec<i32>`
  - 自動型変換: 文字列 → 数値、真偽値など
  - JSON値ベースのデシリアライゼーション

#### Headers (`header.rs`, `header_named.rs`)

- **`Header<T>`**: リクエストヘッダーから値を抽出
  - `String`と`Option<String>`のサポート
  - `ParamContext`を用いた実行時ヘッダー名指定
- **`HeaderStruct<T>`**: 複数のヘッダーを構造体に抽出
  - ヘッダー名の小文字正規化
  - URL-encodedを用いた自動型変換
- **`HeaderNamed<N, T>`**: コンパイル時のヘッダー名指定
  - マーカー型による型安全なヘッダー名: `Authorization`, `ContentType`
  - `String`と`Option<String>`のサポート
  - `HeaderName` trait によるカスタムヘッダー名の定義

#### Cookies (`cookie.rs`, `cookie_named.rs`)

- **`Cookie<T>`**: クッキーから値を抽出
  - `String`と`Option<String>`のサポート
  - `ParamContext`を用いた実行時クッキー名指定
- **`CookieStruct<T>`**: 複数のクッキーを構造体に抽出
  - RFC 6265準拠のクッキーパース
  - URL-decodingサポート
- **`CookieNamed<N, T>`**: コンパイル時のクッキー名指定
  - マーカー型による型安全なクッキー名: `SessionId`, `CsrfToken`
  - `String`と`Option<String>`のサポート
  - `CookieName` trait によるカスタムクッキー名の定義

#### Body Extraction (`body.rs`, `json.rs`, `form.rs`)

- **`Body`**: 生のリクエストボディをバイト列として抽出
- **`Json<T>`**: JSONボディのデシリアライゼーション
  - `serde_json`を用いた型安全なデシリアライゼーション
  - `Deref`と`into_inner()`によるアクセス
- **`Form<T>`**: application/x-www-form-urlencodedフォームデータの抽出
  - Content-Typeの検証
  - `serde_urlencoded`を用いたデシリアライゼーション

#### Multipart Support (`multipart.rs`, requires `multipart` feature)

- **`Multipart`**: multipart/form-dataのサポート
  - `multer`クレートを用いたストリーミング解析
  - ファイルアップロード対応
  - `next_field()`による反復処理

#### Validation Support (`validation.rs`, requires `validation` feature)

- **`Validated<T, V>`**: 検証済みパラメータラッパー
- **`WithValidation` trait**: 検証制約の流暢なAPI
  - **長さ制約**: `min_length()`, `max_length()`
  - **数値範囲**: `min_value()`, `max_value()`
  - **パターンマッチング**: `regex()`
  - **フォーマット検証**: `email()`, `url()`
- **`ValidationConstraints<T>`**: チェーン可能な検証ビルダー
  - `validate_string()`: 文字列値の検証
  - `validate_number()`: 数値の検証
  - 複数制約の組み合わせサポート
- **型エイリアス**: `ValidatedPath<T>`, `ValidatedQuery<T>`, `ValidatedForm<T>`
- **`reinhardt-validators`との統合**

### Planned

現在、計画されている未実装機能はありません。全ての主要機能が実装済みです。

## Quick Start

```rustuse reinhardt_params::{Path, Query, Json};
use serde::Deserialize;

#[derive(Deserialize)]
struct UserQuery {
    page: Option<i32>,
    per_page: Option<i32>,
}

#[endpoint(GET "/users/{id}")]
async fn get_user(
    id: Path<i64>,
    query: Query<UserQuery>,
    body: Json<UpdateUser>,
) -> Result<User> {
    // id.0 is the extracted i64
    // query.page is Option<i32>
    // body.0 is the deserialized UpdateUser
    Ok(User { id: id.0, ..body.0 })
}
```

## Parameter Types

## Path Parameters

```rust
// Single value
#[endpoint(GET "/users/{id}")]
async fn get_user(id: Path<i64>) -> String {
    format!("User ID: {}", id.0)
}

// Multiple values with struct
#[derive(Deserialize)]
struct UserPath {
    org: String,
    user_id: i64,
}

#[endpoint(GET "/orgs/{org}/users/{user_id}")]
async fn get_org_user(path: PathStruct<UserPath>) -> String {
    format!("Org: {}, User: {}", path.org, path.user_id)
}
```

## Query Parameters

```rust
#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    page: Option<i32>,
    tags: Vec<String>,  // ?tags=rust&tags=web → vec!["rust", "web"]
}
```

## Headers & Cookies

```rust
#[derive(Deserialize)]
struct CustomHeaders {
    #[serde(rename = "x-request-id")]
    x_request_id: String,

    #[serde(rename = "x-count")]
    count: i64,  // Auto type conversion: "123" → 123
}

#[endpoint(GET "/info")]
async fn info(headers: HeaderStruct<CustomHeaders>) -> String {
    format!("Request: {}", headers.x_request_id)
}
```

## Form & File Upload

```rust
// Form data
#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

#[endpoint(POST "/login")]
async fn login(form: Form<LoginForm>) -> String { /* ... */ }

// File upload (requires "multipart" feature)
#[endpoint(POST "/upload")]
async fn upload(mut multipart: Multipart) -> Result<()> {
    while let Some(field) = multipart.next_field().await? {
        let data = field.bytes().await?;
        // Process file...
    }
    Ok(())
}
```

## Feature Flags

```toml
[dependencies]reinhardt-params = { version = "0.1", features = ["multipart", "validation"] }
```

- `multi-value-arrays` (default): Multi-value query parameters
- `multipart`: File upload support via multer
- `validation`: Integration with reinhardt-validators

## Testing Status

✅ **183 tests passing**

- Path parameters: 41 tests
- Query parameters: 51 tests (including multi-value)
- Headers: 29 tests (with type conversion)
- Cookies: 29 tests
- Form data: 29 tests
- JSON body: 26 tests
- Unit tests: 7 tests

Integration tests in `tests/`:

- OpenAPI schema generation (3 tests)
- Validation constraints (10 tests)

## Documentation

See [crate documentation](https://docs.rs/reinhardt-params) for detailed API reference and examples.

## License

Dual-licensed under MIT and Apache-2.0.
