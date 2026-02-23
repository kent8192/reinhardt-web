+++
title = "Quickstart"
description = "Get started with Reinhardt — quickstart guide, full setup, and tutorials."
sort_by = "weight"
weight = 10
+++

# Quickstart

Get up and running with Reinhardt in 5 minutes.

## 1. Add to Cargo.toml

```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha", package = "reinhardt-web" }
```

## 2. Create your first app

```rust
use reinhardt::{ServerRouter, DefaultRouter, get, Response};

#[get("/")]
async fn index() -> Response {
    Response::ok().with_body("Hello, Reinhardt!")
}

#[tokio::main]
async fn main() {
    let router = DefaultRouter::new()
        .register(index);

    ServerRouter::bind("0.0.0.0:8000")
        .serve(router)
        .await
        .unwrap();
}
```

## 3. Run

```bash
cargo run
```

See [Getting Started](/quickstart/getting-started/) for a complete guide, or explore the [Tutorials](/quickstart/tutorials/) to learn by building.
