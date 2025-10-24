# Part 3: Views and URLs

In this tutorial, we'll create more views to display our poll data using templates.

## Writing More Views

Let's update our `src/polls.rs` to work with the database models we created in Part 2.

Update `src/polls.rs`:

```rust
use reinhardt::prelude::*;
use sqlx::SqlitePool;
use std::collections::HashMap;

pub async fn index(request: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let pool = request.extensions.get::<SqlitePool>().unwrap();

    // Get latest 5 questions
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
                <a href="/polls/{{ question.id }}/">{{ question.question_text }}</a>
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

    <form action="/polls/{{ question.id }}/vote/" method="post">
        {% for choice in question.choices %}
            <input type="radio" name="choice" id="choice{{ choice.id }}" value="{{ choice.id }}">
            <label for="choice{{ choice.id }}">{{ choice.choice_text }}</label><br>
        {% endfor %}
        <input type="submit" value="Vote">
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
        <li>{{ choice.choice_text }} -- {{ choice.votes }} vote{{ choice.votes|pluralize }}</li>
    {% endfor %}
    </ul>

    <a href="/polls/{{ question.id }}/">Vote again?</a>
</body>
</html>
```

## Using Shortcut Functions

Reinhardt provides shortcut functions to make common tasks easier. We've already used `render_template()`. Let's explore `get_object_or_404()`:

```rust
use reinhardt::prelude::*;

pub async fn detail(request: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let pool = request.extensions.get::<SqlitePool>().unwrap();
    let question_id: i64 = request.path_params.get("question_id")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    // This will automatically return a 404 response if the question doesn't exist
    let question = get_object_or_404(
        crate::models::Question::get(pool, question_id)
    ).await?;

    let mut context = HashMap::new();
    context.insert("question", serde_json::to_value(&question)?);

    render_template(&request, "polls/detail.html", context)
}
```

The `get_object_or_404()` function queries the database and raises a 404 Not Found error if the object doesn't exist, saving you from writing repetitive error handling code.

## Removing Hardcoded URLs in Templates

Currently, our templates have hardcoded URLs like `/polls/{{ question.id }}/`. This makes maintenance difficult. Let's use URL namespacing instead.

Update `src/urls.rs` to add names to our routes:

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

Now you can use the `url` filter in templates:

```html
<a href="{% url 'polls:detail' question.id %}">{{ question.question_text }}</a>
```

## Namespacing URL Names

To avoid name conflicts between different apps, we use namespaced URL names. The format is `app_name:url_name`.

In our case:
- `polls:index` - The index view of the polls app
- `polls:detail` - The detail view of the polls app
- `polls:results` - The results view
- `polls:vote` - The vote action

## Update main.rs

Update `src/main.rs` to configure the template environment:

```rust
mod models;
mod polls;
mod urls;

use reinhardt::prelude::*;
use sqlx::SqlitePool;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup database
    let pool = SqlitePool::connect("sqlite:polls.db").await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Setup template loader
    let template_loader = Arc::new(FileSystemTemplateLoader::new("templates"));

    // Create router
    let mut router = DefaultRouter::new();

    // Add database pool to request extensions
    router.add_extension(pool.clone());
    router.add_extension(template_loader);

    // Register URL patterns
    for route in urls::url_patterns() {
        router.add_route(route);
    }

    // Start server
    let server = Server::new("127.0.0.1:8000", router);

    println!("Starting development server at http://127.0.0.1:8000/");
    println!("Quit the server with CTRL-C.");

    server.run().await?;

    Ok(())
}
```

## Testing the Views

Run your server:

```bash
cargo run
```

Visit these URLs:

- `http://127.0.0.1:8000/polls/` - See the list of polls
- `http://127.0.0.1:8000/polls/1/` - See details for poll #1
- `http://127.0.0.1:8000/polls/1/results/` - See results for poll #1

## Summary

In this tutorial, you learned:

- How to create views that use database models
- How to use templates to render HTML
- How to use template variables and control structures
- How to use shortcut functions like `render_template()` and `get_object_or_404()`
- How to use URL namespacing to avoid hardcoded URLs
- How to configure template loaders

## What's Next?

In the next tutorial, we'll add form processing so users can actually vote on polls.

Continue to [Part 4: Forms and Generic Views](4-forms-and-generic-views.md).
