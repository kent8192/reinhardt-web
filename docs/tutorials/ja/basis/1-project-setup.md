# パート1: プロジェクトのセットアップ

このチュートリアルでは、新しいReinhardtプロジェクトを作成し、最初のビューを作成します。

## インストールの確認

始める前に、RustとCargoが正しくインストールされていることを確認しましょう：

```bash
rustc --version
cargo --version
```

両方のコマンドでバージョン情報が表示されるはずです。表示されない場合は、[rust-lang.org](https://www.rust-lang.org/tools/install)にアクセスしてRustをインストールしてください。

## プロジェクトの作成

コードを保存したいディレクトリに移動し、次のコマンドを実行します：

```bash
cargo new polls_project
cd polls_project
```

これにより、以下の構造を持つ`polls_project`ディレクトリが作成されます：

```
polls_project/
├── Cargo.toml
└── src/
    └── main.rs
```

## Reinhardt依存関係の追加

`Cargo.toml`を開き、Reinhardtの依存関係を追加します。このチュートリアルでは、テンプレート、フォーム、管理画面を含む**standard**フレーバーを使用します：

```toml
[package]
name = "polls_project"
version = "0.1.0"
edition = "2021"

[dependencies]
reinhardt = { version = "0.1.0", features = ["standard"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
```

**注意**: Reinhardtでは、`reinhardt`クレート一つで全ての機能にアクセスできます。必要な機能は`features`フラグで制御します。

## プロジェクト構造の理解

現在のプロジェクト構成を理解しましょう：

- `Cargo.toml` - プロジェクトとその依存関係の設定ファイル
- `src/main.rs` - アプリケーションのエントリーポイント

Djangoとは異なり、Reinhardtには`manage.py`ファイルはありません。代わりに、Cargoコマンドを使用してプロジェクトを実行します。

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
cargo run
```

以下のような出力が表示されるはずです：

```
    Compiling polls_project v0.1.0 (/path/to/polls_project)
     Finished dev [unoptimized + debuginfo] target(s) in 2.34s
      Running `target/debug/polls_project`
Starting development server at http://127.0.0.1:8000/
Quit the server with CTRL-C.
```

Webブラウザを開いて`http://127.0.0.1:8000/`にアクセスします。以下のテキストが表示されるはずです：

```
Hello, world. You're at the polls index.
```

おめでとうございます！最初のReinhardtビューを作成しました。

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

## Pollsアプリモジュールの作成

Reinhardtでは、コードをモジュールに整理することがベストプラクティスです（Djangoのアプリに似ています）。`polls`モジュールを作成しましょう：

新しいファイル`src/polls.rs`を作成します：

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

`src/urls.rs`を更新してこのモジュールを使用します：

```rust
use reinhardt::prelude::*;

pub fn url_patterns() -> Vec<Route> {
    vec![
        path("polls/", crate::polls::index),
    ]
}
```

`src/main.rs`を更新してモジュールを宣言します：

```rust
mod urls;
mod polls;

use reinhardt::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut router = DefaultRouter::new();

    for route in urls::url_patterns() {
        router.add_route(route);
    }

    let server = Server::new("127.0.0.1:8000", router);

    println!("Starting development server at http://127.0.0.1:8000/");
    println!("Quit the server with CTRL-C.");

    server.run().await?;

    Ok(())
}
```

サーバーを再起動（Ctrl-Cを押して`cargo run`を再度実行）して、`http://127.0.0.1:8000/polls/`にアクセスします。同じメッセージが表示されるはずです。

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
