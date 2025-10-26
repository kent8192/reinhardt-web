# Part 4: Forms and Generic Views

In this tutorial, we'll process form submissions and refactor our views using generic views.

## Writing a Simple Form

Let's implement the voting functionality. Update the vote view in `src/polls.rs`:

```rust
use reinhardt::prelude::*;
use sqlx::SqlitePool;
use std::collections::HashMap;

pub async fn vote(request: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let pool = request.extensions.get::<SqlitePool>().unwrap();
    let question_id: i64 = request.path_params.get("question_id")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    // Get the question
    let question = crate::models::Question::get(pool, question_id)
        .await?
        .ok_or("Question not found")?;

    // Parse form data
    let body = String::from_utf8(request.body().to_vec())?;
    let form_data: HashMap<String, String> = serde_urlencoded::from_str(&body)?;

    // Get the selected choice
    let choice_id: i64 = form_data.get("choice")
        .and_then(|s| s.parse().ok())
        .ok_or("You didn't select a choice")?;

    // Verify the choice belongs to this question
    let choice = crate::models::Choice::get(pool, choice_id)
        .await?
        .ok_or("Choice not found")?;

    if choice.question_id != question_id {
        return Err("Invalid choice for this question".into());
    }

    // Increment the vote count
    crate::models::Choice::increment_votes(pool, choice_id).await?;

    // Redirect to results page
    Ok(redirect(&format!("/polls/{}/results/", question_id)))
}
```

Add the helper methods to `src/models.rs`:

```rust
impl Choice {
    /// Get a choice by ID
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

    /// Increment vote count for a choice
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

## Race Condition Prevention

The current implementation has a potential race condition. Two users voting at the same time might cause incorrect vote counts. To prevent this, we use database-level atomic operations.

The `UPDATE choices SET votes = votes + 1` query is atomic at the database level, so this implementation is already safe from race conditions.

## Adding CSRF Protection

Forms should be protected against Cross-Site Request Forgery (CSRF) attacks. Update the detail template to include CSRF protection:

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
      {% csrf_token %} {% for choice in question.choices %}
      <input
        type="radio"
        name="choice"
        id="choice{{ choice.id }}"
        value="{{ choice.id }}"
      />
      <label for="choice{{ choice.id }}">{{ choice.choice_text }}</label><br />
      {% endfor %}
      <input type="submit" value="Vote" />
    </form>
  </body>
</html>
```

## Use Generic Views: Less Code is Better

Our index, detail, and results views are very simple and represent a common case of basic web development: getting data from the database according to a parameter passed in the URL, loading a template and returning the rendered template.

Reinhardt provides a shortcut called "generic views" to handle these patterns.

Let's convert our views to use generic views. We'll need to update our code in several steps:

1. Update URL configuration
2. Delete old, unnecessary views
3. Introduce new views based on generic views

### Update URLs

Currently, we're using function-based views. Let's convert to class-based generic views.

Create a new file `src/views.rs`:

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

### Update main.rs

Update `src/main.rs` to use the new views:

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

## Summary

In this tutorial, you learned:

- How to process form submissions
- How to handle POST data
- How to protect against race conditions using atomic database operations
- How to add CSRF protection to forms
- How to use generic views (`ListView` and `DetailView`)
- How to reduce code by using class-based views

Generic views provide a powerful way to reduce boilerplate code while maintaining flexibility.

## What's Next?

In the next tutorial, we'll write automated tests for our application to ensure everything works correctly.

Continue to [Part 5: Testing](5-testing.md).