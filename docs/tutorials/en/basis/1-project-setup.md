# Part 1: Project Setup

In this tutorial, we'll create a new Reinhardt project and write our first view.

## Verifying Your Installation

Before we begin, let's verify that Rust and Cargo are installed correctly:

```bash
rustc --version
cargo --version
```

You should see version information for both commands. If not, visit
[rust-lang.org](https://www.rust-lang.org/tools/install) to install Rust.

## Installing Reinhardt Admin CLI

First, install the global tool for project generation:

```bash
cargo install reinhardt-admin-cli
```

## Creating a Project

Navigate to a directory where you'd like to store your code, then run:

```bash
reinhardt-admin startproject polls_project --template-type mtv
cd polls_project
```

This creates a `polls_project` directory with the following structure:

```
polls_project/
├── Cargo.toml
├── README.md
├── settings/
│   ├── base.toml
│   ├── local.toml
│   ├── staging.toml
│   └── production.toml
└── src/
    ├── config.rs
    ├── apps.rs
    ├── config/
    │   ├── settings.rs
    │   ├── urls.rs
    │   └── apps.rs
    └── bin/
        ├── runserver.rs
        └── manage.rs
```

**Note**: For this tutorial, we're using the **MTV (Model-Template-View)**
architecture pattern (Django terminology), which provides components for models,
views (using reinhardt-pages for rendering), forms, and the admin interface.

## Understanding the Project Structure

Let's understand the key elements of the generated project:

- `Cargo.toml` - Configuration file for your project and its dependencies
- `settings/` - Environment-specific settings files (base, local, staging,
  production)
- `src/config/` - Project configuration
  - `settings.rs` - Settings loader
  - `urls.rs` - URL routing configuration
  - `apps.rs` - Installed apps registration
- `src/bin/` - Executable files
  - `runserver.rs` - Development server
  - `manage.rs` - Management commands (equivalent to Django's `manage.py`)

## Understanding HTTP Method Decorators

Reinhardt provides FastAPI-inspired HTTP method decorators that make routing
concise and type-safe. These decorators automatically handle HTTP method
validation and route binding.

**Available decorators:**
- `#[get("/path")]` - Handle GET requests
- `#[post("/path")]` - Handle POST requests
- `#[put("/path")]`, `#[delete("/path")]`, `#[patch("/path")]` - Other HTTP methods

**Benefits:**
- **Concise syntax** - Less boilerplate code than traditional routing
- **Compile-time validation** - Path patterns are checked at compile time
- **Automatic method binding** - No manual HTTP method checks needed
- **Named routes** - Optional `name` parameter for URL reversal

**Without decorator (traditional approach):**

```rust
async fn index(req: Request) -> Result<Response> {
    // Manual method check needed
    if req.method() != Method::GET {
        return Err("Method not allowed".into());
    }

    // Handle the request
    Response::ok().with_body("Hello")
}
```

**With decorator (Reinhardt approach):**

```rust
#[get("/", name = "index")]
async fn index() -> Result<Response> {
    // Method check automatic!
    Response::ok().with_body("Hello")
}
```

The decorator approach reduces boilerplate and makes your code more maintainable.
For more advanced routing patterns, see [Part 3: Views and URLs](3-views-and-urls.md).

## Creating Your First View

A view in Reinhardt is a function that takes an HTTP request and returns an HTTP
response. With Reinhardt, you can use HTTP method decorators (`#[get]`, `#[post]`, etc.)
for FastAPI-style routing and dependency injection.

### View Return Types

Reinhardt provides a convenient type alias `ViewResult<T>` for view function
return types. This avoids repetitive error type annotations.

```rust
use reinhardt::ViewResult;  // Pre-defined: Result<T, Box<dyn std::error::Error>>

#[get("/", name = "index")]
async fn index() -> ViewResult<Response> {
    Response::ok().with_body("Hello")
}
```

**Why `ViewResult`?**

- **Concise** - Avoids repeating `Result<Response, Box<dyn std::error::Error>>`
- **Flexible** - Works with the `?` operator for automatic error conversion
- **Consistent** - Standard error handling pattern across all views

**Alternative (explicit typing):**

```rust
async fn index() -> Result<Response, Box<dyn std::error::Error>> {
    // Same as ViewResult<Response>
    Response::ok().with_body("Hello")
}
```

For this tutorial, we use the shorter `Result<Response>` form (which is an alias
to `ViewResult<Response>`), but both forms are equivalent.

Edit `src/main.rs`:

```rust
use reinhardt::prelude::*;
use reinhardt::get;

// Our first view - returns a simple text response
#[get("/", name = "index")]
async fn index() -> Result<Response> {
    Response::ok()
        .with_body("Hello, world. You're at the polls index.")
        .with_header("Content-Type", "text/plain")
}
```

This is the simplest view possible in Reinhardt. The `#[get]` decorator handles
the request parsing, routing, and dependency injection automatically.

## Mapping URLs to Views

To call this view, we need to map it to a URL. Reinhardt uses `UnifiedRouter`
for efficient O(m) route matching.

Create `src/config/urls.rs`:

```rust
use reinhardt::prelude::*;
use reinhardt::routes;

#[routes]
pub fn routes() -> UnifiedRouter {
    UnifiedRouter::new()
        .function("/", Method::GET, crate::index)
}
```

The `#[routes]` attribute macro automatically registers this function with the
framework for discovery via the `inventory` crate.

**Note**: Reinhardt projects generated by `reinhardt-admin startproject` already
include proper routing configuration in `src/bin/runserver.rs`. You don't need
to manually create a `main.rs` with server setup code.

## Running the Development Server

Now let's run the development server using the `runserver` command:

```bash
cargo run --bin runserver
```

**With auto-reload** (if you have `cargo-watch` installed):

```bash
# Install cargo-watch first (one-time setup)
cargo install cargo-watch

# Enable cargo-watch-reload feature in Cargo.toml
# [dependencies]
# reinhardt-commands = { version = "0.1.0-alpha.1", features = ["cargo-watch-reload"] }

# Run with auto-reload (detects changes and automatically rebuilds)
cargo run --bin runserver

# Optional: Clear screen before each rebuild
cargo run --bin runserver -- --clear
```

You should see output similar to:

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

Open your web browser and visit `http://127.0.0.1:8000/`. You should see a
welcome message.

Congratulations! Your Reinhardt project is now running!

## Understanding What Happened

Let's review what we just did:

1. **Created a view function** (`index`) that returns an HTTP response
2. **Created a URL pattern** that maps the root URL (`""`) to our view
3. **Configured a router** to handle incoming requests
4. **Started a development server** on port 8000

This is the basic request-response cycle in Reinhardt:

```
Browser Request → Server → Router → URL Pattern → View → Response → Browser
```

## Route Creation Explained

The `Route::from_handler()` function creates a route by taking two arguments:

```rust
Route::from_handler("", index)
```

- The first argument is the URL pattern (`""` means the root URL)
- The second argument is the view handler to call

You can create more complex patterns:

```rust
Route::from_handler("polls/", polls_index)
Route::from_handler("polls/{id}/", poll_detail)
```

The `{id}` syntax creates a URL parameter that will be passed to your view
handler.

## Creating the Polls App

In Reinhardt, we organize features into apps (similar to Django). Let's create a
`polls` app:

```bash
cargo run --bin manage startapp polls --template-type mtv
```

This creates a `polls` directory with the following structure:

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

### Creating a View

Edit `polls/views.rs`:

```rust
use reinhardt::prelude::*;
use reinhardt::get;

#[get("/", name = "index")]
pub async fn index() -> Result<Response> {
    Response::ok()
        .with_body("Hello, world. You're at the polls index.")
        .with_header("Content-Type", "text/plain")
}
```

### Setting Up URL Patterns

Edit `polls/urls.rs`:

```rust
use reinhardt::routers::UnifiedRouter;
use hyper::Method;
use crate::views;

pub fn url_patterns() -> UnifiedRouter {
    UnifiedRouter::new()
        .function("/", Method::GET, views::index)
}
```

### Registering with the Project

Edit `src/config/urls.rs` to include the polls app routes:

```rust
use reinhardt::prelude::*;
use reinhardt::routes;

#[routes]
pub fn routes() -> UnifiedRouter {
    UnifiedRouter::new()
        .mount("/polls/", polls::urls::url_patterns())
}
```

### Registering the App

Edit `src/config/apps.rs`:

```rust
use reinhardt::installed_apps;

installed_apps! {
    polls: "polls",
}

pub fn get_installed_apps() -> Vec<String> {
    InstalledApp::all_apps()
}
```

**Understanding `installed_apps!`**

The `installed_apps!` macro registers your application modules with Reinhardt's
app registry. This enables several framework features:

**What it does:**

1. **Auto-discovery** - Reinhardt automatically discovers:
   - Models for migrations
   - Admin panel registrations
   - Static files and templates
   - Management commands

2. **Type-safe references** - Creates type-safe app identifiers:
   ```rust
   // You can reference apps by name in code
   InstalledApp::Polls  // Type-safe reference to "polls" app
   ```

3. **Configuration registry** - Centralized app management
   - Apps are initialized in declaration order
   - Dependencies between apps can be managed
   - Apps can be conditionally included based on features

**Basic usage (single app):**

```rust
installed_apps! {
    polls: "polls",  // Your app
}
```

**With contrib apps (authentication, sessions, admin):**

```rust
use reinhardt::installed_apps;

installed_apps! {
    // Built-in framework apps
    auth: "reinhardt.contrib.auth",
    contenttypes: "reinhardt.contrib.contenttypes",
    sessions: "reinhardt.contrib.sessions",
    admin: "reinhardt.contrib.admin",

    // Your custom apps
    polls: "polls",
}
```

**Equivalent to Django's `INSTALLED_APPS`:**

```python
# Django's settings.py
INSTALLED_APPS = [
    'django.contrib.auth',
    'django.contrib.contenttypes',
    'django.contrib.sessions',
    'django.contrib.admin',
    'polls',
]
```

**Why the two-part syntax?**

- `polls:` - The identifier used in Rust code (`InstalledApp::Polls`)
- `"polls"` - The module path where the app is located

This allows flexibility in naming while keeping code references clean.

Restart your server (press Ctrl-C and run `cargo run --bin runserver` again) and
visit `http://127.0.0.1:8000/polls/`. You should see the message.

## Adding More Views

Let's add a few more views to make our URL configuration more interesting.
Update `polls/views.rs`:

```rust
use reinhardt::prelude::*;
use reinhardt::{get, post};
use reinhardt::http::Request;

#[get("/", name = "index")]
pub async fn index() -> Result<Response> {
    Response::ok()
        .with_body("Hello, world. You're at the polls index.")
        .with_header("Content-Type", "text/plain")
}

#[get("/{question_id}", name = "detail")]
pub async fn detail(request: Request) -> Result<Response> {
    // Extract the question_id from path parameters
    let question_id = request.path_params
        .get("question_id")
        .unwrap_or(&"0".to_string());

    Response::ok()
        .with_body(&format!("You're looking at question {}.", question_id))
        .with_header("Content-Type", "text/plain")
}

#[get("/{question_id}/results", name = "results")]
pub async fn results(request: Request) -> Result<Response> {
    let question_id = request.path_params
        .get("question_id")
        .unwrap_or(&"0".to_string());

    Response::ok()
        .with_body(&format!("You're looking at the results of question {}.", question_id))
        .with_header("Content-Type", "text/plain")
}

#[post("/{question_id}/vote", name = "vote")]
pub async fn vote(request: Request) -> Result<Response> {
    let question_id = request.path_params
        .get("question_id")
        .unwrap_or(&"0".to_string());

    Response::ok()
        .with_body(&format!("You're voting on question {}.", question_id))
        .with_header("Content-Type", "text/plain")
}
```

Update `polls/urls.rs` to wire these views:

```rust
use reinhardt::routers::UnifiedRouter;
use hyper::Method;
use crate::views;

pub fn url_patterns() -> UnifiedRouter {
    UnifiedRouter::new()
        // ex: /polls/
        .function("/", Method::GET, views::index)
        // ex: /polls/5/
        .function("/{question_id}", Method::GET, views::detail)
        // ex: /polls/5/results/
        .function("/{question_id}/results", Method::GET, views::results)
        // ex: /polls/5/vote/
        .function("/{question_id}/vote", Method::POST, views::vote)
}
```

Restart the server and try these URLs:

- `http://127.0.0.1:8000/polls/` - Shows the index
- `http://127.0.0.1:8000/polls/34/` - Shows detail for question 34
- `http://127.0.0.1:8000/polls/34/results/` - Shows results for question 34
- `http://127.0.0.1:8000/polls/34/vote/` - Shows voting form for question 34

## What's Next?

We've created a basic Reinhardt project with URL routing and simple views. In
the next tutorial, we'll set up a database and create models to store poll
questions and choices.

When you're ready, move on to
[Part 2: Models and Database](2-models-and-database.md).

## Summary

In this tutorial, you learned:

- How to create a new Reinhardt project
- How to define views as async functions
- How to map URLs to views using `Route::from_handler()`
- How to run the development server
- How to organize code into modules
- How to extract parameters from URLs

You now have a solid foundation for building Reinhardt applications!
