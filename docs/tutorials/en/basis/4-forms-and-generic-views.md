# Part 4: Forms and Generic Views

In this tutorial, we'll process form submissions and refactor our views using generic views.

## Writing a Simple Form

Let's implement the voting functionality using Reinhardt's modern endpoint system. Update the vote view in `polls/views.rs`:

```rust
use reinhardt::prelude::*;
use reinhardt_macros::endpoint;
use reinhardt_db::backends::DatabaseConnection;
use std::sync::Arc;

#[derive(serde::Deserialize)]
struct VoteForm {
    choice: i64,
}

#[endpoint]
pub async fn vote(
    request: Request,
    #[inject] conn: Arc<DatabaseConnection>,
) -> Result<Response> {
    // Extract question_id from path parameters
    let question_id: i64 = request.path_params
        .get("question_id")
        .ok_or("Missing question_id")?
        .parse()?;

    // Get the question
    let question = crate::models::Question::get(&conn, question_id)
        .await?
        .ok_or("Question not found")?;

    // Parse form data using Reinhardt's helper method
    let form_data: VoteForm = request.parse_form().await?;
    let choice_id = form_data.choice;

    // Verify the choice belongs to this question
    let choice = crate::models::Choice::get(&conn, choice_id)
        .await?
        .ok_or("Choice not found")?;

    if choice.question_id != question_id {
        return Err("Invalid choice for this question".into());
    }

    // Increment the vote count
    crate::models::Choice::increment_votes(&conn, choice_id).await?;

    // Redirect to results page
    Response::redirect(&format!("/polls/{}/results/", question_id))
}
```

**Key improvements:**
- `#[endpoint]` macro for automatic request handling
- `#[inject]` for dependency injection of database connection
- `request.parse_form().await?` for clean form parsing
- `Response::redirect()` for type-safe redirects
- No manual `extensions.get()` - DI handles it

Since we're using `#[derive(Model)]`, the `get` method is automatically available through the QuerySet API. For incrementing votes, we'll use F expressions for atomic database updates:

```rust
use reinhardt::prelude::*;

// Get a choice by ID (using generated QuerySet methods)
let choice = Choice::objects()
    .filter(Choice::field_id().eq(choice_id))
    .first(&conn)
    .await?;

// Increment vote count using F expression (atomic operation)
Choice::objects()
    .filter(Choice::field_id().eq(choice_id))
    .update()
    .set(Choice::field_votes(), F::new(Choice::field_votes()) + 1)
    .execute(&conn)
    .await?;
```

**Why F Expressions?**
- **Atomic**: Database-level operation prevents race conditions
- **Type-safe**: Compile-time verification of field types
- **Efficient**: Single SQL UPDATE query, no SELECT needed

## Race Condition Prevention

The F expression approach automatically prevents race conditions:

```rust
// ✅ SAFE: Atomic database operation
Choice::objects()
    .filter(Choice::field_id().eq(choice_id))
    .update()
    .set(Choice::field_votes(), F::new(Choice::field_votes()) + 1)
    .execute(&conn)
    .await?;
```

**Why this is safe:**
1. Single UPDATE query at database level
2. No SELECT needed (avoids read-modify-write race)
3. Database ensures atomicity of the increment

**Unsafe alternative (DON'T DO THIS):**
```rust
// ❌ UNSAFE: Race condition possible
let mut choice = Choice::objects()
    .filter(Choice::field_id().eq(choice_id))
    .first(&conn)
    .await?;
choice.votes += 1;  // Two requests could read same value
choice.save(&conn).await?;  // Last write wins, lost update
```

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
use std::sync::Arc;

pub struct QuestionListView;

impl ListView for QuestionListView {
    type Model = crate::models::Question;

    #[endpoint]
    async fn get_queryset(
        &self,
        request: &Request,
        #[inject] db: Arc<DatabaseConnection>,
    ) -> Result<Vec<Self::Model>, Box<dyn std::error::Error + Send + Sync>> {
        let questions = Question::objects()
            .order_by(Question::field_pub_date(), false)
            .limit(5)
            .all(&db)
            .await?;
        Ok(questions)
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

    #[endpoint]
    async fn get_object(
        &self,
        request: &Request,
        #[inject] db: Arc<DatabaseConnection>,
    ) -> Result<Self::Model, Box<dyn std::error::Error + Send + Sync>> {
        let question_id: i64 = request.path_params.get("question_id")
            .and_then(|s| s.parse().ok())
            .ok_or("Invalid question_id")?;

        Question::objects()
            .filter(Question::field_id().eq(question_id))
            .first(&db)
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

    #[endpoint]
    async fn get_object(
        &self,
        request: &Request,
        #[inject] db: Arc<DatabaseConnection>,
    ) -> Result<Self::Model, Box<dyn std::error::Error + Send + Sync>> {
        let question_id: i64 = request.path_params.get("question_id")
            .and_then(|s| s.parse().ok())
            .ok_or("Invalid question_id")?;

        Question::objects()
            .filter(Question::field_id().eq(question_id))
            .first(&db)
            .await?
            .ok_or("Question not found".into())
    }

    fn get_template_name(&self) -> &str {
        "polls/results.html"
    }

    fn get_context_object_name(&self) -> &str {
        "question"
    }

    #[endpoint]
    async fn get_context_data(
        &self,
        request: &Request,
        object: &Self::Model,
        #[inject] db: Arc<DatabaseConnection>,
    ) -> Result<HashMap<String, serde_json::Value>, Box<dyn std::error::Error + Send + Sync>> {
        let choices = object.choices(&db).await?;

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