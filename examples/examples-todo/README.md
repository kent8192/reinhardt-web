# Reinhardt Todo Example

Canonical Todo application for `reinhardt-pages`.

It demonstrates:

- `page!` views and list rendering
- reactive `Signal` state with hooks
- `#[server_fn]` mutations for create, complete, and delete operations
- SPA route filters for all, active, and completed Todos
- the `ClientLauncher` hydration/mount flow

## Run Locally

From the repository root:

```bash
cp examples/.cargo/config.local.toml examples/.cargo/config.toml
cd examples/examples-todo
cargo run
```

For a browser build, use the same WASM flow as the other `reinhardt-pages`
examples and mount `index.html` with the generated WASM package.

## Routes

| Route | Filter |
|-------|--------|
| `/` | All Todos |
| `/active/` | Incomplete Todos |
| `/completed/` | Completed Todos |
