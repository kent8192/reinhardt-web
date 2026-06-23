+++
title = "Part 6: The Admin and Static Files"
description = "Register poll models with the Reinhardt admin and understand how WASM/static assets are collected and served."
weight = 60

[extra]
sidebar_weight = 60
+++

# Part 6: The Admin and Static Files

The application now has user-owned polls. In this part you will add the Reinhardt admin at `/admin/` and wire the static-file pipeline that serves the WASM bundle, CSS, images, and admin assets.

The admin is useful for seeding and inspecting tutorial data. It is not a replacement for the owner-checked server functions from Part 5; it is an operator interface.

## Register Poll Models with the Admin

Create `src/apps/polls/admin.rs`. The example registers `Question` first:

```rust
use crate::apps::polls::models::{Choice, Question};
use reinhardt::admin;

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
```

Then register choices:

```rust
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

`QuestionAdmin` includes `author_id` because Part 5 made question ownership part of the schema.

Part 2 already exposed the polls app from `src/apps.rs`:

```rust
pub mod polls;
```

Now expose the admin module from the polls app parent so the site configuration can import it:

```rust
#[cfg(server)]
pub mod admin;
```

## Configure the Admin Site

Create `src/config/admin.rs` and register the app admins:

```rust
use crate::apps::polls::admin::{ChoiceAdmin, QuestionAdmin};
use crate::config::settings::get_settings;
use reinhardt::HasCoreSettings;
use reinhardt::admin::AdminSite;

pub fn configure_admin() -> AdminSite {
    let mut site = AdminSite::new("Polls Tutorial Admin");
    let settings = get_settings();

    site.configure(|config| {
        config.site_title = "Polls Tutorial - Admin".into();
        config.site_header = "Polls Administration".into();
        config.list_per_page = 25;
    });
    site.set_jwt_secret(settings.core().secret_key.as_bytes());

    site.register("Question", QuestionAdmin)
        .expect("Failed to register QuestionAdmin");
    site.register("Choice", ChoiceAdmin)
        .expect("Failed to register ChoiceAdmin");

    site
}
```

The site configuration is metadata. Database access is supplied later through DI when routes are mounted.

Expose the configuration module from `src/config.rs`:

```rust
#[cfg(server)]
pub mod admin;
```

## Mount Admin Routes

In `src/config/urls.rs`, import the admin helpers and your site configuration:

```rust
#[cfg(server)]
use reinhardt::admin::{admin_routes_with_di, admin_static_routes};

#[cfg(server)]
use crate::config::admin::configure_admin;
```

Mount `/admin/` and `/static/admin/`:

```rust
#[cfg(server)]
let router = {
    let admin_site = std::sync::Arc::new(configure_admin());
    let (admin_router, admin_di) = admin_routes_with_di(admin_site);
    router
        .mount("/admin/", admin_router)
        .mount("/static/admin/", admin_static_routes())
        .with_di_registrations(admin_di)
};
```

`admin_routes_with_di` returns both a router and DI registrations. Keep the `with_di_registrations(admin_di)` call; admin handlers need the project's database connection.

## Register WASM Build Artifacts

The pages runtime needs to find the WASM bundle produced by `wasm-pack`. Register `dist-wasm/` in `src/config/wasm.rs`:

```rust
use reinhardt::reinhardt_apps::AppStaticFilesConfig;

inventory::submit! {
    AppStaticFilesConfig {
        app_label: "examples-tutorial-basis-wasm",
        static_dir: "dist-wasm",
        url_prefix: "",
    }
}
```

This tells `collectstatic` that the generated WASM artifacts are application static files.

## Understand the Static Directories

The example uses several directories with different responsibilities:

```text
static/
+-- css/style.css          # hand-written project CSS
+-- images/poll-icon.svg   # project static asset
dist-wasm/                 # wasm-pack output
dist/                      # runserver static directory for dev
staticfiles/               # collectstatic output
static/admin/              # generated admin assets
```

`dist/`, `dist-wasm/`, `staticfiles/`, and `static/admin/` are generated and gitignored.

The core cargo-make tasks are:

```toml
[tasks.collectstatic]
description = "Collect static files into STATIC_ROOT"
command = "cargo"
args = ["run", "--bin", "manage", "collectstatic"]
```

```toml
[tasks.wasm-bindgen-dev]
description = "Generate WASM bindings using wasm-pack (debug mode)"
dependencies = ["wasm-compile-dev"]
command = "wasm-pack"
args = [
    "build",
    "--target", "web",
    "--out-dir", "dist-wasm",
    "--dev",
    "--no-typescript"
]
```

```toml
[tasks.dev]
description = "Build WASM, apply migrations, and start the pages development server"
dependencies = ["wasm-build-dev", "migrate", "run-dev-server"]
```

## Follow the Dev Build Path

`cargo make wasm-build-dev` runs `wasm-pack`, then calls `scripts/wasm-build-dev.sh`:

```bash
echo "Running collectstatic..."
cargo run --bin manage collectstatic --no-input
mkdir -p dist
find dist-wasm -maxdepth 1 -type f \( -name '*.js' -o -name '*.wasm' -o -name '*.d.ts' \) -exec cp -f {} dist/ \;
echo "WASM build and collectstatic completed"
```

The final `cargo make dev` step starts the server with pages enabled and avoids rebuilding over the just-created bundle:

```bash
cargo run --bin manage -- runserver --with-pages --noreload --no-override-wasm
```

Release-mode local testing uses:

```bash
cargo make dev-release
```

That path runs `wasm-build-release`, `collectstatic`, `migrate`, and the release-mode dev server script.

## Checkpoint

Run the admin-enabled app:

```bash
cargo make dev
```

Open `http://127.0.0.1:8000/admin/`. You should be able to manage `Question` and `Choice` records through the admin UI.

Before continuing:

- `QuestionAdmin` and `ChoiceAdmin` are registered in `configure_admin()`.
- `/admin/` is mounted with `admin_routes_with_di`.
- `/static/admin/` is mounted with `admin_static_routes()`.
- `dist-wasm/` is registered through `AppStaticFilesConfig`.
- `cargo make dev` builds WASM before starting the pages server.
