# Getting Started with Reinhardt

Welcome to Reinhardt! This guide will help you set up your first Reinhardt project and build a simple REST API.

## Prerequisites

Before you begin, make sure you have:

- **Rust** 1.75 or later ([Install Rust](https://www.rust-lang.org/tools/install))
- **PostgreSQL** (optional, for database features)
- Basic familiarity with Rust and async programming

## Installation

### Step 1: Create a New Project

```bash
cargo new my-api
cd my-api
```

### Step 2: Choose Your Flavor

Reinhardt comes in three flavors. Choose the one that fits your needs:

#### Option A: Micro (Lightweight)

For microservices and simple APIs:

```toml
[dependencies]
reinhardt-micro = "0.1.0"
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
```

#### Option B: Standard (Recommended)

Balanced setup for most projects:

```toml
[dependencies]
reinhardt = "0.1.0"  # or: { version = "0.1.0", features = ["standard"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
```

#### Option C: Full (Everything Included)

All features enabled:

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = ["full"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
```

For this guide, we'll use the **Standard** flavor.

## Your First API

Let's build a simple "Hello World" API.

### Step 3: Create Your First Endpoint

Edit `src/main.rs`:

```rust
use reinhardt::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct HelloResponse {
    message: String,
}

async fn hello_world() -> Result<JsonResponse<HelloResponse>, Error> {
    Ok(JsonResponse::new(HelloResponse {
        message: "Hello, Reinhardt!".to_string(),
    }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a router
    let mut router = Router::new();

    // Register the endpoint
    router.get("/hello", hello_world);

    // Start the server
    println!("Server running on http://127.0.0.1:8000");
    reinhardt::serve("127.0.0.1:8000", router).await?;

    Ok(())
}
```

### Step 4: Run Your API

```bash
cargo run
```

Visit `http://127.0.0.1:8000/hello` in your browser or use curl:

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

Now let's build a more realistic API with CRUD operations.

### Step 5: Define Your Model

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Todo {
    id: Option<i64>,
    title: String,
    completed: bool,
}

impl Todo {
    fn new(title: String) -> Self {
        Self {
            id: None,
            title,
            completed: false,
        }
    }
}
```

### Step 6: Create a ViewSet

```rust
use reinhardt::viewsets::ModelViewSet;
use std::sync::{Arc, Mutex};

// In-memory storage (replace with database in production)
type TodoStore = Arc<Mutex<Vec<Todo>>>;

async fn list_todos(store: Arc<TodoStore>) -> Result<JsonResponse<Vec<Todo>>, Error> {
    let todos = store.lock().unwrap().clone();
    Ok(JsonResponse::new(todos))
}

async fn create_todo(
    Json(mut todo): Json<Todo>,
    store: Arc<TodoStore>,
) -> Result<JsonResponse<Todo>, Error> {
    let mut todos = store.lock().unwrap();
    let id = todos.len() as i64 + 1;
    todo.id = Some(id);
    todos.push(todo.clone());
    Ok(JsonResponse::new(todo))
}
```

### Step 7: Set Up Routes

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = Arc::new(Mutex::new(Vec::new()));

    let mut router = Router::new();

    // Clone store for each route
    let store_list = store.clone();
    let store_create = store.clone();

    router.get("/todos", move || list_todos(store_list.clone()));
    router.post("/todos", move |json| create_todo(json, store_create.clone()));

    println!("Server running on http://127.0.0.1:8000");
    reinhardt::serve("127.0.0.1:8000", router).await?;

    Ok(())
}
```

### Step 8: Test Your API

```bash
# List todos (empty initially)
curl http://127.0.0.1:8000/todos

# Create a todo
curl -X POST http://127.0.0.1:8000/todos \
  -H "Content-Type: application/json" \
  -d '{"title":"Learn Reinhardt","completed":false}'

# List todos again
curl http://127.0.0.1:8000/todos
```

## Project Management Commands

Reinhardt provides Django-style management commands for common development tasks.

### Setting Up Management Commands

First, install the commands crate:

```toml
[dependencies]
reinhardt-commands = "0.1.0"
```

Create `src/bin/manage.rs` for project-specific commands:

```bash
# Use reinhardt-admin to create a new project with manage.rs included
cargo install reinhardt-commands
reinhardt-admin startproject myproject
```

Or manually create `src/bin/manage.rs` - see the [reinhardt-commands documentation](../crates/reinhardt-commands/README.md) for a complete example.

### Common Commands

```bash
# Database migrations
cargo run --bin manage makemigrations
cargo run --bin manage migrate

# Development server
cargo run --bin manage runserver

# Check project for issues
cargo run --bin manage check

# Collect static files
cargo run --bin manage collectstatic

# Interactive shell
cargo run --bin manage shell
```

### Global CLI Tool

Install `reinhardt-admin` globally for project scaffolding:

```bash
cargo install reinhardt-commands

# Create new projects
reinhardt-admin startproject myproject --template-type restful
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
reinhardt = { version = "0.1.0", features = ["standard", "database"] }
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-native-tls"] }
```

Check out the [ORM documentation](api/README.md) for more details.

> **Note**: Complete working examples are planned for future releases.

## Getting Help

- 📖 [API Reference](https://docs.rs/reinhardt)
- 💬 [GitHub Discussions](https://github.com/your-org/reinhardt/discussions)
- 🐛 [Report Issues](https://github.com/your-org/reinhardt/issues)

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
