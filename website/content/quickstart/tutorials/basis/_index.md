+++
title = "Basis Tutorials"
description = "Build a full-stack polling application with Reinhardt pages."
sort_by = "weight"
weight = 10

[extra]
sidebar_weight = 10
+++

# Reinhardt Basis Tutorial

Build a polling application on the Reinhardt pages template: a Rust/WASM client, typed server functions, shared DTOs, session-cookie authentication, ownership-checked CRUD, static assets, tests, and the Reinhardt admin.

The reference implementation lives in [`examples/examples-tutorial-basis`](https://github.com/kent8192/reinhardt-web/tree/main/examples/examples-tutorial-basis). Treat that crate as the answer key. The tutorial introduces the same architecture one working slice at a time, so every part ends with something you can run or click.

## Who This Tutorial Is For

- Developers new to Reinhardt who want the full-stack pages workflow.
- Django developers who want to map familiar ideas onto Reinhardt's Rust/WASM architecture.
- Rust developers building browser applications that talk to typed server functions.

## Prerequisites

- Basic Rust and Cargo knowledge.
- `cargo make` installed.
- Docker Desktop running for the disposable PostgreSQL and Redis development containers.
- A browser capable of running WebAssembly.

## What You'll Build

End users can:

- View the latest polls on a WASM-rendered index page.
- Open a poll, vote on a choice, and see the results update.
- Sign up, log in, and log out.
- Create, edit, and delete their own polls and choices.

Administrators can:

- Manage polls and choices at `/admin/`.
- Serve the compiled WASM bundle and project static files from the same application.

## The Pages Template at a Glance

The completed project has this shape:

```text
examples-tutorial-basis/
+-- Cargo.toml
+-- Makefile.toml
+-- build.rs
+-- index.html
+-- settings/
|   +-- base.toml
|   +-- ci.toml
|   +-- local.toml
+-- src/
|   +-- lib.rs
|   +-- apps.rs
|   +-- config.rs
|   +-- shared.rs
|   +-- client.rs
|   +-- bin/
|   |   +-- manage.rs
|   +-- config/
|   |   +-- admin.rs
|   |   +-- apps.rs
|   |   +-- settings.rs
|   |   +-- urls.rs
|   |   +-- wasm.rs
|   +-- shared/
|   |   +-- forms.rs
|   |   +-- types.rs
|   +-- apps/
|   |   +-- polls.rs
|   |   +-- polls/
|   |   |   +-- client.rs
|   |   |   +-- pages.rs
|   |   |   +-- server.rs
|   |   |   +-- server_fn.rs
|   |   |   +-- urls.rs
|   |   |   +-- client/
|   |   |   |   +-- components.rs
|   |   |   +-- server/
|   |   |       +-- admin.rs
|   |   |       +-- models.rs
|   |   |       +-- serializers.rs
|   |   |       +-- urls.rs
|   |   +-- users.rs
|   |   +-- users/
|   |       +-- client.rs
|   |       +-- pages.rs
|   |       +-- server.rs
|   |       +-- server_fn.rs
|   |       +-- urls.rs
|   |       +-- client/
|   |       |   +-- components.rs
|   |       +-- server/
|   |           +-- models.rs
|   |           +-- urls.rs
|   +-- client/
|       +-- components.rs
|       +-- components/
|       |   +-- nav.rs
|       +-- lib.rs
+-- migrations/
|   +-- polls/
|       +-- 0001_initial.rs
|       +-- 0002_question_author.rs
+-- static/
|   +-- css/
|       +-- style.css
+-- tests/
    +-- createsuperuser.rs
    +-- integration.rs
    +-- wasm/
        +-- polls_mock_test.rs
        +-- users_mock_test.rs
```

Three rules keep this structure predictable:

1. **Server and client are separate targets.** `#[cfg(server)]` code runs in the native server binary. `#[cfg(client)]` code runs in the browser as WASM. Database models, admin definitions, serializers, and server URL marker registration live under each app's `server/` module.
2. **Server functions are the bridge.** Anything the WASM client needs from the database goes through a `#[server_fn]` in `src/apps/<app>/server_fn.rs`. The client receives explicit wire DTOs from `src/shared/types.rs`, not server-only model modules.
3. **Routing belongs to each app.** Each app exposes `server_url_patterns()` and `client_url_patterns()` from its target-neutral `src/apps/<app>/urls.rs`, and page entry points live in `src/apps/<app>/pages.rs`. The project-level `src/config/urls.rs` aggregates those app routers, session middleware, admin routes, and static-file routes.

## Tutorial Structure

### [Part 1: Project Setup and SPA Shell](1-project-setup/)

Generate a pages project, inspect the `src/{apps,config,shared,client,bin}` layout, wire settings through `ProjectSettings`, install the WASM tools, and run the empty SPA shell.

### [Part 2: Your First Feature - the Poll Index](2-poll-index/)

Create the `polls` app, define `Question` and `Choice`, generate the first migration without ownership, expose `get_questions`, and render the poll list in the browser.

### [Part 3: Detail Pages and Voting](3-detail-and-voting/)

Add poll detail and results routes, submit votes through `form!` and `#[server_fn]`, and update the page through reactive client state.

### [Part 4: Users and Authentication](4-users-and-authentication/)

Create the `users` app, define the `User` model, register the project-local `AuthUserManager`, add login/register/logout/current-user server functions, and show authentication state in the nav.

### [Part 5: Ownership and Poll CRUD](5-ownership-and-crud/)

Add the `Question.author` foreign key in migration `0002`, then build create/edit/delete flows for polls and choices with server-side ownership checks.

### [Part 6: The Admin and Static Files](6-admin-and-static-files/)

Register poll models with the Reinhardt admin, mount `/admin/`, and learn how `dist-wasm/`, `static/`, and `staticfiles/` fit together.

### [Part 7: Testing](7-testing/)

Run native integration tests with isolated SQLite fixtures, test management commands, and exercise WASM client flows with MSW-backed server-function mocks.

## Recommended Learning Path

Work through the parts in order. Each part assumes the files from the previous one exist and compile. When your project differs from the text, compare it with `examples/examples-tutorial-basis` before inventing a local workaround.

## REST Tutorial Comparison

Use this tutorial when you want the full-stack pages architecture: WASM client, typed server functions, shared DTOs, session auth, admin, and static assets.

Use the [REST tutorial](../rest/) when you want classic JSON endpoints built with `#[get]`, `#[post]`, serializers, and viewsets. The model and migration APIs are shared between both tracks.
