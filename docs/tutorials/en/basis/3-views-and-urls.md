# Part 3: Views and URLs

In this tutorial, we'll create more views to display our poll data using templates and Reinhardt's modern endpoint system.

## Writing More Views with Endpoint Macro

Let's update our `polls/views.rs` to use the `#[endpoint]` macro and dependency injection.

Update `polls/views.rs`:

```rust
use reinhardt::prelude::*;
use reinhardt_macros::endpoint;
use reinhardt_db::backends::DatabaseConnection;
use std::sync::Arc;
use serde_json::json;
use crate::models::{Question, Choice};

#[endpoint]
pub async fn index(
    #[inject] conn: Arc<DatabaseConnection>,
) -> Result<Response> {
    // Get latest 5 questions
    let questions = Question::all(&conn).await?;
    let latest_questions: Vec<_> = questions.into_iter().take(5).collect();

    let context = json!({
        "latest_question_list": latest_questions,
    });

    Response::ok()
        .render_template("polls/index.html", &context)
}

#[endpoint]
pub async fn detail(
    request: Request,
    #[inject] conn: Arc<DatabaseConnection>,
) -> Result<Response> {
    let question_id: i64 = request.path_params
        .get("question_id")
        .ok_or("Missing question_id")?
        .parse()?;

    let question = Question::get(&conn, question_id)
        .await?
        .ok_or_else(|| Response::not_found().with_body("Question not found"))?;

    let choices = question.choices(&conn).await?;

    let context = json!({
        "question": question,
        "choices": choices,
    });

    Response::ok()
        .render_template("polls/detail.html", &context)
}

#[endpoint]
pub async fn results(
    request: Request,
    #[inject] conn: Arc<DatabaseConnection>,
) -> Result<Response> {
    let question_id: i64 = request.path_params
        .get("question_id")
        .ok_or("Missing question_id")?
        .parse()?;

    let question = Question::get(&conn, question_id)
        .await?
        .ok_or_else(|| Response::not_found().with_body("Question not found"))?;

    let choices = question.choices(&conn).await?;

    let context = json!({
        "question": question,
        "choices": choices,
    });

    Response::ok()
        .render_template("polls/results.html", &context)
}

#[endpoint]
pub async fn vote(
    mut request: Request,
    #[inject] conn: Arc<DatabaseConnection>,
) -> Result<Response> {
    let question_id: i64 = request.path_params
        .get("question_id")
        .ok_or("Missing question_id")?
        .parse()?;

    // Parse form data
    let form_data = request.parse_form().await?;
    let choice_id: i64 = form_data
        .get("choice")
        .ok_or("No choice selected")?
        .parse()?;

    // Get the choice and increment votes
    let mut choice = Choice::get(&conn, choice_id)
        .await?
        .ok_or("Choice not found")?;

    choice.increment_votes(&conn).await?;

    // Redirect to results page
    Response::redirect(&format!("/polls/{}/results/", question_id))
}
```

**Key improvements:**

- `#[endpoint]` macro handles request parsing automatically
- `#[inject]` provides database connection via dependency injection
- `Response::ok().render_template()` replaces manual template rendering
- `Response::not_found()` and `Response::redirect()` for HTTP status codes
- No manual `extensions.get::<Pool>()` - DI handles it

## Creating Templates

Templates allow you to separate HTML from your Rust code. Let's create template files.

Create the templates directory:

```bash
mkdir -p templates/polls
```

Create `templates/polls/index.html`:

```html
<!DOCTYPE html>
<html>
  <head>
    <title>Polls</title>
  </head>
  <body>
    <h1>Latest Polls</h1>

    {% if latest_question_list %}
    <ul>
      {% for question in latest_question_list %}
      <li>
        <a href="{% url 'polls:detail' question.id %}">{{ question.question_text }}</a>
      </li>
      {% endfor %}
    </ul>
    {% else %}
    <p>No polls are available.</p>
    {% endif %}
  </body>
</html>
```

Create `templates/polls/detail.html`:

```html
<!DOCTYPE html>
<html>
  <head>
    <title>{{ question.question_text }}</title>
  </head>
  <body>
    <h1>{{ question.question_text }}</h1>

    <form action="{% url 'polls:vote' question.id %}" method="post">
      {% csrf_token %}
      {% for choice in choices %}
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

Create `templates/polls/results.html`:

```html
<!DOCTYPE html>
<html>
  <head>
    <title>Results for {{ question.question_text }}</title>
  </head>
  <body>
    <h1>{{ question.question_text }}</h1>

    <ul>
      {% for choice in choices %}
      <li>
        {{ choice.choice_text }} -- {{ choice.votes }} vote{{
        choice.votes|pluralize }}
      </li>
      {% endfor %}
    </ul>

    <a href="{% url 'polls:detail' question.id %}">Vote again?</a>
  </body>
</html>
```

## Using Shortcut Functions

Reinhardt provides shortcut functions to make common tasks easier. The `Response` builder pattern includes several shortcuts:

```rust
// 404 Not Found
Response::not_found()
    .with_body("Question not found")

// 302 Redirect
Response::redirect("/polls/1/results/")

// 200 OK with JSON
Response::ok()
    .with_json(&data)?

// 200 OK with template rendering
Response::ok()
    .render_template("polls/index.html", &context)?
```

These replace manual error handling and status code management.

## Configuring URL Patterns with UnifiedRouter

Update `polls/urls.rs` to use UnifiedRouter with namespacing:

```rust
use reinhardt_routers::UnifiedRouter;
use hyper::Method;
use crate::views;

pub fn url_patterns() -> UnifiedRouter {
    UnifiedRouter::new()
        .with_namespace("polls")
        .function("/", Method::GET, views::index)
        .function("/:question_id", Method::GET, views::detail)
        .function("/:question_id/results", Method::GET, views::results)
        .function("/:question_id/vote", Method::POST, views::vote)
}
```

**Namespace benefits:**

- URLs are automatically named: `polls:index`, `polls:detail`, etc.
- No naming conflicts with other apps
- Use in templates with `{% url 'polls:detail' question.id %}`

## Mounting the Polls Router

Update `src/config/urls.rs` to mount the polls router:

```rust
use reinhardt_routers::UnifiedRouter;
use hyper::Method;
use std::sync::Arc;

pub fn url_patterns() -> Arc<UnifiedRouter> {
    let router = UnifiedRouter::new()
        .mount("/polls", polls::urls::url_patterns());

    Arc::new(router)
}
```

The `mount()` method:
- Prefixes all routes with `/polls`
- Preserves namespace (`polls:index`, `polls:detail`, etc.)
- Enables hierarchical routing

## Template Configuration

Reinhardt projects generated by `reinhardt-admin startproject` automatically configure templates. The template loader is set up in `src/bin/runserver.rs`.

If you need to customize template configuration, update `settings/base.toml`:

```toml
[templates]
dirs = ["templates"]
debug = true
```

## URL Reversal in Code

You can also reverse URLs in Rust code:

```rust
use reinhardt_urls::reverse;

let detail_url = reverse("polls:detail", &[("question_id", "5")])?;
// Returns: "/polls/5"

let redirect = Response::redirect(&detail_url);
```

## Testing the Views

Run your server:

```bash
cargo run --bin runserver
```

Visit these URLs:

- `http://127.0.0.1:8000/polls/` - See the list of polls
- `http://127.0.0.1:8000/polls/1/` - See details for poll #1
- `http://127.0.0.1:8000/polls/1/results/` - See results for poll #1

## Response Builder Patterns

Reinhardt's `Response` type uses a builder pattern for flexibility:

```rust
// Simple text response
Response::ok()
    .with_body("Hello, world!")

// JSON response
Response::ok()
    .with_json(&data)?
    .with_header("X-Custom-Header", "value")

// Template response
Response::ok()
    .render_template("template.html", &context)?

// Redirect
Response::redirect("/new-location")

// Error responses
Response::not_found()
    .with_body("Page not found")

Response::bad_request()
    .with_body("Invalid input")

Response::internal_server_error()
    .with_body("Something went wrong")
```

## Summary

In this tutorial, you learned:

- How to use the `#[endpoint]` macro for FastAPI-style views
- How to use dependency injection with `#[inject]` for database connections
- How to use the Response builder pattern (`.ok()`, `.redirect()`, `.not_found()`)
- How to create and use templates with the template system
- How to use UnifiedRouter with namespacing
- How to use template tags like `{% url %}` for URL reversal
- How to configure URL patterns hierarchically with `mount()`

## What's Next?

In the next tutorial, we'll add form processing and introduce generic views to reduce boilerplate code.

Continue to [Part 4: Forms and Generic Views](4-forms-and-generic-views.md).
