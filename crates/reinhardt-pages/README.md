# reinhardt-pages

WASM-based reactive frontend framework for Reinhardt with Django-like API.

## Features

- **Fine-grained Reactivity**: Leptos/Solid.js-style Signal system with automatic dependency tracking
- **Hybrid Rendering**: SSR + Client-side Hydration for optimal performance and SEO
- **Django-like API**: Familiar patterns for Reinhardt developers
- **Low-level Only**: Built on wasm-bindgen, web-sys, and js-sys (no high-level framework dependencies)
- **Security First**: Built-in CSRF protection, XSS prevention, and session management
- **Simplified Conditional Compilation**: `cfg_aliases` integration and automatic event handler handling
- **Action State Helpers**: `use_action_state` and `Action::dispatching*` reduce async mutation boilerplate

For a React concept mapping, see
[Reinhardt Pages for React developers](docs/react_to_reinhardt.md).

## Quick Start

### Using the Prelude (Recommended)

The prelude provides all commonly used types with a single import:

```rust
// Instead of multiple scattered imports:
// use reinhardt_pages::{Signal, View, use_state, ...};
// use reinhardt_pages::component::{ElementView, IntoView};
// use reinhardt_pages::reactive::{Effect, Memo};

// Use the unified prelude:
use reinhardt_pages::prelude::*;
// or via reinhardt crate:
use reinhardt::pages::prelude::*;
```

### Typed standard events and raw custom events

Standard intrinsic events select an exact payload from the authoritative event
catalog. The payload is inferred in `page!`, or can be named explicitly for an
external function or `Callback`:

```rust,ignore
use reinhardt_pages::event::{ClickEvent, InputEvent};
use reinhardt_pages::prelude::*;

fn inspect_click(event: ClickEvent) {
    let _origin = event.target();
    let _listener = event.current_target();
}

page!({
    button { @click: inspect_click, "Inspect" }
    input { @input: |event: InputEvent| {
        match event.value() {
            Ok(value) => info_log!("value={value}"),
            Err(error) => warn_log!("input extraction failed: {error}"),
        }
    } }
})
```

Use `@custom("name")` and `platform::Event` for an arbitrary raw DOM event.
Custom typed `detail` values are intentionally deferred to #5636. Component
`@event` props remain typed by the component's declared prop type; the DOM
event catalog applies only to intrinsic elements.

### Simplified cfg Attributes with cfg_aliases

Configure `cfg_aliases` in your project's `build.rs`:

```rust
// build.rs
use cfg_aliases::cfg_aliases;

fn main() {
    // Rust 2024 edition requires explicit check-cfg declarations
    println!("cargo::rustc-check-cfg=cfg(wasm)");
    println!("cargo::rustc-check-cfg=cfg(native)");

    cfg_aliases! {
        // Browser-WASM only (wasm32-unknown-unknown); excludes WASI / emscripten.
        wasm: { all(target_family = "wasm", target_os = "unknown") },
        native: { not(all(target_family = "wasm", target_os = "unknown")) },
    }
}
```

Add to `Cargo.toml`:

```toml
[build-dependencies]
cfg_aliases = "0.2"
```

Now you can use shorter cfg attributes:

```rust
// Before:
#[cfg(target_arch = "wasm32")]
// After:
#[cfg(wasm)]

// Before:
#[cfg(not(target_arch = "wasm32"))]
// After:
#[cfg(native)]
```

### Automatic Event Handler Handling

The `page!` macro automatically handles event handlers for server-side rendering. You no longer need to write duplicate conditional blocks:

```rust
use reinhardt_pages::prelude::*;

// This works on both WASM and native targets!
// On WASM: Event handlers are bound to DOM events
// On native: Event handlers are stored and can be dispatched by component tests
fn my_button(on_click: Signal<bool>) -> View {
    page!({
        button {
            @click: move |_| { on_click.set(true); },
            "Click me"
        }
    })
}
```

**Before** (manual conditional compilation):
```rust
#[cfg(target_arch = "wasm32")]
{
    page!({
        button {
            @click: move |_| { on_click.set(true); },
            "Click me"
        }
    })
}
#[cfg(not(target_arch = "wasm32"))]
{
    let _ = on_click; // suppress warning
    page!({
        button { "Click me" }
    })
}
```

**After** (automatic handling):
```rust
// Just write once - the macro handles everything!
page!({
    button {
        @click: move |_| { on_click.set(true); },
        "Click me"
    }
})
```

### `page!` Body Forms

Use `page!({ ... })` for app screens and ordinary functions that return a
`Page`. Free value identifiers from the surrounding Rust scope are treated as
implicit captures and cloned into generated reactive/event closures. Captured
values must implement `Clone`; `Signal<T>`, `Callback`, `Page`, `String`, and
most application handles are intended to be cheap to clone.

Use `page!(|| { ... })` or `page!(|props: Props| { ... })` when you want a
reusable factory that is called later. Closure-form pages keep strict capture
discipline: every value used in the body must be listed as a closure parameter.
Existing body-only pages that relied on surrounding values should migrate to
`page!({ ... })`. Use `page!(|| { ... })` for no-argument factories that must
remain callable, and use `page!(|value: Value| { ... })` when a factory needs
caller-supplied state.

### Reactive I18n

Enable the `i18n` feature to use `reinhardt-i18n` catalogs directly from
`page!`. `t!` returns lazily translated page text, so SSR renders the current
locale and later locale switches update reactive snapshots without explicitly
threading a resource through each component.

```rust,ignore
use reinhardt_i18n::{MessageCatalog, TranslationContext};
use reinhardt_pages::prelude::*;

let mut translations = TranslationContext::new("ja", "en-US");
let mut ja = MessageCatalog::new("ja");
ja.add_translation("dashboard.title", "ダッシュボード");
translations.add_catalog("ja", ja)?;

let i18n = I18nContext::new(translations);
let mut renderer = SsrRenderer::with_options(SsrOptions::new().i18n_context(i18n));

let html = renderer.render_page_with_view_head(page!(|| {
    h1 { { t!("dashboard.title") } }
})());
```

The SSR renderer serializes resolved catalogs into the hydration state under
`pages.i18n`, so client hydration can restore translations without refetching
the catalog.

### Forms: Static Definition and Dynamic Behavior

`form!` defines static form structure: field names, widgets, labels,
validation metadata, server function binding, and rendering. `use_form` owns
typed runtime behavior: values, field signals, dirty/touched state, validation
errors, loading, success, reset, and submit orchestration.

Create the form with `form!`, then attach runtime behavior to that generated
form:

```rust
use reinhardt_pages::{form, use_form};

let login_form = form! {
    name: LoginForm,
    action: "/login",
    fields: {
        username: CharField { initial: String::new() }
        password: CharField { initial: String::new() }
    }
};

let runtime = use_form(&login_form).build();
runtime.set_value(login_form.username_field(), "ada".to_string());
```

DTO request types can opt in to generated client-form companions with
`ClientForm`. This keeps request field names, enum choices, and typed request
assembly tied to the DTO while still using the same `use_form` runtime. Add
`#[client_form(validate)]` when the DTO implements `Validate` and should feed
those errors into the generated form runtime:

```rust,ignore
use reinhardt_pages::{ClientForm, ClientFormChoices, use_form};

#[derive(Clone, Default, PartialEq, ClientFormChoices)]
#[serde(rename_all = "snake_case")]
enum ProviderMode {
    #[default]
    Fake,
    LiveApi,
}

#[reinhardt::dto]
#[derive(Clone, serde::Serialize, serde::Deserialize, ClientForm)]
#[client_form(server_fn = crate::server::submit_project, validate)]
struct ProjectRequest {
    name: String,
    title: Option<String>,
    provider_mode: ProviderMode,
}

let form = ProjectRequestClientForm::new();
let runtime = use_form(&form).build();
runtime.set_value(ProjectRequestClientFormField::Title, "  ".to_string());
let request = ProjectRequestClientForm::to_request(&runtime);
assert_eq!(request.title, None);
let outcome = form.submit(&runtime).await?;
```

`ClientFormChoices` mirrors serde's externally tagged string names for unit
variants, including matching `rename_all` and variant `rename`; tagged,
untagged, or directionally renamed enum representations are rejected because
form choices submit bare strings. DTO fields marked with serde skip attributes
are kept out of editable form fields and preserved through generated request
values. Exported DTOs cannot use private editable fields; mark the field public
or make it an explicit hidden field with `#[client_form(skip)]` or a serde skip
attribute. Forms with generated `server_fn` submit helpers reject serde-skipped
request fields because the browser payload must match native request
deserialization exactly.

Use `use_form_action` when a validated form should dispatch a typed async
mutation:

```rust,ignore
use reinhardt_pages::{form, use_form, use_form_action};

let runtime = use_form(&login_form).build();
let save = use_form_action(&runtime, |values: LoginFormValues| async move {
    submit_login(values).await
})
.on_success(|runtime, _result| {
    runtime.reset_default_values();
});

if !save.is_pending() {
    save.submit();
}
```

`FileField` and `ImageField` also participate in the generated runtime
contract as `Option<web_sys::File>` values. File values are browser-owned and
are tracked for dirty/touched state without treating the file payload as a
serializable scalar.

Stable native widget coverage includes the following `form!` DSL items:

| DSL item | HTML output | Value state |
|---|---|---|
| `MonthInput` | `<input type="month">` | string field |
| `WeekInput` | `<input type="week">` | string field |
| `ResetButton` | `<button type="reset">` | none |
| `Button` | `<button type="button">` | none |
| `ImageInput` | `<input type="image">` | none |
| `Datalist` | `<datalist>` | option source only |
| `OptGroup` | `<optgroup>` | choice grouping only |
| `Output` | `<output>` | none |
| `Meter` | `<meter>` | none |
| `Progress` | `<progress>` | none |

Typed native attributes are accepted for the controls that support them:

| Attribute | Compatible controls |
|---|---|
| `min` / `max` / `step` | number, range, date, time, datetime-local, month, week |
| `size` | text-like inputs |
| `accept` / `capture` | file-like inputs |
| `multiple` | file-like inputs and multi-select |
| `list` | datalist-compatible text-like inputs |

`FieldGroup` renders as semantic `<fieldset>` output. When `label` is
present, the label is rendered as a `<legend>` inside the fieldset.

`CustomWidget` is experimental and must opt in explicitly:

```rust,ignore
date_range: CharField {
    widget: CustomWidget(crate::widgets::DateRangePicker) {
        experimental,
        adapter: crate::widgets::DateRangeAdapter,
    },
}
```

The adapter API may change in a minor release with a documented migration path.

Arguments supplied from ambient context use `ambient_arguments`. The old
`strip_arguments` name remains as a deprecated alias. CSRF should stay at the
transport layer: `#[server_fn]` client stubs attach `X-CSRFToken`, while
non-WASM forms still render the hidden CSRF input for traditional posts.

### Reactive Conditional Rendering

`page!` wraps expression, `if`, and `for` nodes in reactive render scopes. When
those nodes read `Signal` values, they re-evaluate as the signals change.

#### Why Signal Reads Belong Inside `page!`

When you extract Signal values before the `page!` macro, they become static:

```rust
// Problem: Static values don't update when Signal changes
let has_error = error.get().is_some(); // Static bool captured at render time
page!({
    if has_error {
        div { "Error occurred" }
    }
})
```

Read the signal inside the page body instead:

```rust
page!({
    if error.get().is_some() {
        div { { error.get().unwrap_or_default() } }
    }
})
```

#### Signal-first Pattern

For reactive UIs, pass Signals directly to the `page!` macro instead of extracting values:

```rust
use reinhardt_pages::prelude::*;

fn error_display() -> View {
    let (error, set_error) = use_state(None::<String>);

    // Read the Signal inside page! (not before it)
    let error_signal = error.clone();

    page!({
        if error_signal.get().is_some() {
            div {
                class: "alert-danger",
                { error_signal.get().unwrap_or_default() }
            }
        }
    })
}
```

#### Reactive vs Static Values

| Syntax | Use Case | Behavior |
|--------|----------|----------|
| `if signal.get().is_some() { ... }` | Signal-dependent branches | Re-evaluates when the signal changes |
| `for item in items.get() { ... }` | Signal-dependent lists | Rebuilds the list when the signal changes |
| `if precomputed_bool { ... }` | Static values | Uses the value captured when the page is built |

#### Best Practices

1. **Pass Signals directly**: Use `Signal<T>` parameters instead of extracting values
2. **Clone Signals**: `Signal::clone()` is cheap (Rc-based), so clone freely
3. **Clone captured handles**: direct `page!({ ... })` clones captured values into generated closures
4. **Use closure form for factories**: keep `page!(|props: Props| { ... })` when the page must be called later

## Typed Server Function Sets

`#[server_fnset]` groups existing `#[server_fn]` markers into a named, typed
registration chain. Members retain their individual codecs, CSRF behavior,
extractors, injected parameters, metadata, and mock identity. Registration stays
explicit:

```rust,ignore
#[server_fnset(name = "admin")]
pub fn admin_fns() -> impl ServerFnSetRegistration {
    ServerFnSet::new()
        .server_fn(load_dashboard::marker)
        .server_fn(export_data::marker)
}

let router = ServerRouter::new().server_fnset(admin_fns());
```

The opt-in `model-server-fnset` feature generates exactly six typed POST RPCs
for a resource: `list`, `retrieve`, `create`, `update`, `partial_update`, and
`destroy`. Resources use explicit wire DTO mappings, a typed unique lookup, and
a mandatory policy; unrestricted access requires choosing `AllowAllPolicy`.
Standard methods can be replaced with checked overrides, while additional
methods use `#[action(detail = ..., transactional = ...)]`. Action underscores
normalize to hyphens under `/api/server_fn/<set-name>/<action>`.

Offset pagination defaults to 25, accepts limits from 1 through 100, and returns
the policy-scoped total before slicing. Structured model errors map to stable
400, 401, 403, 404, 409, and 500 responses, with internal failures sanitized on
the wire. Each generated action has its own marker for component and MSW mocks;
`reinhardt-test` model-action mocks use `model-server-fnset-msw`.

WASM builds retain wire contracts, metadata, markers, and client stubs. ORM
resources, policies, action contexts, database executors, native handlers, and
the `ModelServerFnSet` constructor remain native-only. Model sets do not provide
action subsets, a read-only set type, REST/OpenAPI generation, cursor
pagination, bulk or nested actions, composite lookups, global discovery, or
automatic model-to-DTO derivation. See the
[Server Function Macro Guide](docs/server_fn_macro.md#typed-server-function-sets)
for the complete `ArticleResource` example and target boundary.

## Testing

### Native Component Tests

Use `reinhardt_pages::testing::component::render` for fast interaction tests
that do not need a browser:

```rust
use reinhardt_pages::testing::component::{Role, render};

#[tokio::test]
async fn refresh_loads_jobs() {
    let screen = render(jobs_page);
    screen.mock_server_fn::<load_jobs::marker>(|_args| Ok(vec!["Index job".to_string()]));

    screen.get_by_role(Role::Button, "Refresh").click();
    screen.settle().await;

    assert!(screen.query_by_text("Index job").is_some());
}
```

The mock API uses `MockableServerFn` markers and therefore requires the
`msw` feature. Use direct `server_fn` calls for business logic tests and
WASM/browser tests for hydration or browser API coverage. The native renderer
resolves reactive views, active suspense branches, and deferred content branches
before exposing queryable text and roles.

## Architecture

This framework consists of several key modules:

- **`reactive`**: Fine-grained reactivity system (Signal, Effect, Memo)
- **`dom`**: DOM abstraction layer
- **`builder`**: HTML element builder API
- **`component`**: Component system with IntoView trait
- **`form`**: Django Form integration (native only)
- **`csrf`**: CSRF protection
- **`auth`**: Authentication integration
- **`api`**: API client with Django QuerySet-like interface
- **`server_fn`**: Server Functions (RPC)
- **`ssr`**: Server-side rendering
- **`hydration`**: Client-side hydration
- **`router`**: Client-side routing (reinhardt-urls compatible)
- **`platform`**: Platform abstraction types
- **`prelude`**: Unified imports

## Prelude Contents

The prelude includes:

### Reactive System
- `Signal`, `Effect`, `Memo`, `Resource`, `ResourceState`
- `LatestResourceValue`, `LatestResourceState`, `use_latest_resource_value`
- Context: `Context`, `ContextGuard`, `create_context`, `get_context`, `provide_context`, `remove_context`
- Explicit batching: `reinhardt_pages::reactive::batch`

### Hooks
- `use_state`, `use_effect`, `use_memo`, `use_callback`, `use_context`
- `use_ref`, `use_reducer`, `use_transition`, `use_deferred_value`
- `use_id`, `use_layout_effect`, `use_debug_value`
- `use_optimistic`, `use_action`, `Action::with_optimistic`, `use_shared_state`, `use_sync_external_store`
- `use_resource` (async data fetching; `use_resource(fetcher, deps![...])` uses an explicit dependency list, while SSR registers the fetcher in the request context, awaits it up to `SsrOptions::resource_timeout`, and serializes resolved state for hydration)
- `use_query`, `use_mutation`, `QueryKey` (app-wide keyed async data cache over `#[server_fn]` reads, with manual refetch, polling, stale-time policy, and mutation invalidation)

`Resource::latest_after(&action)` and `use_latest_resource_value(resource)` compose loaded resource state with one or more `Action` success values. Later actions have higher priority, and `refetch_on_success()` can automatically refresh the resource after a mutation succeeds.

For cross-component reads, prefer `use_query` with the key helper generated by
`#[server_fn]`:

```rust,ignore
let jobs = use_query(list_project_jobs::key(project_id)).poll(Duration::from_secs(5));
let retry = use_mutation(retry_job).invalidates(list_project_jobs::key(project_id));
```

The cache canonicalizes JSON object arguments, hashes the canonical payload in
the generated key ID, and deduplicates mounted queries with the same key. Raw
server-function arguments are therefore not written into SSR hydration keys. It
keeps successful data available during refetch and uses the same SSR resource
serialization channel as `use_resource` for hydration seeding. `is_pending()`
reports the initial load, while `is_fetching()` also reports background
refreshes. Server-function keys that depend on request extractors or injected
parameters skip native SSR prefetching and are left for the browser fetch path
or native component-test mocks. Query handles can also be tracked by
`SuspenseBoundary::track(...)`.

### Component System
- `Component`, `ElementView`, `IntoView`, `View`, `Props`, `ViewEventHandler`
- `SuspenseBoundary`, `ErrorBoundary`, `BoundaryError`, `ErrorTracker`

### Events and Callbacks
- `EventPayload`, catalog-generated payloads such as `ClickEvent` and `InputEvent`
- `EventTarget`, `EventTargetError`, `EventFile`, `Modifiers`, `Point`
- `Callback`, `IntoTypedEventHandler`, `typed_event_handler`
- `raw_event_handler` and `platform::Event` for explicit raw custom events
- [Native component testing](docs/native_component_testing.md)

### DOM
- `Document`, `Element`, `EventHandle`, `EventType`, `document`

### Routing
- `Link`, `Router`, `Route`, `RouterOutlet`, `PathPattern`

### API and Server Functions
- `ApiModel`, `ApiQuerySet`, `Filter`, `FilterOp`
- `ServerFn`, `ServerFnError`
- See [Server Function Macro Guide](docs/server_fn_macro.md) for detailed usage and migration information
- Use `#[client_page]` for client page functions that must also compile as native route-table stubs
- See [WASM/server API Parity Macro](docs/wasm_server_api.md) for APIs that need matching public surfaces with target-specific implementations
- See [React-to-Reinhardt Guide](docs/react_to_reinhardt.md) for React hooks, JSX, actions, routing, SSR, and hydration mappings

### Authentication and Security
- `AuthData`, `AuthError`, `AuthState`, `auth_state`
- `CsrfManager`, `get_csrf_token`

### SSR and Hydration
- `HydrationContext`, `HydrationError`, `hydrate`
- `SsrOptions`, `SsrRenderer`, `SsrStream`, `SsrState`

SSR rendering APIs are async. Use `render_page(...).await` for streamed output
or `render_page_to_string(...).await` when a buffered string is needed:

```rust,no_run
use reinhardt_pages::component::{Component, Page};
use reinhardt_pages::ssr::{SsrOptions, SsrRenderer};
use std::time::Duration;

struct App;

impl Component for App {
    fn render(&self) -> Page {
        Page::text("Hello")
    }

    fn name() -> &'static str {
        "App"
    }
}

async fn render_app() {
    let app = App;
    let mut renderer = SsrRenderer::new();
    let stream = renderer.render_page(&app).await;

    let mut renderer = SsrRenderer::with_options(
        SsrOptions::new().resource_timeout(Duration::from_secs(1)),
    );
    let html = renderer.render_page_to_string(&app).await;
    let _ = (stream, html);
}
```

Resources created with `use_resource` during SSR are keyed deterministically,
resolved on the server, and embedded in the hydration payload. Use
`use_resource_with_key` when a resource hook is conditionally rendered and needs
a stable explicit hydration key. Implicit resource keys are allocated at the
document level so marker-rendered islands and their hydration replays preserve
the same key order. Suspense boundaries keep fallback and content roots
transparent; streaming metadata is emitted outside the branch DOM.

### I18n
- `I18nContext`, `I18nStateError`, `TranslatedText`, `tr`, `tn`, `tp`, `tnp`
- `provide_i18n_context`, `use_i18n_context`, `set_locale`, `locale`
- `with_i18n_context`

### Forms (native only)
- `FormBinding`, `FormComponent`
- `Widget`, `FieldMetadata`, `FormMetadata`

### Macros
- `page!`
- `head!`
- `form!`
- `t!` (with the `i18n` feature)
- `client_page`
- `wasm_server_api`

### Task spawning (cross-target)
- `spawn_task`, `defer_yield` (no-op on native)

### WASM-specific
- `spawn_local` (re-exported from wasm_bindgen_futures; **deprecated** — use `spawn_task`)

## Example

```rust
use reinhardt_pages::prelude::*;

fn counter() -> View {
    let (count, set_count) = use_state(0);

    page!({
        div {
            p { { format!("Count: {}", count.get()) } }
            button {
                @click: move |_| set_count.update(|current| current + 1),
                "Increment"
            }
        }
    })
}
```

## Feature Flags

| Feature | Description |
|---------|-------------|
| `msgpack` | MessagePack serialization support |
| `model-server-fnset` | Native model-backed typed CRUD server function sets plus cross-target wire/client generation |
| `msw` | Typed marker metadata for component and MSW server function mocks |
| `testing` | Cross-target testing support; combine with `msw` for native component mocks |
| `pages-full` | Browser-oriented bundle (`msgpack` + `web-sys-full`); enable `model-server-fnset`, `msw`, and `testing` separately |
| `static` | Static file serving |
| `urls` | URL routing integration |
| `debug-hooks` | Debug hooks for development |
| `uuid` | UUID type support |
| `chrono` | Chrono date/time type support |
| `ast` | AST processing support |
| `web-sys-full` | All required web-sys features for WASM applications |

## License

Licensed under the BSD 3-Clause License.
