+++
title = "Basics Tutorials"
description = "Core concepts and fundamentals of Reinhardt."
sort_by = "weight"
weight = 10

[extra]
sidebar_weight = 10
+++


# Reinhardt Basis Tutorial

Learn the fundamentals of the Reinhardt framework by building a real-world polling application on the **reinhardt-pages template** — a WASM client, server functions, shared DTOs, an admin panel, and session-cookie authentication.

## Overview

This tutorial series walks you through building a fully functional polling application from scratch. The reference implementation lives under [`examples/examples-tutorial-basis`](https://github.com/kent8192/reinhardt-web/tree/main/examples/examples-tutorial-basis); following the chapters in order will produce a project whose module layout is logically equivalent to it.

Reinhardt's basis tutorial is intentionally different from a classic server-rendered Django-style stack: the UI is a Rust-compiled WebAssembly (WASM) client, the backend exposes typed **server functions** via `#[server_fn]`, and client and server share the same DTO types. You will see each of these layers introduced explicitly below.

## Who This Tutorial Is For

- Developers new to Reinhardt who want to learn the framework from the ground up
- Django developers transitioning to Rust who want to understand Reinhardt's pages architecture
- Anyone building full-stack web applications where the browser runs Rust (WASM) and talks to a server function backend

## Prerequisites

- Basic knowledge of Rust programming
- Familiarity with Cargo and `cargo make`
- Understanding of HTTP concepts and web development
- A code editor or IDE

## What You'll Build

A polling application where end users can:

- View the latest polls on a WASM-rendered index page
- Sign up, log in, and create their own polls
- Open a poll, vote on a choice, and see the result update reactively
- See aggregated voting results on a results page
- Edit or delete their own polls (ownership-checked server-side)

Administrators can:

- Create and manage polls via the Reinhardt admin at `/admin/` (registered as a server-rendered admin panel with WASM admin assets)
- Add, edit, and remove choices for each poll

## The Pages Template at a Glance

Every chapter maps onto this layout, which matches the completed example under `examples/examples-tutorial-basis/`:

```text
# Project tree: _index
examples-tutorial-basis/
├── Cargo.toml                 # cdylib + rlib; reinhardt with "pages" + "client-router" + "auth-session" features
├── Makefile.toml              # cargo make tasks: runserver, migrate, dev, wasm-build-dev, collectstatic, test, …
├── build.rs                   # cfg_aliases: `native` vs `wasm`
├── index.html                 # SPA shell with #root mount point and UnoCSS runtime
├── settings/                  # TOML settings (base.toml, ci.toml, local.toml)
├── src/
│   ├── lib.rs                 # Entry: declares apps / config / shared / client modules with cfg gates
│   ├── apps.rs                # pub mod polls; pub mod users;
│   ├── config.rs              # pub mod admin/settings/wasm (cfg native); apps / urls compile both targets
│   ├── shared.rs              # pub mod forms (cfg native); pub mod types (both targets)
│   ├── client.rs              # pub mod lib / pages / components (wasm-only via crate root)
│   ├── bin/
│   │   └── manage.rs          # CLI binary (manage.py equivalent), required-features = ["with-reinhardt"]
│   ├── config/
│   │   ├── settings.rs        # #[settings(core: CoreSettings | contacts: ContactSettings)] ProjectSettings + SettingsBuilder + profile loading
│   │   ├── apps.rs            # installed_apps! entries added by startapp
│   │   ├── urls.rs            # #[routes] routes() -> UnifiedRouter (app server-router mounts, admin mount, session middleware, client-router aggregation)
│   │   ├── wasm.rs            # AppStaticFilesConfig for dist-wasm/, registered via inventory::submit!
│   │   └── admin.rs           # configure_admin() -> AdminSite + register Question/Choice admins
│   ├── shared/
│   │   ├── types.rs           # Re-exported generated *Info companions; VoteRequest, LoginRequest, RegisterRequest
│   │   └── forms.rs           # #[cfg(server)] create_vote_form() — form! + use_form metadata/runtime contract
│   ├── apps/
│   │   ├── polls/
│   │   │   ├── models.rs      # #[model] Question (author FK to User), Choice (question FK)
│   │   │   ├── client.rs      # wasm-only app UI module
│   │   │   ├── server_fn.rs   # #[server_fn] get_questions / get_question_detail / vote / submit_vote / create_question / …
│   │   │   ├── urls.rs        # exposes app-level server/client router functions
│   │   │   ├── urls/
│   │   │   │   ├── server_urls.rs   # ServerRouter with polls server_fn marker registrations
│   │   │   │   └── client_router.rs # ClientRouter (.route / .route_path + reverse helper)
│   │   │   ├── admin.rs       # #[admin(model, for = Question, …)] QuestionAdmin / ChoiceAdmin
│   │   │   └── serializers.rs # QuestionSerializer / ChoiceSerializer with #[validate(...)]
│   │   └── users/
│   │       ├── models.rs      # #[user(...)] + #[model] User + project-local AuthUserManager (#[injectable_factory])
│   │       ├── server_fn.rs   # #[server_fn] login / register / logout / current_user (session cookie based)
│   │       └── urls/          # server_urls.rs (auth server_fn registrations) + client_router.rs (login / logout / signup pages)
│   └── client/                # WASM-only UI layer (declared in crate root via `pub mod client;` under cfg)
│       ├── lib.rs             # #[wasm_bindgen(start)] main(); ClientLauncher::new("#root").register_routes_from_inventory().launch()
│       ├── pages.rs           # Page factory functions; wraps body components in with_nav(...)
│       ├── components.rs      # pub mod nav;
│       └── components/        # nav.rs — shared navigation shell
└── tests/
    ├── integration.rs          # native; required-features = ["with-reinhardt"]; rstest + serial_test + sqlx + tempfile
    └── wasm/polls_mock_test.rs # WASM-only; required-features = ["msw"]; wasm-bindgen-test
```

Three rules keep this structure predictable:

1. **Server vs client** — `#[cfg(server)]` code runs on the server (server function bodies, forms, admin, database-only helpers). `#[cfg(client)]` code runs in the browser (`src/client/` plus each app's `client` module). Model modules compile on both targets so the `#[model]` macro can expose generated `QuestionInfo`, `ChoiceInfo`, and `UserInfo` companions to WASM; the ORM implementation it generates remains server-only.
2. **Server functions are the bridge, and they live per-app** — anything the WASM client needs from the database goes through a `#[server_fn]` defined in `src/apps/<app>/server_fn.rs` (so it sits alongside that app's models, client UI, and admin), and the result is returned as a generated `*Info` companion or request DTO from `src/shared/types.rs`. There is no top-level `src/server_fn/` directory.
3. **Routing is also per-app, with a `urls/` directory module** — each app exposes app-level `server_url_patterns()` and `client_url_patterns()` functions from `src/apps/<app>/urls.rs`. Server functions are registered in app-local `server_urls.rs` files, and `src/config/urls.rs` aggregates the app-level router functions rather than importing individual handlers.

## Tutorial Structure

### [Part 1: Project Setup](1-project-setup/)

- Install `reinhardt-admin-cli`, generate a project from the **`pages`** template, and create `polls` / `users` apps with `startapp`
- Walk the `src/{lib,apps,config,shared,client,bin}` layout the template emits
- Configure `settings/base.toml` with `[core]`, `[core.databases.default]`, and `[contacts]`, then load it through the `ProjectSettings` + `SettingsBuilder` pipeline (note: `TomlFileSource` interpolation is enabled by default)
- Run the dev server with `cargo make runserver` and the full WASM workflow with `cargo make dev`

### [Part 2: Models and Database](2-models-and-database/)

- Define `Question` and `Choice` under `src/apps/polls/models.rs` with `#[model(app_label = "polls", table_name = "...")]`, using `#[field(...)]` and `#[rel(foreign_key, related_name = "...")]` to wire `Question.author -> User` and `Choice.question -> Question`
- Introduce the `users` app and the `User` model defined with `#[user(hasher = Argon2Hasher, username_field = "username", manager = false)] + #[model(...)]`, plus a project-local `AuthUserManager` registered via `#[injectable_factory(scope = "transient")]`
- Use the `installed_apps! { polls: "polls", users: "users" }` entries that `startapp` added in Part 1
- Generate and apply migrations with `cargo make makemigrations` and `cargo make migrate`

### [Part 3: Server Functions and URLs](3-views-and-urls/)

- Write **server functions** under `src/apps/polls/server_fn.rs` and `src/apps/users/server_fn.rs` — this is the "views" layer for the WASM client
- Split routing into `src/apps/<app>/urls/server_urls.rs` (`ServerRouter`) and `src/apps/<app>/urls/client_router.rs` (`ClientRouter`)
- Register each app's server functions in its own `urls/server_urls.rs` with `ServerFnRouterExt::server_fn(...)`
- Expose app-level router functions from `src/apps/<app>/urls.rs`; `src/config/urls.rs` aggregates those functions rather than importing each server function directly
- Bootstrap the SPA in `src/client/lib.rs` with `ClientLauncher::new("#root").register_routes_from_inventory().launch()`; the `#[routes]` aggregator in `src/config/urls.rs` composes each app's client router via `UnifiedRouter::mount_unified`, which the launcher then collects

### [Part 4: Forms and Generic Views](4-forms-and-generic-views/)

- Define `create_vote_form()` in `src/shared/forms.rs` (server-only, behind `#[cfg(server)]`) from the same `form!` source that `use_form(&form).build()` can exercise in tests
- Let the client-side `form!` macro emit the matching `FormMetadata`; CSRF for WASM submits is supplied by the generated `#[server_fn]` client stub and verified by middleware
- Build the voting UI in `src/apps/polls/client/components.rs` with the **`form!` macro** + `watch { ... }` blocks inside a `page!` component
- Call `submit_vote` (a `#[server_fn]` in `crate::apps::polls::server_fn`) on submit; show server validation errors reactively

### [Part 5: Testing](5-testing/)

- Use `rstest` fixtures + `reinhardt-test` helpers + `sqlx` + `tempfile` (all under `[target.'cfg(not(...))'.dev-dependencies]`) to spin up an isolated SQLite for native integration tests
- Mark the native integration target with `[[test]] name = "integration", required-features = ["with-reinhardt"]`
- Add a WASM-only target at `tests/wasm/polls_mock_test.rs` (`#![cfg(client)]`, `required-features = ["msw"]`) that mocks server function HTTP calls via MSW
- Follow the Arrange-Act-Assert pattern with `// Arrange`, `// Act`, `// Assert` labels

### [Part 6: Static Files](6-static-files/)

- Understand the two static-asset tiers used by the pages template:
  - `dist-wasm/` — output of `cargo make wasm-build-dev` / `wasm-build-release`, registered via `AppStaticFilesConfig` + `inventory::submit!` in `src/config/wasm.rs`
  - `staticfiles/` — final output of `cargo make collectstatic`, served at `/static/`
- Wire it all up through `Makefile.toml` tasks (`runserver`, `dev`, `wasm-build-dev`, `collectstatic`, `dev-release`)

### [Part 7: Admin Customization](7-admin-customization/)

- Register `ModelAdmin` implementations app-side with `#[admin(model, for = ..., ...)]` in `src/apps/polls/admin.rs`
- Compose the project-wide `AdminSite` in `src/config/admin.rs` and mount it at `/admin/` from `src/config/urls.rs` via `admin_routes_with_di`
- Customize list columns, search fields, filters, ordering, and per-page limits

## Recommended Learning Path

Work through the chapters in order. Each chapter assumes the directory layout produced by the previous one. If you get stuck, compare your tree against [`examples/examples-tutorial-basis/`](https://github.com/kent8192/reinhardt-web/tree/main/examples/examples-tutorial-basis) — the reference source is the authoritative answer key.

## Getting Help

- [Reinhardt repository](https://github.com/kent8192/reinhardt-web)
- [Getting Started Guide](/quickstart/getting-started/)
- [Feature Flags Guide](/docs/feature-flags/)
- [GitHub Discussions](https://github.com/kent8192/reinhardt-web/discussions)

## Comparison with REST Tutorial

If you're also interested in building pure JSON APIs, see the [REST Tutorial](../rest/).

- **Basis Tutorial** (this one): full-stack pages template — WASM client + `#[server_fn]` + shared DTOs + admin + session auth.
- **REST Tutorial**: `#[get]` / `#[post]` views, `Serializer`s, and `ViewSet` + `Router` for classic REST endpoints.

The underlying model and database layers are identical, so lessons transfer in both directions.

## Let's Get Started!

Head over to [Part 1: Project Setup](1-project-setup/) to generate your first pages project.
