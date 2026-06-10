+++
title = "Quickstart"
description = "Get started with Reinhardt — quickstart guide, full setup, and tutorials."
sort_by = "weight"
weight = 10
+++

# Quickstart

Get up and running with Reinhardt in 5 minutes.

## 1. Install Reinhardt Admin CLI

Install the latest stable CLI, or keep `--version` as a reproducibility pin.
The literal below is auto-bumped by release-plz on each release.

<!-- reinhardt-version-sync -->
```bash
cargo install reinhardt-admin-cli --version "0.1.4"
```

## 2. Create your project

```bash
reinhardt-admin startproject my-api --with-rest
cd my-api
```

## 3. Create your first app

```bash
reinhardt-admin startapp hello --template rest
```

Edit `hello/views.rs`:

```rust
use reinhardt::prelude::*;
use reinhardt::get;

#[get("/hello", name = "hello_world")]
pub async fn hello_world() -> ViewResult<Response> {
    Response::ok().with_body("Hello, Reinhardt!")
}
```

## 4. Run

```bash
cargo make runserver
```

Visit `http://127.0.0.1:8000/hello` in your browser.

See [Getting Started](/quickstart/getting-started/) for a complete guide, or explore the [Tutorials](/quickstart/tutorials/) to learn by building.
