+++
title = "Quickstart"
description = "Get started with Reinhardt in 5 minutes."
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
use reinhardt::prelude::*;

#[tokio::main]
async fn main() {
    let app = Reinhardt::new();
    app.run("0.0.0.0:8000").await.unwrap();
}
```

## 3. Run

```bash
cargo run
```

See [Getting Started](/docs/getting-started/) for a complete guide.
