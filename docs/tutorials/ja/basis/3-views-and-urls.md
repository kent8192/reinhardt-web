# パート3: ビューとURL

このチュートリアルでは、テンプレートを使用して投票データを表示するビューを作成します。

## さらにビューを作成

パート2で作成したデータベースモデルと連携するように`src/polls.rs`を更新しましょう。

`src/polls.rs`を更新します：

```rust
use reinhardt::prelude::*;
use sqlx::SqlitePool;
use std::collections::HashMap;

pub async fn index(request: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let pool = request.extensions.get::<SqlitePool>().unwrap();

    // 最新5件の質問を取得
    let questions = crate::models::Question::all(pool).await?;
    let latest_questions: Vec<_> = questions.into_iter().take(5).collect();

    let mut context = HashMap::new();
    context.insert("latest_question_list", serde_json::to_value(&latest_questions)?);

    render_template(&request, "polls/index.html", context)
}

pub async fn detail(request: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let pool = request.extensions.get::<SqlitePool>().unwrap();
    let question_id: i64 = request.path_params.get("question_id")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let question = crate::models::Question::get(pool, question_id)
        .await?
        .ok_or("Question not found")?;

    let mut context = HashMap::new();
    context.insert("question", serde_json::to_value(&question)?);

    render_template(&request, "polls/detail.html", context)
}

pub async fn results(request: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let pool = request.extensions.get::<SqlitePool>().unwrap();
    let question_id: i64 = request.path_params.get("question_id")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let question = crate::models::Question::get(pool, question_id)
        .await?
        .ok_or("Question not found")?;

    let choices = question.choices(pool).await?;

    let mut context = HashMap::new();
    context.insert("question", serde_json::to_value(&question)?);
    context.insert("choices", serde_json::to_value(&choices)?);

    render_template(&request, "polls/results.html", context)
}
```

## テンプレートの作成

テンプレートを使用すると、HTMLをRustコードから分離できます。テンプレートファイルを作成しましょう。

templatesディレクトリを作成します：

```bash
mkdir -p templates/polls
```

`templates/polls/index.html`を作成します：

```html
<!DOCTYPE html>
<html>
  <head>
    <title>投票</title>
  </head>
  <body>
    <h1>最新の投票</h1>

    {% if latest_question_list %}
    <ul>
      {% for question in latest_question_list %}
      <li>
        <a href="/polls/{{ question.id }}/">{{ question.question_text }}</a>
      </li>
      {% endfor %}
    </ul>
    {% else %}
    <p>利用可能な投票はありません。</p>
    {% endif %}
  </body>
</html>
```

`templates/polls/detail.html`を作成します：

```html
<!DOCTYPE html>
<html>
  <head>
    <title>{{ question.question_text }}</title>
  </head>
  <body>
    <h1>{{ question.question_text }}</h1>

    <form action="/polls/{{ question.id }}/vote/" method="post">
      {% for choice in question.choices %}
      <input
        type="radio"
        name="choice"
        id="choice{{ choice.id }}"
        value="{{ choice.id }}"
      />
      <label for="choice{{ choice.id }}">{{ choice.choice_text }}</label><br />
      {% endfor %}
      <input type="submit" value="投票" />
    </form>
  </body>
</html>
```

`templates/polls/results.html`を作成します：

```html
<!DOCTYPE html>
<html>
  <head>
    <title>{{ question.question_text }}の結果</title>
  </head>
  <body>
    <h1>{{ question.question_text }}</h1>

    <ul>
      {% for choice in choices %}
      <li>{{ choice.choice_text }} -- {{ choice.votes }} 票</li>
      {% endfor %}
    </ul>

    <a href="/polls/{{ question.id }}/">再度投票しますか？</a>
  </body>
</html>
```

## ショートカット関数の使用

Reinhardtは一般的なタスクを簡単にするショートカット関数を提供しています。すでに`render_template()`を使用しました。`get_object_or_404()`を見てみましょう：

```rust
use reinhardt::prelude::*;

pub async fn detail(request: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let pool = request.extensions.get::<SqlitePool>().unwrap();
    let question_id: i64 = request.path_params.get("question_id")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    // 質問が存在しない場合、自動的に404レスポンスを返します
    let question = get_object_or_404(
        crate::models::Question::get(pool, question_id)
    ).await?;

    let mut context = HashMap::new();
    context.insert("question", serde_json::to_value(&question)?);

    render_template(&request, "polls/detail.html", context)
}
```

`get_object_or_404()`関数はデータベースをクエリし、オブジェクトが存在しない場合は404 Not Foundエラーを発生させます。これにより、繰り返しのエラー処理コードを書く必要がなくなります。

## テンプレート内のハードコードされたURLの削除

現在、テンプレートには`/polls/{{ question.id }}/`のようなハードコードされたURLがあります。これはメンテナンスを困難にします。代わりにURL名前空間を使用しましょう。

`src/urls.rs`を更新してルートに名前を追加します：

```rust
use reinhardt::prelude::*;

pub fn url_patterns() -> Vec<Route> {
    vec![
        path("polls/", crate::polls::index).name("polls:index"),
        path("polls/{question_id}/", crate::polls::detail).name("polls:detail"),
        path("polls/{question_id}/results/", crate::polls::results).name("polls:results"),
        path("polls/{question_id}/vote/", crate::polls::vote).name("polls:vote"),
    ]
}
```

これで、テンプレートで`url`フィルタを使用できます：

```html
<a href="{% url 'polls:detail' question.id %}">{{ question.question_text }}</a>
```

## URL名の名前空間化

異なるアプリ間の名前の競合を避けるため、名前空間化されたURL名を使用します。形式は`app_name:url_name`です。

この場合：

- `polls:index` - pollsアプリのインデックスビュー
- `polls:detail` - pollsアプリの詳細ビュー
- `polls:results` - 結果ビュー
- `polls:vote` - 投票アクション

## main.rsの更新

`src/main.rs`を更新してテンプレート環境を設定します：

```rust
mod models;
mod polls;
mod urls;

use reinhardt::prelude::*;
use sqlx::SqlitePool;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // データベースをセットアップ
    let pool = SqlitePool::connect("sqlite:polls.db").await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    // テンプレートローダーをセットアップ
    let template_loader = Arc::new(FileSystemTemplateLoader::new("templates"));

    // ルーターを作成
    let mut router = DefaultRouter::new();

    // リクエストエクステンションにデータベースプールを追加
    router.add_extension(pool.clone());
    router.add_extension(template_loader);

    // URLパターンを登録
    for route in urls::url_patterns() {
        router.add_route(route);
    }

    // サーバーを起動
    let server = Server::new("127.0.0.1:8000", router);

    println!("Starting development server at http://127.0.0.1:8000/");
    println!("Quit the server with CTRL-C.");

    server.run().await?;

    Ok(())
}
```

## ビューのテスト

サーバーを実行します：

```bash
cargo run
```

以下のURLにアクセスします：

- `http://127.0.0.1:8000/polls/` - 投票のリストを表示
- `http://127.0.0.1:8000/polls/1/` - 投票#1の詳細を表示
- `http://127.0.0.1:8000/polls/1/results/` - 投票#1の結果を表示

## まとめ

このチュートリアルで学んだこと：

- データベースモデルを使用するビューの作成方法
- テンプレートを使用してHTMLをレンダリングする方法
- テンプレート変数と制御構造の使用方法
- `render_template()`や`get_object_or_404()`などのショートカット関数の使用方法
- ハードコードされたURLを避けるためのURL名前空間の使用方法
- テンプレートローダーの設定方法

## 次は何をする？

次のチュートリアルでは、ユーザーが実際に投票できるようにフォーム処理を追加します。

[パート4: フォームと汎用ビュー](4-forms-and-generic-views.md)に進んでください。