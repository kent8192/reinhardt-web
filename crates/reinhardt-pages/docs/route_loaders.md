# Route-level data loaders

Route loaders make entry data an explicit part of a routed page. A loader is
prepared after URL matching and before the destination route is committed, so
the old route remains mounted while the next route is loading.

## Declare and bind a loader

`#[loader]` keeps the original async function callable for ordinary Rust unit
tests and generates a stable marker for routing:

```rust,ignore
use reinhardt_pages::{Loader, Path, Page, component, loader, page};

#[loader]
async fn project_loader(Path(project_id): Path<i64>) -> Result<Project, String> {
    fetch_project(project_id).await
}

#[component(
    "/projects/{project_id}/",
    name = "project-detail",
    loader = project_loader
)]
fn project_detail(Loader(project): Loader<Project>) -> Page {
    page!(|project: Project| { h1 { { project.name } } })(project)
}
```

The `Loader<T>` type must match the loader's `Result<T, E>` data type. A loader
may also accept `Query<T>` and one `CancellationToken` extractor. Path and
query inputs are decoded before they are included in the shared query-cache
key. Layouts use the same `loader = ...` option and receive their value before
the leaf route is rendered.

## Navigation and cancellation

`ClientLauncher` installs a pages-owned `NavigationCoordinator`. It performs
matching and guards synchronously, then prepares all matched layout and leaf
loaders concurrently. Only the latest navigation generation can commit. A
superseded attempt drops its loader futures and releases its query leases;
Reinhardt-managed browser requests also receive an abort signal. Work started
by application code outside the loader future is intentionally not cancelled.

`use_transition().is_pending` includes coordinator navigation pending state in
addition to local transition work. A failed loader leaves the current route and
reactive scopes mounted. Configure a sibling fallback on `RouterOutlet` when a
route-specific error view is needed:

```rust,ignore
RouterOutlet::new(router).navigation_error_fallback(|error| {
    page!(|message: String| { p { { message } } })(error.public_message().to_owned())
})
```

Loader errors expose only their safe public message and optional status. The
internal diagnostic cause never enters the browser state payload. Failed pop
navigations restore the committed history entry after preparation fails.
On first launch, legacy or host-created history state without a framework entry
index is upgraded in place to index `0`; an existing framework index is retained
across reloads so back/forward restoration remains monotonic.

## Prefetch and query sharing

Links can prepare the same cache entry before a click:

```rust,ignore
use reinhardt_pages::{Link, PrefetchMode};

Link::new("/projects/42/", "Project 42")
    .prefetch(PrefetchMode::Hover);
```

`Hover` covers pointer intent and keyboard focus. `Viewport` uses an
`IntersectionObserver`. Prefetch does not change history, route signals, or
navigation pending state, and a settled prefetch error is silent. Navigation,
prefetch, mounted routes, and `use_query` all use the single keyed query cache;
an in-flight request is shared while at least one RAII lease remains.

## SSR and hydration

`SsrRenderer::render_route_to_string` matches the route, prepares every
matched loader concurrently, installs a request-local `LoaderStore`, and then
renders the loaded layout/leaf tree:

```rust,ignore
let mut renderer = SsrRenderer::new();
let output = renderer
    .render_route_to_string(&router, "/projects/42/")
    .await;
assert_eq!(output.status, 200);
```

Successful values are emitted through the existing HTML-safe `SsrState`
serialization path under both a stable `route-loader:<id>` entry and the
opaque query-cache key. Hydration reconstructs the initial typed loader store
from the stable entry and restores the query success state by key, so the
first client render does not issue a duplicate request. Cancellations, partial
results, and internal error causes are never serialized.

## Testing loaders directly

The original function remains an ordinary async function, so direct tests do
not need a router or browser runtime:

```rust,ignore
let project = project_loader(Path(42)).await?;
```

Use a matched `ClientRouteTreeMatch` and `loader_cache_id` when testing input
canonicalization or cache deduplication. Native SSR tests can assert both the
rendered output and `SsrState::get_route_loader_state`.
