# パート4: フォームと汎用ビュー

このチュートリアルでは、フォーム送信を処理し、汎用ビューを使用してビューをリファクタリングします。

## シンプルなフォームを作成

投票機能を実装しましょう。`src/polls.rs`のvoteビューを更新します：

```rust
use reinhardt::prelude::*;
use sqlx::SqlitePool;
use std::collections::HashMap;

pub async fn vote(request: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let pool = request.extensions.get::<SqlitePool>().unwrap();
    let question_id: i64 = request.path_params.get("question_id")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    // 質問を取得
    let question = crate::models::Question::get(pool, question_id)
        .await?
        .ok_or("Question not found")?;

    // フォームデータを解析
    let body = String::from_utf8(request.body().to_vec())?;
    let form_data: HashMap<String, String> = serde_urlencoded::from_str(&body)?;

    // 選択された選択肢を取得
    let choice_id: i64 = form_data.get("choice")
        .and_then(|s| s.parse().ok())
        .ok_or("You didn't select a choice")?;

    // 選択肢がこの質問に属することを確認
    let choice = crate::models::Choice::get(pool, choice_id)
        .await?
        .ok_or("Choice not found")?;

    if choice.question_id != question_id {
        return Err("Invalid choice for this question".into());
    }

    // 投票数を増やす
    crate::models::Choice::increment_votes(pool, choice_id).await?;

    // 結果ページにリダイレクト
    Ok(redirect(&format!("/polls/{}/results/", question_id)))
}
```

`src/models.rs`にヘルパーメソッドを追加します：

```rust
impl Choice {
    /// IDで選択肢を取得
    pub async fn get(pool: &SqlitePool, id: i64) -> Result<Option<Choice>, sqlx::Error> {
        let choice = sqlx::query_as!(
            Choice,
            "SELECT id, question_id, choice_text, votes FROM choices WHERE id = ?",
            id
        )
        .fetch_optional(pool)
        .await?;

        Ok(choice)
    }

    /// 選択肢の投票数を増やす
    pub async fn increment_votes(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE choices SET votes = votes + 1 WHERE id = ?",
            id
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
```

## レースコンディションの防止

現在の実装には潜在的なレースコンディションがあります。2人のユーザーが同時に投票すると、不正確な投票数になる可能性があります。これを防ぐため、データベースレベルのアトミック操作を使用します。

`UPDATE choices SET votes = votes + 1`クエリはデータベースレベルでアトミックなので、この実装はすでにレースコンディションから安全です。

## CSRF保護の追加

フォームはクロスサイトリクエストフォージェリ（CSRF）攻撃から保護する必要があります。detailテンプレートを更新してCSRF保護を含めます：

`templates/polls/detail.html`:

```html
<!DOCTYPE html>
<html>
<head>
    <title>{{ question.question_text }}</title>
</head>
<body>
    <h1>{{ question.question_text }}</h1>

    {% if error_message %}
        <p><strong>{{ error_message }}</strong></p>
    {% endif %}

    <form action="/polls/{{ question.id }}/vote/" method="post">
        {% csrf_token %}
        {% for choice in question.choices %}
            <input type="radio" name="choice" id="choice{{ choice.id }}" value="{{ choice.id }}">
            <label for="choice{{ choice.id }}">{{ choice.choice_text }}</label><br>
        {% endfor %}
        <input type="submit" value="投票">
    </form>
</body>
</html>
```

## 汎用ビューの使用: コードが少ないほど良い

index、detail、resultsビューは非常にシンプルで、基本的なWeb開発の一般的なケースを表しています：URLで渡されたパラメータに従ってデータベースからデータを取得し、テンプレートをロードして、レンダリングされたテンプレートを返します。

Reinhardtは、これらのパターンを処理するための「汎用ビュー」と呼ばれるショートカットを提供します。

ビューを汎用ビューを使用するように変換しましょう。いくつかのステップでコードを更新する必要があります：

1. URL設定を更新
2. 古い不要なビューを削除
3. 汎用ビューに基づく新しいビューを導入

### URLを更新

現在、関数ベースのビューを使用しています。クラスベースの汎用ビューに変換しましょう。

新しいファイル`src/views.rs`を作成します：

```rust
use reinhardt::prelude::*;
use sqlx::SqlitePool;
use std::collections::HashMap;

pub struct QuestionListView;

impl ListView for QuestionListView {
    type Model = crate::models::Question;

    async fn get_queryset(&self, request: &Request) -> Result<Vec<Self::Model>, Box<dyn std::error::Error + Send + Sync>> {
        let pool = request.extensions.get::<SqlitePool>().unwrap();
        let questions = crate::models::Question::all(pool).await?;
        Ok(questions.into_iter().take(5).collect())
    }

    fn get_template_name(&self) -> &str {
        "polls/index.html"
    }

    fn get_context_object_name(&self) -> &str {
        "latest_question_list"
    }
}

pub struct QuestionDetailView;

impl DetailView for QuestionDetailView {
    type Model = crate::models::Question;

    async fn get_object(&self, request: &Request) -> Result<Self::Model, Box<dyn std::error::Error + Send + Sync>> {
        let pool = request.extensions.get::<SqlitePool>().unwrap();
        let question_id: i64 = request.path_params.get("question_id")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        crate::models::Question::get(pool, question_id)
            .await?
            .ok_or("Question not found".into())
    }

    fn get_template_name(&self) -> &str {
        "polls/detail.html"
    }

    fn get_context_object_name(&self) -> &str {
        "question"
    }
}

pub struct ResultsView;

impl DetailView for ResultsView {
    type Model = crate::models::Question;

    async fn get_object(&self, request: &Request) -> Result<Self::Model, Box<dyn std::error::Error + Send + Sync>> {
        let pool = request.extensions.get::<SqlitePool>().unwrap();
        let question_id: i64 = request.path_params.get("question_id")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        crate::models::Question::get(pool, question_id)
            .await?
            .ok_or("Question not found".into())
    }

    fn get_template_name(&self) -> &str {
        "polls/results.html"
    }

    fn get_context_object_name(&self) -> &str {
        "question"
    }

    async fn get_context_data(&self, request: &Request, object: &Self::Model) -> Result<HashMap<String, serde_json::Value>, Box<dyn std::error::Error + Send + Sync>> {
        let pool = request.extensions.get::<SqlitePool>().unwrap();
        let choices = object.choices(pool).await?;

        let mut context = HashMap::new();
        context.insert("question".to_string(), serde_json::to_value(object)?);
        context.insert("choices".to_string(), serde_json::to_value(&choices)?);

        Ok(context)
    }
}
```

### main.rsを更新

`src/main.rs`を更新して新しいビューを使用します：

```rust
mod models;
mod polls;
mod urls;
mod views;

use reinhardt::prelude::*;
use sqlx::SqlitePool;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = SqlitePool::connect("sqlite:polls.db").await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let template_loader = Arc::new(FileSystemTemplateLoader::new("templates"));

    let mut router = DefaultRouter::new();
    router.add_extension(pool.clone());
    router.add_extension(template_loader);

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

## まとめ

このチュートリアルで学んだこと：

- フォーム送信の処理方法
- POSTデータの処理方法
- アトミックデータベース操作を使用したレースコンディションの防止方法
- フォームへのCSRF保護の追加方法
- 汎用ビュー（`ListView`と`DetailView`）の使用方法
- クラスベースビューを使用したコードの削減方法

汎用ビューは、柔軟性を維持しながらボイラープレートコードを削減する強力な方法を提供します。

## 次は何をする？

次のチュートリアルでは、すべてが正しく機能することを確認するために、アプリケーションの自動テストを作成します。

[パート5: テスト](5-testing.md)に進んでください。
