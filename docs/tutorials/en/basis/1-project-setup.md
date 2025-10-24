# Part 1: Project Setup

In this tutorial, we'll create a new Reinhardt project and write our first view.

## Verifying Your Installation

Before we begin, let's verify that Rust and Cargo are installed correctly:

```bash
rustc --version
cargo --version
```

You should see version information for both commands. If not, visit [rust-lang.org](https://www.rust-lang.org/tools/install) to install Rust.

## Creating a Project

Navigate to a directory where you'd like to store your code, then run:

```bash
cargo new polls_project
cd polls_project
```

This creates a `polls_project` directory with the following structure:

```
polls_project/
├── Cargo.toml
└── src/
    └── main.rs
```

## Adding Reinhardt Dependencies

Open `Cargo.toml` and add the Reinhardt dependencies. For this tutorial, we'll use the **standard** flavor which includes templates, forms, and the admin interface:

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

## Understanding the Project Structure

Let's understand what we have:

- `Cargo.toml` - Configuration file for your project and its dependencies
- `src/main.rs` - The entry point for your application

Unlike Django, Reinhardt doesn't have a `manage.py` file. Instead, you'll use Cargo commands to run your project.

## Creating Your First View

A view in Reinhardt is a function that takes an HTTP request and returns an HTTP response. Let's create a simple view.

Edit `src/main.rs`:

```rust
use reinhardt::prelude::*;

// Our first view - returns a simple text response
async fn index(_request: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let mut response = Response::new();
    response.set_status(StatusCode::OK);
    response.set_body("Hello, world. You're at the polls index.".to_string());
    response.set_header("Content-Type", "text/plain");
    Ok(response)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // We'll add routing configuration here next
    println!("Server setup complete");
    Ok(())
}
```

This is the simplest view possible in Reinhardt. It returns a plain text response with "Hello, world. You're at the polls index."

## Mapping URLs to Views

To call this view, we need to map it to a URL. Create a new file `src/urls.rs`:

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

Now update `src/main.rs` to use this URL configuration:

```rust
mod urls;

use reinhardt::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a router
    let mut router = DefaultRouter::new();

    // Register our URL patterns
    for route in urls::url_patterns() {
        router.add_route(route);
    }

    // Create and configure the server
    let server = Server::new("127.0.0.1:8000", router);

    println!("Starting development server at http://127.0.0.1:8000/");
    println!("Quit the server with CTRL-C.");

    // Run the server
    server.run().await?;

    Ok(())
}
```

## Running the Development Server

Now let's run the development server:

```bash
cargo run
```

You should see output similar to:

```
    Compiling polls_project v0.1.0 (/path/to/polls_project)
     Finished dev [unoptimized + debuginfo] target(s) in 2.34s
      Running `target/debug/polls_project`
Starting development server at http://127.0.0.1:8000/
Quit the server with CTRL-C.
```

Open your web browser and visit `http://127.0.0.1:8000/`. You should see the text:

```
Hello, world. You're at the polls index.
```

Congratulations! You've created your first Reinhardt view.

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

## Path Function Explained

The `path()` function takes two arguments:

```rust
path("", index)
```

- The first argument is the URL pattern (`""` means the root URL)
- The second argument is the view function to call

You can create more complex patterns:

```rust
path("polls/", polls_index)
path("polls/{id}/", poll_detail)
```

The `{id}` syntax creates a URL parameter that will be passed to your view.

## Creating the Polls App Module

In Reinhardt, it's best practice to organize your code into modules (similar to Django's apps). Let's create a `polls` module:

Create a new file `src/polls.rs`:

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

Update `src/urls.rs` to use this module:

```rust
use reinhardt::prelude::*;

pub fn url_patterns() -> Vec<Route> {
    vec![
        path("polls/", crate::polls::index),
    ]
}
```

Update `src/main.rs` to declare the module:

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

Restart your server (press Ctrl-C and run `cargo run` again) and visit `http://127.0.0.1:8000/polls/`. You should see the same message.

## Adding More Views

Let's add a few more views to make our URL configuration more interesting. Update `src/polls.rs`:

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
    // We'll extract the question_id from the URL in a later tutorial
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

Update `src/urls.rs` to wire these views into the URL configuration:

```rust
use reinhardt::prelude::*;

pub fn url_patterns() -> Vec<Route> {
    vec![
        // ex: /polls/
        path("polls/", crate::polls::index),
        // ex: /polls/5/
        path("polls/{question_id}/", crate::polls::detail),
        // ex: /polls/5/results/
        path("polls/{question_id}/results/", crate::polls::results),
        // ex: /polls/5/vote/
        path("polls/{question_id}/vote/", crate::polls::vote),
    ]
}
```

Restart the server and try these URLs:

- `http://127.0.0.1:8000/polls/` - Shows the index
- `http://127.0.0.1:8000/polls/34/` - Shows detail for question 34
- `http://127.0.0.1:8000/polls/34/results/` - Shows results for question 34
- `http://127.0.0.1:8000/polls/34/vote/` - Shows voting form for question 34

## What's Next?

We've created a basic Reinhardt project with URL routing and simple views. In the next tutorial, we'll set up a database and create models to store poll questions and choices.

When you're ready, move on to [Part 2: Models and Database](2-models-and-database.md).

## Summary

In this tutorial, you learned:

- How to create a new Reinhardt project
- How to define views as async functions
- How to map URLs to views using `path()`
- How to run the development server
- How to organize code into modules
- How to extract parameters from URLs

You now have a solid foundation for building Reinhardt applications!