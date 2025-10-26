# パート1: プロジェクトのセットアップ

このチュートリアルでは、新しいReinhardtプロジェクトを作成し、最初のビューを作成します。

## インストールの確認

始める前に、RustとCargoが正しくインストールされていることを確認しましょう：

```bash
rustc --version
cargo --version
```

両方のコマンドでバージョン情報が表示されるはずです。表示されない場合は、[rust-lang.org](https://www.rust-lang.org/tools/install)にアクセスしてRustをインストールしてください。

## Reinhardt Adminのインストール

まず、プロジェクト生成用のグローバルツールをインストールします：

```bash
cargo install reinhardt-admin
```

## プロジェクトの作成

コードを保存したいディレクトリに移動し、次のコマンドを実行します：

```bash
reinhardt-admin startproject polls_project --template-type mtv
cd polls_project
```

これにより、以下の構造を持つ`polls_project`ディレクトリが作成されます：

```
polls_project/
├── Cargo.toml
├── README.md
└── src/
    ├── main.rs
    ├── config.rs
    ├── apps.rs
    ├── config/
    │   ├── settings.rs
    │   ├── settings/
    │   │   ├── base.rs
    │   │   ├── local.rs
    │   │   ├── staging.rs
    │   │   └── production.rs
    │   ├── urls.rs
    │   └── apps.rs
    └── bin/
        ├── runserver.rs
        └── manage.rs
```

**注意**: このチュートリアルでは、テンプレート、フォーム、管理画面を含む**MTV (Model-Template-View)**テンプレートを使用します。

## プロジェクト構造の理解

生成されたプロジェクトの主要な要素を理解しましょう：

- `Cargo.toml` - プロジェクトとその依存関係の設定ファイル
- `src/main.rs` - アプリケーションのエントリーポイント
- `src/config/` - プロジェクト設定
  - `settings/` - 環境別設定（base, local, staging, production）
  - `urls.rs` - URLルーティング設定
  - `apps.rs` - インストール済みアプリの登録
- `src/bin/` - 実行可能ファイル
  - `manage.rs` - 管理コマンド（Djangoの`manage.py`に相当）
  - `runserver.rs` - 開発サーバー

## 最初のビューの作成

Reinhardtのビューは、HTTPリクエストを受け取ってHTTPレスポンスを返す関数です。シンプルなビューを作成しましょう。

`src/main.rs`を編集します：

```rust
use reinhardt::prelude::*;

// 最初のビュー - シンプルなテキストレスポンスを返す
async fn index(_request: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let mut response = Response::new();
    response.set_status(StatusCode::OK);
    response.set_body("Hello, world. You're at the polls index.".to_string());
    response.set_header("Content-Type", "text/plain");
    Ok(response)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 次にルーティング設定を追加します
    println!("Server setup complete");
    Ok(())
}
```

これはReinhardtで最もシンプルなビューです。"Hello, world. You're at the polls index."というプレーンテキストレスポンスを返します。

## URLとビューのマッピング

このビューを呼び出すには、URLにマッピングする必要があります。新しいファイル`src/urls.rs`を作成します：

```rust
use reinhardt::prelude::*;

pub async fn index(_request: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let mut response = Response::new();
    response.set_status(StatusCode::OK);
    response.set_body("Hello, world. You're at the polls index.".to_string());
    response.set_header("Content-Type", "text/plain");
    Ok(response)
}

pub fn url_patterns() -> Vec<Route> {
    vec![
        path("", index),
    ]
}
```

次に`src/main.rs`を更新して、このURL設定を使用します：

```rust
mod urls;

use reinhardt::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ルーターを作成
    let mut router = DefaultRouter::new();

    // URLパターンを登録
    for route in urls::url_patterns() {
        router.add_route(route);
    }

    // サーバーを作成して設定
    let server = Server::new("127.0.0.1:8000", router);

    println!("Starting development server at http://127.0.0.1:8000/");
    println!("Quit the server with CTRL-C.");

    // サーバーを実行
    server.run().await?;

    Ok(())
}
```

## 開発サーバーの実行

それでは開発サーバーを実行しましょう：

```bash
# runserverバイナリを使用（推奨）
cargo run --bin runserver

# またはmanageコマンドを使用
cargo run --bin manage runserver
```

以下のような出力が表示されるはずです：

```
    Compiling polls_project v0.1.0 (/path/to/polls_project)
     Finished dev [unoptimized + debuginfo] target(s) in 2.34s
      Running `target/debug/runserver`

Reinhardt Development Server
──────────────────────────────────────────────────

  ✓ http://127.0.0.1:8000
  Environment: Debug

Quit the server with CTRL+C
```

Webブラウザを開いて`http://127.0.0.1:8000/`にアクセスします。ウェルカムメッセージが表示されるはずです。

おめでとうございます！Reinhardtプロジェクトが正常に起動しました。

## 何が起こったのか

今行ったことを振り返りましょう：

1. **ビュー関数を作成**（`index`）してHTTPレスポンスを返す
2. **URLパターンを作成**してルートURL（`""`）をビューにマッピング
3. **ルーターを設定**して受信リクエストを処理
4. **開発サーバーを起動**してポート8000でリッスン

これがReinhardtの基本的なリクエスト-レスポンスサイクルです：

```
ブラウザリクエスト → サーバー → ルーター → URLパターン → ビュー → レスポンス → ブラウザ
```

## path関数の説明

`path()`関数は2つの引数を取ります：

```rust
path("", index)
```

- 最初の引数はURLパターン（`""`はルートURLを意味します）
- 2番目の引数は呼び出すビュー関数

より複雑なパターンも作成できます：

```rust
path("polls/", polls_index)
path("polls/{id}/", poll_detail)
```

`{id}`構文はビューに渡されるURLパラメータを作成します。

## Pollsアプリの作成

Reinhardtでは、機能ごとにアプリを作成して整理します（Djangoと同様）。`polls`アプリを作成しましょう：

```bash
cargo run --bin manage startapp polls --template-type mtv
```

これにより、以下の構造を持つ`polls`ディレクトリが作成されます：

```
polls/
├── lib.rs
├── models.rs
├── models/
├── views.rs
├── views/
├── admin.rs
├── urls.rs
└── tests.rs
```

### ビューの作成

`polls/views.rs`を編集します：

```rust
use reinhardt::prelude::*;

pub async fn index(_request: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let mut response = Response::new();
    response.set_status(StatusCode::OK);
    response.set_body("Hello, world. You're at the polls index.".to_string());
    response.set_header("Content-Type", "text/plain");
    Ok(response)
}
```

### URLパターンの設定

`polls/urls.rs`を編集します：

```rust
use reinhardt_routers::UnifiedRouter;
use crate::views;

pub fn url_patterns() -> UnifiedRouter {
    let router = UnifiedRouter::builder()
        .build();

    router.add_function_route("/", Method::GET, views::index);

    router
}
```

### プロジェクトURLへの登録

`src/config/urls.rs`を編集して、pollsアプリのURLを含めます：

```rust
use reinhardt::prelude::*;
use std::sync::Arc;

pub fn url_patterns() -> Arc<UnifiedRouter> {
    let router = UnifiedRouter::builder()
        .build();

    // pollsアプリのルーターを含める
    router.include_router("/polls/", polls::urls::url_patterns(), Some("polls".to_string()));

    Arc::new(router)
}
```

### アプリの登録

`src/config/apps.rs`を編集します：

```rust
use reinhardt_macros::installed_apps;

installed_apps! {
    polls: "polls",
}

pub fn get_installed_apps() -> Vec<String> {
    InstalledApp::all_apps()
}
```

サーバーを再起動（Ctrl-Cを押して`cargo run --bin runserver`を再度実行）して、`http://127.0.0.1:8000/polls/`にアクセスします。メッセージが表示されるはずです。

## さらにビューを追加

URL設定をより興味深くするために、いくつかのビューを追加しましょう。`src/polls.rs`を更新します：

```rust
use reinhardt::prelude::*;

pub async fn index(_request: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let mut response = Response::new();
    response.set_status(StatusCode::OK);
    response.set_body("Hello, world. You're at the polls index.".to_string());
    response.set_header("Content-Type", "text/plain");
    Ok(response)
}

pub async fn detail(request: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    // 後のチュートリアルでURLからquestion_idを抽出します
    let question_id = request.path_params.get("question_id").unwrap_or(&"0".to_string());

    let mut response = Response::new();
    response.set_status(StatusCode::OK);
    response.set_body(format!("You're looking at question {}.", question_id));
    response.set_header("Content-Type", "text/plain");
    Ok(response)
}

pub async fn results(request: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let question_id = request.path_params.get("question_id").unwrap_or(&"0".to_string());

    let mut response = Response::new();
    response.set_status(StatusCode::OK);
    response.set_body(format!("You're looking at the results of question {}.", question_id));
    response.set_header("Content-Type", "text/plain");
    Ok(response)
}

pub async fn vote(request: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let question_id = request.path_params.get("question_id").unwrap_or(&"0".to_string());

    let mut response = Response::new();
    response.set_status(StatusCode::OK);
    response.set_body(format!("You're voting on question {}.", question_id));
    response.set_header("Content-Type", "text/plain");
    Ok(response)
}
```

`src/urls.rs`を更新して、これらのビューをURL設定に接続します：

```rust
use reinhardt::prelude::*;

pub fn url_patterns() -> Vec<Route> {
    vec![
        // 例: /polls/
        path("polls/", crate::polls::index),
        // 例: /polls/5/
        path("polls/{question_id}/", crate::polls::detail),
        // 例: /polls/5/results/
        path("polls/{question_id}/results/", crate::polls::results),
        // 例: /polls/5/vote/
        path("polls/{question_id}/vote/", crate::polls::vote),
    ]
}
```

サーバーを再起動して、以下のURLを試してみてください：

- `http://127.0.0.1:8000/polls/` - インデックスを表示
- `http://127.0.0.1:8000/polls/34/` - 質問34の詳細を表示
- `http://127.0.0.1:8000/polls/34/results/` - 質問34の結果を表示
- `http://127.0.0.1:8000/polls/34/vote/` - 質問34の投票フォームを表示

## 次は何をする？

基本的なReinhardtプロジェクトとURLルーティング、シンプルなビューを作成しました。次のチュートリアルでは、データベースをセットアップし、投票の質問と選択肢を保存するモデルを作成します。

準備ができたら、[パート2: モデルとデータベース](2-models-and-database.md)に進んでください。

## まとめ

このチュートリアルで学んだこと：

- 新しいReinhardtプロジェクトの作成方法
- 非同期関数としてビューを定義する方法
- `path()`を使用してURLをビューにマッピングする方法
- 開発サーバーの実行方法
- コードをモジュールに整理する方法
- URLからパラメータを抽出する方法

Reinhardtアプリケーション構築の堅固な基盤ができました！