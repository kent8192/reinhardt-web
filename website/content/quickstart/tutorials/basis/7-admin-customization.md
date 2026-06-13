+++
title = "Part 7: Admin Customization"
weight = 70

[extra]
sidebar_weight = 70
+++

# Part 7: Admin Customization

In this chapter we wire the auto-generated Reinhardt admin into the polls project. The completed example lives in [`examples/examples-tutorial-basis/`](https://github.com/kent8192/reinhardt-web/tree/main/examples/examples-tutorial-basis); by the end of this chapter your tree will match its admin-related files exactly.

The admin is the last piece of the pages template scaffold introduced in [Part 1](../1-project-setup/). It does **not** live in `src/client/`, it is **not** a WASM component, and it does **not** rely on any of the typed `#[server_fn]` machinery you wrote in [Part 3](../3-views-and-urls/). It is a self-contained, server-rendered admin panel — its own `ServerRouter` mounted at `/admin/`, with its own static assets at `/static/admin/`. Both halves of the wiring are short, and both are mechanical once you understand the split.

## How the Admin Is Split

The Reinhardt admin is configured in two files that already exist in the generated project:

| File | Responsibility |
|------|----------------|
| `src/apps/<app>/admin.rs` | **App-local registration.** Declare one `#[admin(model, for = …, …)]` struct per model the admin should manage. This is the per-app configuration: list columns, search fields, ordering, filters, per-page limits. |
| `src/config/admin.rs` | **Project-wide `AdminSite`.** Instantiate `AdminSite::new(...)`, set site-wide metadata (`site_title`, `site_header`, `list_per_page`), and register each app's admin types with the site. |

`src/config/urls.rs` then mounts the site at `/admin/` and serves its static assets at `/static/admin/` — gated on `#[cfg(server)]` because the admin is server-only.

This split mirrors the way the rest of the project is organized: each app owns its own models, server functions, URLs, and admin metadata, and `src/config/` aggregates them into a single project router. There is no global admin module that owns every model in the project; `configure_admin()` simply imports the admin structs from each app and registers them by name.

The admin module on `apps/polls.rs` is `#[cfg(server)]`-gated, exactly like the other server-only modules (`admin`, `models`, `serializers`) — only `server_fn` and `urls` compile on both targets, while the app-local UI module is `#[cfg(client)]`. The relevant module declarations look like this:

```rust
// File: src/apps/polls.rs
#[cfg(server)]
use reinhardt::app_config;

#[cfg(server)]
pub mod admin;
#[cfg(client)]
pub mod client;
#[cfg(server)]
pub mod models;
#[cfg(server)]
pub mod serializers;
pub mod server_fn;
pub mod urls;

#[cfg(server)]
#[app_config(name = "polls", label = "polls")]
pub struct PollsConfig;
```

That `#[app_config(name = "polls", label = "polls")]` annotation on `PollsConfig` is what registers the polls app with Reinhardt's app config registry so the admin can discover its models. The `users` app does not have a `PollsConfig` equivalent — and that is intentional: the tutorial never registers `User` with the admin, so there is no `users/admin.rs` and no `#[app_config]` on `apps/users.rs`. If you later add a `users/admin.rs` with a `UserAdmin`, you also add `#[app_config(name = "users", label = "users")]` to `apps/users.rs`.

## App-Local Registration: `src/apps/polls/admin.rs`

Open `src/apps/polls/admin.rs`. The file is short — it imports the two polls models and declares one `#[admin]` struct per model:

```rust
// File: src/apps/polls/admin.rs
//! Admin configuration for the polls app.
//!
//! Demonstrates the `#[admin(model, ...)]` macro by registering the two
//! polls models (Question, Choice) so they appear in the auto-generated
//! `/admin/` interface.

use crate::apps::polls::models::{Choice, Question};
use reinhardt::admin;

/// Admin configuration for the Question model.
///
/// Lists id / question_text / pub_date / author_id columns, supports
/// search over the question text, and orders newest-first by default.
#[admin(model,
    for = Question,
    name = "Question",
    list_display = [id, question_text, pub_date, author_id],
    fields = [question_text, author_id],
    list_filter = [pub_date],
    search_fields = [question_text],
    ordering = [(pub_date, desc)],
    readonly_fields = [id, pub_date],
    list_per_page = 25,
    permissions = allow_all,
)]
pub struct QuestionAdmin;

/// Admin configuration for the Choice model.
///
/// Shows the foreign-key `question_id` alongside the choice text and
/// vote count, allowing operators to inspect and adjust vote totals
/// directly when seeding tutorial data.
#[admin(model,
    for = Choice,
    name = "Choice",
    list_display = [id, question_id, choice_text, votes],
    fields = [question_id, choice_text, votes],
    list_filter = [question_id],
    search_fields = [choice_text],
    ordering = [(id, asc)],
    readonly_fields = [id],
    list_per_page = 50,
    permissions = allow_all,
)]
pub struct ChoiceAdmin;
```

`#[admin(model, for = …, …)]` is an attribute macro that generates a `ModelAdmin` implementation on the empty marker struct (`QuestionAdmin`, `ChoiceAdmin`). The struct itself carries no data — it exists only so the macro has a name to hang the trait implementation on, and so `configure_admin()` has something to pass to `site.register(...)`.

### What Each Attribute Does

Each key inside `#[admin(...)]` is a field of the generated `ModelAdmin` impl. The values reference fields on the target model — so the field names you list have to match the actual `#[field(...)]` columns on `Question` and `Choice` from `src/apps/polls/models.rs`:

```rust
// File: src/apps/polls/models.rs (recap from Part 2)
#[model(app_label = "polls", table_name = "questions")]
#[derive(Serialize, Deserialize)]
pub struct Question {
    #[field(primary_key = true)]
    pub id: i64,
    #[field(max_length = 200)]
    pub question_text: String,
    #[field(auto_now_add = true)]
    pub pub_date: DateTime<Utc>,
    #[rel(foreign_key, related_name = "questions")]
    pub author: ForeignKeyField<User>,
}

#[model(app_label = "polls", table_name = "choices")]
#[derive(Serialize, Deserialize)]
pub struct Choice {
    #[field(primary_key = true)]
    pub id: i64,
    #[rel(foreign_key, related_name = "choices")]
    pub question: ForeignKeyField<Question>,
    #[field(max_length = 200)]
    pub choice_text: String,
    #[field(default = 0)]
    pub votes: i32,
}
```

With that in mind, here is what each `#[admin]` attribute controls:

| Attribute | Purpose |
|-----------|---------|
| `model` | Marks this struct as a model admin (not, for example, an inline admin). Always required. |
| `for = Question` | The target model. The macro looks up the model's field accessors (from `#[model]`) to validate the attribute names below. |
| `name = "Question"` | The human-readable label shown in the admin sidebar and breadcrumbs. |
| `list_display = [id, question_text, pub_date, author_id]` | The columns shown on the change-list page (the table listing all `Question` rows). Notice `author_id` — when a model has a `ForeignKeyField<User>` field named `author`, the underlying column is `author_id`, which is what the admin queries and renders. |
| `fields = [question_text, author_id]` | The fields shown on the change form (the form for editing a single row). `id` and `pub_date` are excluded because they are not user-editable. |
| `list_filter = [pub_date]` | Adds a sidebar filter on the change-list page so you can narrow results by `pub_date` ranges. |
| `search_fields = [question_text]` | The fields included in the search box at the top of the change-list page. A user typing `"color"` runs a `LIKE`-style match against `question_text`. |
| `ordering = [(pub_date, desc)]` | The default sort order applied to the change-list query. `desc` shows newest-first; `asc` shows oldest-first. |
| `readonly_fields = [id, pub_date]` | Fields shown on the change form but not editable. `id` is the auto-generated primary key; `pub_date` is `auto_now_add`, so the database fills it on insert and the admin should not let the user overwrite it. |
| `list_per_page = 25` | Paginates the change-list at 25 rows per page. `ChoiceAdmin` uses `50` to make poll seeding faster when you have many choices per question. |
| `permissions = allow_all` | The permission policy. `allow_all` permits any authenticated request to read, add, change, and delete; it is the dev-time default this tutorial uses so you can exercise the UI immediately. Production projects should replace it with a real `ModelAdminPermissions` implementation that checks user roles (for example, "only the question's author can edit it"). |

The `ChoiceAdmin` block uses the same attributes with slightly different choices: `list_display` includes the foreign-key column `question_id` so you can see which question each choice belongs to; `ordering = [(id, asc)]` shows choices in insertion order, which usually matches the order they appear on the poll detail page.

## Project-Wide Site: `src/config/admin.rs`

`src/apps/polls/admin.rs` declares **what** the admin knows about each model; `src/config/admin.rs` decides **how the site looks** and **which admins are registered**. The whole file is fifteen lines of logic:

```rust
// File: src/config/admin.rs
//! Admin panel configuration for examples-tutorial-basis.
//!
//! Builds an `AdminSite` and registers per-app `ModelAdmin` configurations
//! so the Django-style auto-generated admin UI is reachable at `/admin/`.
//! Mounting and DI wiring happen in `crate::config::urls`.

use crate::apps::polls::admin::{ChoiceAdmin, QuestionAdmin};
use reinhardt::admin::AdminSite;

/// Configure the admin site and register all polls-app model admins.
///
/// The database connection is supplied later via DI (see
/// `admin_routes_with_di` in `crate::config::urls`), so this function
/// only handles registration metadata.
pub fn configure_admin() -> AdminSite {
    let site = AdminSite::new("Polls Tutorial Admin");

    site.configure(|config| {
        config.site_title = "Polls Tutorial - Admin".into();
        config.site_header = "Polls Administration".into();
        config.list_per_page = 25;
    });

    site.register("Question", QuestionAdmin)
        .expect("Failed to register QuestionAdmin");
    site.register("Choice", ChoiceAdmin)
        .expect("Failed to register ChoiceAdmin");

    site
}
```

Three things happen here:

1. **Instantiate the site.** `AdminSite::new("Polls Tutorial Admin")` creates an `AdminSite` whose default branding string is `"Polls Tutorial Admin"`. This is the friendly project name passed to the constructor.
2. **Set site-wide metadata** with `site.configure(|config| { … })`. The closure receives a mutable `AdminSiteConfig` whose fields you can override:
   - `site_title` — the HTML `<title>` of every admin page.
   - `site_header` — the header text displayed at the top of every admin page (the equivalent of the Django admin's `site_header`).
   - `list_per_page` — the **site-wide** default pagination size. Individual `#[admin]` attributes override it per model (recall that `QuestionAdmin` keeps `25` and `ChoiceAdmin` sets `50`).
3. **Register each admin struct by name** with `site.register("Question", QuestionAdmin)`. The first argument is the registration key (the name `#[admin(... name = "Question" ...)]` declared); the second is the marker struct. `register(...)` returns a `Result`, and the example uses `.expect(...)` so a duplicate or invalid registration fails loudly at startup rather than degrading the admin silently at request time.

`configure_admin()` returns the configured `AdminSite`. It deliberately does **not** know anything about HTTP routing, middleware, or the database connection — those are the responsibility of the next step.

## Mounting the Admin: `src/config/urls.rs`

The site is mounted in `src/config/urls.rs`, the same `#[routes]` function you saw in [Part 3](../3-views-and-urls/) that registers every server function, aggregates client routes, and applies the session middleware. The admin mount is a `#[cfg(server)]` block, because the admin is server-only and the SPA never needs to know about it:

```rust
// File: src/config/urls.rs
#[cfg(server)]
use reinhardt::admin::{admin_routes_with_di, admin_static_routes};
// …
#[cfg(server)]
use crate::config::admin::configure_admin;

#[routes]
pub fn routes() -> UnifiedRouter {
    // … server-function registration omitted for clarity …

    // Mount the auto-generated admin panel at /admin/ (server-only).
    // `admin_routes_with_di` returns both the router and a DI registration
    // list that lazily provides `AdminDatabase` to admin handlers from the
    // project's `DatabaseConnection`.
    #[cfg(server)]
    let router = {
        let admin_site = std::sync::Arc::new(configure_admin());
        let (admin_router, admin_di) = admin_routes_with_di(admin_site);
        router
            .mount("/admin/", admin_router)
            .mount("/static/admin/", admin_static_routes())
            .with_di_registrations(admin_di)
    };

    // … session middleware applied next …

    router
}
```

Five moving parts, each doing one job:

1. **`Arc::new(configure_admin())`** — wraps the `AdminSite` in an `Arc` so the admin router can clone a cheap handle to it. The admin runs handlers concurrently, so shared ownership is required.
2. **`admin_routes_with_di(admin_site)`** — destructures into `(admin_router, admin_di)`. The router is a `ServerRouter` containing every admin endpoint (the change-list, change-form, add-form, and delete-form views for each registered model, plus the index page). The `admin_di` value is a **DI registration list** that lazily provides `AdminDatabase` to admin handlers, sourcing it from the project's existing `DatabaseConnection`. This is the bridge between the admin's `AdminDatabase` abstraction and your real database — you do not configure it twice; the admin reuses the same connection your `#[server_fn]` handlers use.
3. **`.mount("/admin/", admin_router)`** — mounts the admin's `ServerRouter` at `/admin/`. Hitting `http://127.0.0.1:8000/admin/` lands on the admin index; `/admin/polls/question/` lists every `Question`; `/admin/polls/question/1/change/` opens the change form for `Question` with PK `1`.
4. **`.mount("/static/admin/", admin_static_routes())`** — serves the admin's CSS and JavaScript bundle at `/static/admin/`. These are static files shipped by the admin crate (not by your project), and they are required for the admin UI to render correctly.
5. **`.with_di_registrations(admin_di)`** — registers the `AdminDatabase` provider returned by `admin_routes_with_di(...)` with the project's DI container. Without this call, the admin's handlers would be unable to resolve `AdminDatabase` and every request would 500. This is the call that makes the admin reuse your project's `DatabaseConnection` instead of opening a parallel connection of its own.

That is the entire mount. No additional routes, no admin-side middleware, no second router instance — just a `ServerRouter` mounted at `/admin/` and its static assets mounted at `/static/admin/`, sitting alongside your public pages in the same `UnifiedRouter`.

## Running the Admin

Migrations are already wired up from [Part 2](../2-models-and-database/), and the session middleware from [Part 3](../3-views-and-urls/) is what gives the admin its login flow. To see the admin running:

```bash
# Terminal: project root
cargo make migrate
cargo make runserver
```

Visit `http://127.0.0.1:8000/admin/`. Because `permissions = allow_all` is set on both `QuestionAdmin` and `ChoiceAdmin`, you can browse the change-list pages, click into individual rows, and edit them without any further setup. The `Question` change-list shows the columns from `list_display` (id, question_text, pub_date, author_id), the search box queries `question_text`, the sidebar filters on `pub_date`, and rows are sorted newest-first.

If you want to seed some data first, you can do it from a `cargo make shell` session or directly from a `#[server_fn]` handler — both paths land in the same `Question::objects().create_with_conn(...)` / `Choice::objects().create_with_conn(...)` API.

## Extending the Admin

Adding a third model to the admin is purely additive — you do not have to touch any existing file beyond the two project-wide registration sites:

1. **Create `src/apps/<new_app>/admin.rs`** with one or more `#[admin(model, for = MyModel, …)]` structs. Pattern-match against `apps/polls/admin.rs`: pick `list_display`, `fields`, `list_filter`, `search_fields`, `ordering`, `readonly_fields`, `list_per_page`, and a `permissions` policy.
2. **Add the module to `src/apps/<new_app>.rs`** with `#[cfg(server)] pub mod admin;`, mirroring the polls layout.
3. **Annotate the new app's config struct** with `#[app_config(name = "<new_app>", label = "<new_app>")]` so the admin can discover it through the app config registry.
4. **Import the new admin types in `src/config/admin.rs`** and call `site.register("MyModel", MyModelAdmin).expect(...)` once per model. No changes to `src/config/urls.rs` are needed — the admin router picks up every registered model automatically.

The `users` app intentionally has no admin in this tutorial: there is no `src/apps/users/admin.rs`, no `UserAdmin`, and no `site.register("User", ...)` call in `configure_admin()`. The `User` model is still fully usable from `#[server_fn]` handlers (login, register, current_user) — it is simply not exposed through the admin panel. If you want to manage users from the admin in your own project, the recipe above applies: write a `UserAdmin` next to the `User` model, register it from `configure_admin()`, and tighten `permissions` to something stricter than `allow_all` (since exposing a writable user list to anyone with `allow_all` is a privilege-escalation footgun).

## What This Chapter Did Not Touch

A few things deliberately stayed out of scope, because the example does not use them:

- **No WASM admin component.** The admin is **server-rendered**. There is no `src/client/components/admin.rs`, no `page!` macro for admin pages, no admin-specific entry in `src/client/lib.rs`. The admin's UI is produced entirely by `admin_routes_with_di(...)` and styled by the assets at `/static/admin/`. The SPA you built in [Part 3](../3-views-and-urls/) and the admin at `/admin/` are two independent UIs that happen to share a single `UnifiedRouter`.
- **No inline editing, custom actions, or fieldsets.** These are useful features that the admin crate supports, but the tutorial example does not exercise them — every admin struct in `src/apps/polls/admin.rs` is a flat list of attributes with no nested configuration. When you outgrow the flat `#[admin]` form, the crate-level documentation on `reinhardt::admin` is the right next stop.
- **No production-grade permissions.** `permissions = allow_all` is the development default, period. In a real deployment the admin lives behind authentication, and the permission policy should encode your application's authorization rules.

## Summary

You wired the Reinhardt admin into the polls project in three short steps:

- Declared `QuestionAdmin` and `ChoiceAdmin` in `src/apps/polls/admin.rs` using the `#[admin(model, for = …, …)]` attribute macro, one struct per model the admin should manage.
- Configured the project-wide `AdminSite` in `src/config/admin.rs` with `AdminSite::new(...)`, `site.configure(|config| { … })` for site-wide metadata, and one `site.register(name, AdminStruct)` call per registered admin.
- Mounted the admin in `src/config/urls.rs` with `admin_routes_with_di(Arc::new(configure_admin()))`, attaching the returned router at `/admin/`, the static assets at `/static/admin/`, and the returned DI registrations via `.with_di_registrations(admin_di)` so admin handlers resolve `AdminDatabase` from the project's existing `DatabaseConnection`.

## Congratulations!

You have finished the Reinhardt basis tutorial. The polls project now matches every file in [`examples/examples-tutorial-basis/`](https://github.com/kent8192/reinhardt-web/tree/main/examples/examples-tutorial-basis): two apps (`polls`, `users`), shared DTOs (`src/shared/types.rs`), a server-only form (`src/shared/forms.rs`), per-app `#[server_fn]` and URL modules, a WASM client under `src/client/`, native and WASM tests under `tests/`, and now a registered admin panel mounted at `/admin/`. The reference source under `examples/examples-tutorial-basis/` is the authoritative answer key — if anything you wrote diverged from it, the example is right.

## Where to Go From Here

- **[REST Tutorial](../rest/)** — build a pure JSON API on the same model layer, using `#[get]` / `#[post]` views, `Serializer`s, and `ViewSet` + `Router`.
- **[Feature Flags Guide](/docs/feature-flags/)** — tune which `reinhardt` features your project pulls in to keep build size and compile time in check.
- **Individual crate docs** on docs.rs — deep dives into `reinhardt-admin`, `reinhardt-pages`, `reinhardt-db`, and the rest of the crate fan-out used here.

Happy hacking with Reinhardt.
