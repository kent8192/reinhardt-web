# Reinhardt Basis Tutorial Example - Polling Application

This example demonstrates the concepts covered in the [Reinhardt Basis Tutorial](../../../website/content/quickstart/tutorials/basis/). It implements a complete polling application with two cooperating apps (`polls` and `users`) — server-rendered REST endpoints, typed RPC server functions, an admin panel, and a WASM single-page-application client all in a single crate.

## What This Example Covers

This example corresponds to the basis tutorial parts 1-7:

- **Part 1: Project Setup** - Project structure, development server, first views
- **Part 2: Models and Database** - Database configuration, ORM models, admin panel
- **Part 3: Views and URLs** - View functions, URL routing, templates
- **Part 4: Forms and Generic Views** - HTML forms, form processing, generic views
- **Part 5: Testing** - Automated testing, model and view tests
- **Part 6: Static Files** - CSS, images, static file management
- **Part 7: Admin Customization** - Admin interface customization

## Features

### Models

- **`Question`** (`src/apps/polls/models.rs`) — poll question with `question_text`, `pub_date` (`auto_now_add`), and an `author` foreign key to `User` (`#[rel(foreign_key, related_name = "questions")]`).
- **`Choice`** (`src/apps/polls/models.rs`) — answer option with a `question` foreign key (`#[rel(foreign_key, related_name = "choices")]`), `choice_text`, and a `votes` counter.
- **`User`** (`src/apps/users/models.rs`) — minimal authentication model defined with `#[user(hasher = Argon2Hasher, username_field = "username", manager = false)]` on top of `#[model(app_label = "users", table_name = "users")]`. `manager = false` opts out of the auto-generated user manager so the example can register a project-local `UserManager` via `#[injectable_factory(scope = "transient")]`.

### Views

The example exposes the same business logic through two layers:

- **Server-rendered REST endpoints** in `src/apps/polls/views.rs` — `#[get]` / `#[post]` handlers that take `Path<i64>` / `Json<VoteRequest>` and return JSON. Mounted by `apps/polls/urls/server_urls.rs::server_url_patterns()`.
- **Typed RPC server functions** in `src/apps/<app>/server_fn.rs` — `#[server_fn]` functions (`get_questions`, `get_question_detail`, `vote`, `create_question`, …, plus `login` / `logout` / `register` / `current_user` for the `users` app). The macro generates a typed client stub for WASM and a server-side handler for native; dependencies are resolved positionally with `#[inject]` (`DatabaseConnection`, `SessionData`, …).

### URL Structure

App routers are auto-mounted by `#[url_patterns(InstalledApp::<app>, mode = server | client)]`, so the project-level `src/config/urls.rs` does not need explicit `.mount("/polls/", …)` calls.

| Path | Layer | Where it is defined |
|------|-------|---------------------|
| `/polls/` | Server REST list + SPA home (`polls:index`) | `apps/polls/views.rs::index` + `apps/polls/urls/client_router.rs` |
| `/polls/{question_id}/` | Server REST detail + SPA route (`polls:detail`) | `views::detail` + `client_router.rs` |
| `/polls/{question_id}/results/` | Server REST results + SPA route (`polls:results`) | `views::results` + `client_router.rs` |
| `/polls/{question_id}/vote/` (POST) | Server REST vote submission | `views::vote` |
| `/polls/new/`, `/polls/{question_id}/edit/`, `/polls/{question_id}/delete/` | Author-only CUD client routes backed by `#[server_fn]`s | `apps/polls/urls/client_router.rs` + `apps/polls/server_fn.rs` |
| `/polls/{question_id}/choices/new/`, `…/edit/`, `…/delete/` | Choice CUD client routes backed by `#[server_fn]`s | `client_router.rs` + `server_fn.rs` |
| `/users/login/`, `/users/logout/`, `/users/signup/` | Auth client routes; server functions registered in `src/config/urls.rs` | `apps/users/urls/client_router.rs` |
| `/admin/` | Auto-generated admin panel | `src/config/admin.rs` mounted in `src/config/urls.rs` |

## Setup

### Prerequisites

- Rust 1.94 or later (2024 edition, matches the workspace MSRV)
- `cargo-make` (`cargo install cargo-make`)
- `wasm-pack` for the WASM client build
- Docker (optional, for TestContainers in integration tests)

### Installation

```bash
# From the project root
cd examples/examples-tutorial-basis

# Build the project
cargo build
```

## Usage

### Run the Development Server

The recommended workflow is driven entirely through `cargo make` (defined in `Makefile.toml`):

```bash
# Build the WASM bundle, collect static files, then start the dev server.
# Use this when iterating on either the server or the WASM client.
cargo make dev

# Server only (no WASM rebuild). Runs migrations then `manage runserver --with-pages`.
cargo make runserver

# Optimised path: release-mode WASM build + collectstatic + dev server.
cargo make wasm-build-release
cargo make dev-release
```

The server listens at `http://127.0.0.1:8000/`.

### Available Endpoints

```bash
# Server-rendered REST endpoints (src/apps/polls/views.rs)
curl http://127.0.0.1:8000/polls/
curl http://127.0.0.1:8000/polls/1/
curl http://127.0.0.1:8000/polls/1/results/

# Submit a vote (POST). The CSRF token is enforced by middleware; for an
# interactive flow, drive the same endpoints through the WASM SPA at /polls/.
curl -X POST http://127.0.0.1:8000/polls/1/vote/ \
  -H "Content-Type: application/json" \
  -d '{"question_id": 1, "choice_id": 1}'
```

## Project Structure

```
examples-tutorial-basis/
├── Cargo.toml                      # cdylib + rlib; reinhardt facade with
│                                   # pages + client-router + auth-session
├── Makefile.toml                   # cargo make tasks (runserver, dev,
│                                   # wasm-build-dev, wasm-build-release, …)
├── build.rs                        # cfg_aliases: declares `wasm`, `native`,
│                                   # `with_reinhardt` cfgs
├── index.html                      # SPA shell with #root + UnoCSS runtime
├── README.md                       # This file
├── favicon.png
├── settings/
│   ├── base.toml                   # [core] + [core.security] + [database] (postgres)
│   │                               # + top-level redis_url
│   ├── ci.toml                     # CI overlay
│   └── local.example.toml          # Template for local profile overlay
├── migrations/                     # Generated by `cargo make makemigrations`
├── scripts/                        # Helper shell scripts for wasm-build-*,
│                                   # run-dev-server, clean-cache,
│                                   # infra_up.sh / infra_down.sh
│                                   # (disposable PostgreSQL + Redis via
│                                   # docker run --rm), parse_local_toml.py
├── static/                         # Project-level static assets
├── src/
│   ├── lib.rs                      # `#[cfg(native)]` / `#[cfg(wasm)]` gating;
│   │                               # `mod apps; mod config; mod shared; mod client;`
│   ├── apps.rs                     # `pub mod polls; pub mod users;`
│   ├── config.rs                   # `#[cfg(native)]` admin/settings/wasm;
│   │                               # `apps` & `urls` compile on both targets
│   ├── shared.rs                   # `#[cfg(native)] mod forms;` + `mod types;`
│   ├── client.rs                   # `pub mod lib; pages; components; links;`
│   │                               # (WASM entry point family)
│   ├── bin/
│   │   └── manage.rs               # Native CLI; `required-features = ["with-reinhardt"]`
│   ├── config/
│   │   ├── settings.rs             # `#[settings(core: CoreSettings)] ProjectSettings`
│   │   │                           # + `SettingsBuilder` composition
│   │   ├── apps.rs                 # `installed_apps! { polls: "polls", users: "users" }`
│   │   ├── urls.rs                 # `#[routes(standalone)] routes()` — registers every
│   │   │                           # server_fn + mounts /admin/ + applies middleware
│   │   ├── wasm.rs                 # `AppStaticFilesConfig` for dist-wasm via
│   │   │                           # `inventory::submit!`
│   │   └── admin.rs                # `AdminSite::new("Polls Tutorial Admin")` +
│   │                               # registers QuestionAdmin / ChoiceAdmin
│   ├── shared/
│   │   ├── types.rs                # DTOs that cross WASM ↔ native: UserInfo,
│   │   │                           # QuestionInfo, ChoiceInfo, VoteRequest,
│   │   │                           # LoginRequest, RegisterRequest
│   │   └── forms.rs                # `#[cfg(native)]` only — `create_vote_form()`
│   │                               # using `reinhardt::forms::{CharField, Form}`
│   ├── apps/
│   │   ├── polls.rs                # `#[cfg(native)]` admin/models/serializers/views;
│   │   │                           # server_fn + urls compile on both targets
│   │   ├── polls/
│   │   │   ├── models.rs           # Question (author FK → User), Choice (question FK)
│   │   │   ├── server_fn.rs        # `#[server_fn]` RPC handlers + `require_user` helper
│   │   │   ├── views.rs            # `#[get]/#[post]` REST handlers for index / detail
│   │   │   │                       # / results / vote
│   │   │   ├── urls.rs             # Declares `server_urls` (cfg native) +
│   │   │   │                       # `client_router` (cfg wasm) submodules
│   │   │   ├── urls/
│   │   │   │   ├── server_urls.rs  # `#[url_patterns(InstalledApp::polls, mode = server)]`
│   │   │   │   └── client_router.rs # `#[url_patterns(InstalledApp::polls, mode = client)]`
│   │   │   ├── admin.rs            # `#[admin(model, for = Question/Choice, …)]`
│   │   │   └── serializers.rs      # QuestionSerializer / QuestionResponse /
│   │   │                           # ChoiceSerializer / ChoiceResponse
│   │   ├── users.rs                # `pub mod models (cfg native); server_fn; urls`
│   │   └── users/
│   │       ├── models.rs           # `#[user(…)]` User + `#[injectable_factory]`
│   │       │                       # UserManager
│   │       ├── server_fn.rs        # `login`, `register`, `logout`, `current_user`
│   │       ├── urls.rs             # Declares `server_urls` + `client_router`
│   │       └── urls/
│   │           ├── server_urls.rs  # Empty `ServerRouter` (auth is RPC-only)
│   │           └── client_router.rs # `users:login`, `users:logout`, `users:signup`
│   └── client/
│       ├── lib.rs                  # `#[wasm_bindgen(start)] main()` —
│       │                           # `ClientLauncher::new("#root")…launch()`
│       ├── pages.rs                # Page factory fns; wrap bodies in `with_nav(…)`
│       ├── components.rs           # `pub mod nav; polls; users;`
│       ├── components/
│       │   ├── nav.rs              # Nav bar with login/logout state
│       │   ├── polls.rs            # `page! { … }` components for the polls app
│       │   └── users.rs            # `page! { … }` components for the users app
│       └── links.rs                # `ResolvedUrls` helpers: `polls_index()`,
│                                   # `poll_detail(id)`, `question_edit(id)`, …
└── tests/
    ├── integration.rs              # Native; `required-features = ["with-reinhardt"]`
    └── wasm/
        └── polls_mock_test.rs      # WASM; `#![cfg(wasm)]`, `required-features = ["msw"]`
```

## Learning Path

This example is designed to be studied alongside the basis tutorial:

1. **Start with the tutorial**: Read [Part 1](../../../website/content/quickstart/tutorials/basis/1-project-setup.md)
2. **Examine the code**: Look at how concepts are implemented in this example
3. **Run the tests**: `cargo make test` to see the functionality in action
4. **Experiment**: Modify the code and see what happens

## Key Concepts Demonstrated

### 1. Models (`src/apps/polls/models.rs`)

Both `Question` and `Choice` are defined with `#[model(app_label = "polls", table_name = "…")]`. Field metadata is attached with `#[field(…)]` and foreign keys with `#[rel(foreign_key, related_name = "…")]`. `related_name` is **required** on `#[rel(foreign_key)]`.

```rust
use chrono::{DateTime, Utc};
use reinhardt::db::associations::ForeignKeyField;
use reinhardt::prelude::*;
use serde::{Deserialize, Serialize};

use crate::apps::users::models::User;

#[model(app_label = "polls", table_name = "questions")]
#[derive(Serialize, Deserialize)]
pub struct Question {
    #[field(primary_key = true)]
    pub id: i64,

    #[field(max_length = 200)]
    pub question_text: String,

    #[field(auto_now_add = true)]
    pub pub_date: DateTime<Utc>,

    // Author of the question. Only the author can edit or delete it
    // (enforced server-side in `crate::apps::polls::server_fn`).
    #[rel(foreign_key, related_name = "questions")]
    pub author: ForeignKeyField<User>,
}

#[model(app_label = "polls", table_name = "choices")]
#[derive(Serialize, Deserialize)]
pub struct Choice {
    #[field(primary_key = true)]
    pub id: i64,

    // ⚠️ IMPORTANT: related_name is REQUIRED for #[rel(foreign_key)]
    #[rel(foreign_key, related_name = "choices")]
    pub question: ForeignKeyField<Question>,

    #[field(max_length = 200)]
    pub choice_text: String,

    #[field(default = 0)]
    pub votes: i32,
}
```

Instances are constructed via the typestate `build()` API:

```rust
let question = Question::build()
    .question_text("What's your favorite color?")
    .author(1_i64)  // FK accepts `IntoPrimaryKey` — either `&User` or raw PK.
    .finish();
```

### 2. Server Functions (`src/apps/polls/server_fn.rs`)

`#[server_fn]` produces a typed client stub for WASM and a native handler in one go. Parameters annotated with `#[inject]` are resolved from the DI container; for `get_questions`, that means a `DatabaseConnection`. The body compiles on `#[cfg(native)]` only — the WASM build sees the generated stub.

```rust
/// Get all questions (latest 5)
///
/// Returns the 5 most recent poll questions.
#[server_fn]
pub async fn get_questions(
    #[inject] _db: reinhardt::DatabaseConnection,
) -> std::result::Result<Vec<QuestionInfo>, ServerFnError> {
    use crate::apps::polls::models::Question;
    use reinhardt::Model;

    let manager = Question::objects();
    let questions = manager
        .all()
        .all()
        .await
        .map_err(|e| ServerFnError::application(e.to_string()))?;

    // Take latest 5 questions
    let latest: Vec<QuestionInfo> = questions
        .into_iter()
        .take(5)
        .map(QuestionInfo::from)
        .collect();

    Ok(latest)
}
```

Mutating server functions also accept `#[inject] session: SessionData` and use the local `require_user(&session)` helper to gate on `USER_ID_SESSION_KEY`.

### 3. URL Routing — per-app split

Every app owns a `urls.rs` that declares two cfg-gated submodules: a `ServerRouter` for native and a `ClientRouter` for WASM. The `#[url_patterns(InstalledApp::<app>, mode = …)]` attribute auto-registers each router with the framework via `inventory` and derives the URL prefix from the `InstalledApp` variant — so `src/config/urls.rs` only needs to register server functions and apply middleware.

```rust
// src/apps/polls/urls.rs
#[cfg(native)]
pub mod server_urls;

#[cfg(wasm)]
pub mod client_router;
```

```rust
// src/apps/polls/urls/server_urls.rs
use reinhardt::ServerRouter;
use reinhardt::url_patterns;

use crate::apps::polls::views;
use crate::config::apps::InstalledApp;

#[url_patterns(InstalledApp::polls, mode = server)]
pub fn server_url_patterns() -> ServerRouter {
    ServerRouter::new()
        .endpoint(views::index)
        .endpoint(views::detail)
        .endpoint(views::results)
        .endpoint(views::vote)
}
```

```rust
// src/apps/polls/urls/client_router.rs
#[url_patterns(InstalledApp::polls, mode = client)]
pub fn client_url_patterns() -> ClientRouter {
    ClientRouter::new()
        .named_route("index", "/", index_page)
        .named_route("question_new", "/polls/new/", question_new_page)
        .named_route_path(
            "detail",
            "/polls/{question_id}/",
            |ClientPath(question_id): ClientPath<i64>| polls_detail_page(question_id),
        )
        .named_route_path(
            "results",
            "/polls/{question_id}/results/",
            |ClientPath(question_id): ClientPath<i64>| polls_results_page(question_id),
        )
        // … question/choice CUD routes …
        .not_found(|| error_page("Page not found"))
}
```

The `mode = client` macro namespaces every `named_route` under `polls:`, so SPA components resolve URLs as `polls:detail`, `polls:results`, etc. through `ResolvedUrls` (see `src/client/links.rs`).

### 4. Configuration (`src/config/*`)

- **`settings.rs`** — `#[settings(core: CoreSettings)] ProjectSettings` composed via `SettingsBuilder` from `DefaultSource`, `LowPriorityEnvSource`, and `TomlFileSource` overlays (`base.toml` + a profile-specific file).
- **`apps.rs`** — `installed_apps! { polls: "polls", users: "users" }` generates the `InstalledApp` enum consumed by every `#[url_patterns(InstalledApp::<app>, …)]`.
- **`urls.rs`** — `#[routes(standalone)] routes()` registers all server functions, mounts `/admin/` and `/static/admin/`, and applies `SessionMiddleware` with a two-week TTL.
- **`wasm.rs`** — submits an `AppStaticFilesConfig` pointing at `dist-wasm/` so `cargo make collectstatic` picks up the WASM bundle.
- **`admin.rs`** — `AdminSite::new("Polls Tutorial Admin")` configured with `QuestionAdmin` and `ChoiceAdmin`.

## Testing

Tests are split between native (`tests/integration.rs`) and WASM (`tests/wasm/polls_mock_test.rs`). The native integration target is gated by `required-features = ["with-reinhardt"]` and the WASM mock target by `required-features = ["msw"]` — Cargo cannot auto-discover test files under `tests/<subdir>/`, hence the explicit `[[test]]` declarations in `Cargo.toml`. Run them through `cargo make`:

```bash
# All tests (native lib + integration; forwards --all-features)
cargo make test

# Integration tests only
cargo make test-integration

# Unit tests only (library)
cargo make test-unit

# WASM tests in headless Chrome (msw-mocked server functions)
cargo make wasm-test
```

The `msw` feature is forwarded to the `reinhardt` facade so `#[server_fn]` generates the `MockableServerFn` markers that `tests/wasm/polls_mock_test.rs` consumes; until that facade flag ships upstream, the WASM mock target skips cleanly via its `required-features`.

## Next Steps

After understanding this example:

1. **Extend the models**: Add user authentication, comments, or tags
2. **Add database integration**: Implement actual database storage
3. **Create templates**: Add HTML templates for views
4. **Customize admin**: Create custom admin interface
5. **Add static files**: Include CSS and JavaScript

## Related Documentation

- [Basis Tutorial](../../../website/content/quickstart/tutorials/basis/) - Step-by-step guide
- [API Documentation](https://docs.rs/reinhardt-web) - Complete API reference

## License

This example is part of the Reinhardt project and is licensed under the BSD 3-Clause License.
