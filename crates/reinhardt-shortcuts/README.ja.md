# reinhardt-shortcuts

Reinhardtフレームワーク向けのDjangoスタイルのショートカット関数

## 概要

Djangoの`django.shortcuts`モジュールに着想を得た、一般的なHTTP操作のための便利なショートカット関数です。これらの関数は、レスポンスの作成、リダイレクト、および自動404エラー処理を伴うデータベースクエリの処理のためのシンプルで直感的なAPIを提供します。

## 実装済み ✓

### リダイレクトショートカット

#### `redirect()`

一時的なリダイレクト(HTTP 302)レスポンスを作成します。

**使用例:**

```rust
use reinhardt_shortcuts::redirect;

let response = redirect("/new-location");
// HTTP 302とLocation: /new-locationを返します
```

#### `redirect_permanent()`

恒久的なリダイレクト(HTTP 301)レスポンスを作成します。

**使用例:**

```rust
use reinhardt_shortcuts::redirect_permanent;

let response = redirect_permanent("/permanent-location");
// HTTP 301とLocation: /permanent-locationを返します
```

### レスポンスレンダリングショートカット

#### `render_json()`

データをJSONとしてレンダリングし、HTTP 200レスポンスを返します。

**使用例:**

```rust
use reinhardt_shortcuts::render_json;
use serde_json::json;

let data = json!({
    "status": "success",
    "message": "Hello, world!"
});

let response = render_json(&data);
// Content-Type: application/jsonでHTTP 200を返します
```

#### `render_json_pretty()`

データをインデント付きの整形されたJSONとしてレンダリングします。

**使用例:**

```rust
use reinhardt_shortcuts::render_json_pretty;
use serde_json::json;

let data = json!({"key": "value"});
let response = render_json_pretty(&data);
// 改行とインデントを含む整形されたJSONを返します
```

#### `render_html()`

HTMLコンテンツをレンダリングし、HTTP 200レスポンスを返します。

**使用例:**

```rust
use reinhardt_shortcuts::render_html;

let html = "<h1>Hello, World!</h1>";
let response = render_html(html);
// Content-Type: text/html; charset=utf-8でHTTP 200を返します
```

#### `render_text()`

プレーンテキストコンテンツをレンダリングし、HTTP 200レスポンスを返します。

**使用例:**

```rust
use reinhardt_shortcuts::render_text;

let text = "Plain text content";
let response = render_text(text);
// Content-Type: text/plain; charset=utf-8でHTTP 200を返します
```

### データベースショートカット(404エラー処理)

#### `get_or_404_response()`

単一のオブジェクトを取得するか、見つからない場合は404レスポンスを返します。

**使用例:**

```rust
use reinhardt_shortcuts::get_or_404_response;

// データベースクエリ結果をシミュレート
let result = Ok(Some(user));

match get_or_404_response(result) {
    Ok(user) => {
        // ユーザーが見つかった場合、処理を続行
    }
    Err(response) => {
        // HTTP 404レスポンスを返す
        return response;
    }
}
```

#### `get_list_or_404_response()`

オブジェクトのリストを取得するか、リストが空の場合は404レスポンスを返します。

**使用例:**

```rust
use reinhardt_shortcuts::get_list_or_404_response;

let result = Ok(vec![user1, user2]);

match get_list_or_404_response(result) {
    Ok(users) => {
        // ユーザーが見つかった場合、処理を続行
    }
    Err(response) => {
        // リストが空の場合、HTTP 404を返す
        return response;
    }
}
```

#### `exists_or_404_response()`

レコードが存在するかチェックするか、404レスポンスを返します。

**使用例:**

```rust
use reinhardt_shortcuts::exists_or_404_response;

let result = Ok(true);

match exists_or_404_response(result) {
    Ok(_) => {
        // レコードが存在する場合、続行
    }
    Err(response) => {
        // 見つからない場合、HTTP 404を返す
        return response;
    }
}
```

### エラータイプ

#### `GetError`

データベースクエリ操作のエラータイプ

**バリアント:**

- `NotFound` - データベースでオブジェクトが見つからない
- `MultipleObjectsReturned` - 1つのオブジェクトが期待されるクエリで複数のオブジェクトが返された
- `DatabaseError(String)` - データベース操作エラー

## 機能ゲート付きで実装済み ✓

### ORM統合(`database`機能が必要)

#### `get_object_or_404<M>(pk: M::PrimaryKey) -> Result<M, Response>`

単一オブジェクト取得のための直接的なデータベース統合。ORMを使用してデータベースをクエリし、
オブジェクトが見つからない場合はHTTP 404を返します。

**使用例:**

```rust
use reinhardt_shortcuts::get_object_or_404;

async fn user_detail(user_id: i64) -> Result<Response, Response> {
    let user = get_object_or_404::<User>(user_id).await?;
    render_json(&user)
}
```

#### `get_list_or_404<M>(queryset: QuerySet<M>) -> Result<Vec<M>, Response>`

リスト取得のための直接的なデータベース統合。QuerySetを実行し、
結果リストが空の場合はHTTP 404を返します。

**使用例:**

```rust
use reinhardt_shortcuts::get_list_or_404;

async fn user_list(status: &str) -> Result<Response, Response> {
    let queryset = User::objects()
        .filter("status", FilterOperator::Eq, FilterValue::String(status.to_string()));

    let users = get_list_or_404(queryset).await?;
    render_json(&users)
}
```

### テンプレート統合(`templates`機能が必要)

テンプレートは`FileSystemTemplateLoader`を使用してファイルシステムから読み込まれます。テンプレート
ディレクトリは`REINHARDT_TEMPLATE_DIR`環境変数で指定されます(デフォルトは`./templates`)。

**現在の実装**: `{{ variable }}`構文を使用した基本的な変数置換がサポートされています。
テンプレートはシンプルなプレースホルダー置換で実行時に動的にレンダリングされます。

**計画されている拡張機能**:

- 完全なAsamaテンプレート構文: 制御構造(`{% if %}`, `{% for %}`)
- テンプレート継承(`{% extends %}`, `{% block %}`)
- カスタムフィルタとタグ

#### `render_template(request: &Request, template_name: &str, context: HashMap) -> Result<Response, Response>`

コンテキストを使用したテンプレートレンダリング。ファイルシステムからテンプレートファイルを読み込み、
テンプレートコンテンツを含むHTTPレスポンスを返します。コンテキスト変数は
デバッグモードでHTMLコメントとして表示されます。

**使用例:**

```rust
use reinhardt_shortcuts::render_template;
use std::collections::HashMap;

async fn index_view(request: Request) -> Result<Response, Response> {
    let mut context = HashMap::new();
    context.insert("title", "Welcome");
    context.insert("user", request.user().name());

    render_template(&request, "index.html", context)
}
```

#### `render_to_response(request: &Request, template_name: &str, context: HashMap) -> Result<Response, Response>`

カスタムレスポンス設定を伴う高度なテンプレートレンダリング。`render_template`と
同様ですが、さらにカスタマイズ可能な可変Responseを返します。

**使用例:**

```rust
use reinhardt_shortcuts::render_to_response;
use std::collections::HashMap;

async fn custom_view(request: Request) -> Result<Response, Response> {
    let mut context = HashMap::new();
    context.insert("message", "Custom response");

    let mut response = render_to_response(&request, "custom.html", context)?;

    // レスポンスをカスタマイズ
    response.status = hyper::StatusCode::CREATED;
    response.headers.insert(
        hyper::header::CACHE_CONTROL,
        hyper::header::HeaderValue::from_static("no-cache"),
    );

    Ok(response)
}
```

## 予定されている機能

### エラー処理(未実装)

- カスタムエラーページ(404、500など)
- エラーページテンプレート
- 開発用デバッグエラーページ

### テンプレートエンジン統合

- 変数置換のための完全なAsamaテンプレートエンジン統合
- テンプレート継承とインクルード
- カスタムテンプレートフィルタとタグ

## 使用パターン

### ショートカットの組み合わせ

```rust
use reinhardt_shortcuts::{render_json, get_or_404_response};

async fn get_user_handler(user_id: i64) -> Response {
    let result = database::find_user(user_id).await;

    match get_or_404_response(result) {
        Ok(user) => render_json(&user),
        Err(not_found_response) => not_found_response,
    }
}
```

### アクション後のリダイレクト

```rust
use reinhardt_shortcuts::{redirect, redirect_permanent};

async fn create_user_handler(user_data: UserData) -> Response {
    database::create_user(user_data).await;
    redirect("/users/")
}

async fn old_url_handler() -> Response {
    redirect_permanent("/new-url/")
}
```

## 依存関係

- `reinhardt-http` - HTTPタイプ(Request、Response)
- `serde` - シリアライゼーションサポート
- `serde_json` - JSONレンダリング
- `bytes` - 効率的なバイトバッファ処理
- `thiserror` - エラータイプ定義

## 関連クレート

- `reinhardt-http` - HTTPプリミティブ
- `reinhardt-views` - ビューレイヤー
- `reinhardt-orm` - データベースORM(将来の直接統合用)

## ライセンス

Apache License, Version 2.0またはMITライセンスのいずれかの条件の下でライセンスされています。
