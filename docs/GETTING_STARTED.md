# Getting Started with Reinhardt

Welcome to Reinhardt! This guide will help you set up your first Reinhardt project and build a simple REST API.

## Prerequisites

Before you begin, make sure you have:

- **Rust** 1.75 or later ([Install Rust](https://www.rust-lang.org/tools/install))
- **PostgreSQL** (optional, for database features)
- Basic familiarity with Rust and async programming

## Installation

### Step 1: Install Reinhardt Admin

```bash
cargo install reinhardt-admin-cli
```

### Step 2: Create a New Project

```bash
# Create a RESTful API project
reinhardt-admin startproject my-api --template-type restful
cd my-api

# Or create a Model-Template-View (MTV) project
reinhardt-admin startproject my-web --template-type mtv
```

This generates a complete project structure with:
- Configuration files (`src/config/`)
- Settings management with environment support
- URL routing configuration
- App registry
- Management commands (`src/bin/manage.rs`)
- Development server (`src/bin/runserver.rs`)

### Step 3: Choose Your Flavor

Reinhardt comes in three flavors. Choose the one that fits your needs:

#### Option A: Micro (Lightweight)

For microservices and simple APIs:

```toml
[dependencies]
reinhardt-micro = "0.1.0-alpha.1"
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
```

#### Option B: Standard (Recommended)

Balanced setup for most projects:

```toml
[dependencies]
reinhardt = "0.1.0-alpha.1"  # or: { version = "0.1.0-alpha.1", features = ["standard"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
```

#### Option C: Full (Everything Included)

All features enabled:

```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["full"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
```

For this guide, we'll use the **Standard** flavor.

**💡 Want more control?** See the [Feature Flags Guide](FEATURE_FLAGS.md) for detailed information on 70+ individual feature flags to fine-tune your build.

The project template already includes all necessary dependencies in `Cargo.toml`.

## Your First API

The generated project already has a working server! Let's customize it.

### Step 4: Run the Development Server

```bash
# Using the runserver binary (recommended)
cargo run --bin runserver

# Or using manage command
cargo run --bin manage runserver
```

Visit `http://127.0.0.1:8000/` in your browser. You should see a welcome message.

### Step 5: Create Your First App

```bash
# Create a new app for our API
cargo run --bin manage startapp hello --template-type restful
```

This creates a `hello` app with the following structure:

```
hello/
├── lib.rs
├── models.rs
├── views.rs
├── serializers.rs
├── urls.rs
├── admin.rs
└── tests.rs
```

### Step 6: Create a Simple Endpoint

Edit `hello/views.rs`:

```rust
use reinhardt_http::{Request, Response, StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct HelloResponse {
    message: String,
}

pub async fn hello_world(_req: Request) -> Result<Response, Box<dyn std::error::Error>> {
    let response_data = HelloResponse {
        message: "Hello, Reinhardt!".to_string(),
    };

    let json = serde_json::to_string(&response_data)?;
    Ok(Response::new(StatusCode::OK, json.into()))
}
```

### Step 7: Register the Route

Edit `hello/urls.rs`:

```rust
use reinhardt_routers::UnifiedRouter;
use hyper::Method;
use crate::views;

pub fn url_patterns() -> UnifiedRouter {
    UnifiedRouter::new()
        .function("/hello", Method::GET, views::hello_world)
}
```

### Step 8: Include in Project URLs

Edit `src/config/urls.rs`:

```rust
use reinhardt::prelude::*;
use std::sync::Arc;

pub fn url_patterns() -> Arc<UnifiedRouter> {
    let router = UnifiedRouter::new()
        .mount("/", hello::urls::url_patterns());

    Arc::new(router)
}
```

### Step 9: Register the App

Edit `src/config/apps.rs`:

```rust
use reinhardt_macros::installed_apps;

installed_apps! {
    hello: "hello",
}

pub fn get_installed_apps() -> Vec<String> {
    InstalledApp::all_apps()
}
```

### Step 10: Test Your Endpoint

Restart the server and visit:

```bash
curl http://127.0.0.1:8000/hello
```

You should see:

```json
{
  "message": "Hello, Reinhardt!"
}
```

## Building a Real API

Now let's build a more realistic API with CRUD operations using ViewSets.

### Step 11: Create a Todos App

```bash
cargo run --bin manage startapp todos --template-type restful
```

### Step 12: Define Your Model

Edit `todos/models.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: Option<i64>,
    pub title: String,
    pub completed: bool,
}

impl Todo {
    pub fn new(title: String) -> Self {
        Self {
            id: None,
            title,
            completed: false,
        }
    }
}
```

### Step 13: Create Serializer

Edit `todos/serializers.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoSerializer {
    pub id: Option<i64>,
    pub title: String,
    pub completed: bool,
}
```

### Step 14: Create a ViewSet

Edit `todos/views.rs`:

```rust
use reinhardt::viewsets::ModelViewSet;
use crate::models::Todo;
use crate::serializers::TodoSerializer;

// Create a ViewSet for automatic CRUD operations
pub struct TodoViewSet;

impl TodoViewSet {
    pub fn new() -> ModelViewSet<Todo, TodoSerializer> {
        ModelViewSet::new("todo")
    }
}
```

### Step 15: Register ViewSet in URLs

Edit `todos/urls.rs`:

```rust
use reinhardt_routers::UnifiedRouter;
use std::sync::Arc;
use crate::views::TodoViewSet;

pub fn url_patterns() -> UnifiedRouter {
    // Register ViewSet - this creates all CRUD endpoints automatically
    UnifiedRouter::new()
        .viewset("/todos", Arc::new(TodoViewSet::new()))
}
```

This automatically creates:
- `GET /todos/` - List all todos
- `POST /todos/` - Create a new todo
- `GET /todos/{id}/` - Retrieve a todo
- `PUT /todos/{id}/` - Update a todo
- `DELETE /todos/{id}/` - Delete a todo

### Step 16: Include in Project

Edit `src/config/urls.rs`:

```rust
use reinhardt::prelude::*;
use std::sync::Arc;

pub fn url_patterns() -> Arc<UnifiedRouter> {
    let router = UnifiedRouter::new()
        .mount("/api/", todos::urls::url_patterns());

    Arc::new(router)
}
```

Edit `src/config/apps.rs`:

```rust
use reinhardt_macros::installed_apps;

installed_apps! {
    todos: "todos",
}

pub fn get_installed_apps() -> Vec<String> {
    InstalledApp::all_apps()
}
```

### Step 17: Test Your API

```bash
# List todos (empty initially)
curl http://127.0.0.1:8000/api/todos/

# Create a todo
curl -X POST http://127.0.0.1:8000/api/todos/ \
  -H "Content-Type: application/json" \
  -d '{"title":"Learn Reinhardt","completed":false}'

# Get a specific todo
curl http://127.0.0.1:8000/api/todos/1/

# Update a todo
curl -X PUT http://127.0.0.1:8000/api/todos/1/ \
  -H "Content-Type: application/json" \
  -d '{"title":"Learn Reinhardt","completed":true}'

# Delete a todo
curl -X DELETE http://127.0.0.1:8000/api/todos/1/
```

## Project Management Commands

The generated project includes `src/bin/manage.rs` for Django-style management commands.

### Common Commands

```bash
# Create a new app
cargo run --bin manage startapp myapp --template-type restful

# Development server
cargo run --bin manage runserver

# Database migrations (when using database features)
cargo run --bin manage makemigrations
cargo run --bin manage migrate

# Check project for issues
cargo run --bin manage check

# Collect static files
cargo run --bin manage collectstatic

# Interactive shell
cargo run --bin manage shell
```

### Global CLI Tool

You already installed `reinhardt-admin` in Step 1. Use it for:

```bash
# Create new projects
reinhardt-admin startproject myproject --template-type restful
reinhardt-admin startproject myweb --template-type mtv

# Create new apps (from project root)
reinhardt-admin startapp myapp --template-type restful
```

For more details, see the [reinhardt-commands documentation](../crates/reinhardt-commands/README.md).

## Next Steps

Congratulations! You've built your first Reinhardt API. Here's what to explore next:

### 📚 Tutorials

- [Tutorial 1: Serialization](tutorials/en/rest/1-serialization.md)
- [Tutorial 2: Requests and Responses](tutorials/en/rest/2-requests-and-responses.md)
- [Tutorial 3: Class-Based Views](tutorials/en/rest/3-class-based-views.md)
- [Tutorial 4: Authentication & Permissions](tutorials/en/rest/4-authentication-and-permissions.md)

### 🎛️ Advanced Features

- [Dependency Injection](tutorials/en/07-dependency-injection.md) - FastAPI-style DI
- [Feature Flags Guide](FEATURE_FLAGS.md) - Optimize your build
- [Database Integration](#database-integration) - Connect to PostgreSQL/MySQL
- [Management Commands](../crates/reinhardt-commands/README.md) - Django-style CLI

### 🔌 Database Integration

To use a database instead of in-memory storage:

```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "database"] }
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-native-tls"] }
```

Check out the [ORM documentation](api/README.md) for more details.

> **Note**: Complete working examples are planned for future releases.

## Getting Help

- 📖 [API Reference](https://docs.rs/reinhardt)
- 💬 [GitHub Discussions](https://github.com/kent8192/reinhardt/discussions)
- 🐛 [Report Issues](https://github.com/kent8192/reinhardt/issues)

## Common Issues

### Port Already in Use

If you see "Address already in use", change the port:

```rust
reinhardt::serve("127.0.0.1:3000", router).await?;
```

### Compilation Errors

Make sure you're using Rust 1.75 or later:

```bash
rustc --version
rustup update
```

### Async Runtime

Reinhardt requires an async runtime. Make sure you have `#[tokio::main]` on your main function:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Your code here
}
```

---

Happy coding with Reinhardt! 🚀
